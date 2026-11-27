//! The report of `reflect`, computed from values: time per role, project and
//! day, targets, budgets, revenue, breaks, who decided and the overlaps of a
//! period.
//!
//! The app reads the events of the period, builds their [`State`] and
//! hands it to [`Report::build`] with the [`Catalog`], the period, the
//! [`ReportFilter`], now and the report's zone. When
//! [`Report::needs_all_events`] says so, it also reads every event, for
//! the budgets. The core reads nothing.

mod breaks;
mod filter;
mod revenue;
mod split;
mod time;

pub use breaks::{HoldReport, HoldShare, PauseShare, ReasonCount, ReportCatch, TOP_REASONS};
pub use filter::ReportFilter;
pub(crate) use filter::canonical_project;
use jiff::{SignedDuration, ZonedDateTime, tz::TimeZone};
use pace_error::PaceResult;
use pace_time::TimeRange;
pub use revenue::{ClientRevenue, ProjectRevenue, Revenue};
use serde::Serialize;
pub use time::{BudgetUse, DayTime, DecidedShares, ProjectTime, RoleTarget, RoleTime, TimeShare};

use crate::{
    Catalog, Entry, EntryId, State, billable_charges,
    report::{breaks::BreakTally, split::Stretch, time::TimeTally},
};

/// The hours of a week; role targets above it can't all be met.
const WEEK: SignedDuration = SignedDuration::from_hours(7 * 24);

/// What [`Report::build`] takes.
#[derive(Debug, Clone, Copy)]
pub struct ReportInput<'input> {
    /// The state of the period's events; [`State::resolve_projects`] has
    /// moved its entries to their project records.
    pub state: &'input State,
    /// The roles, clients and project records.
    pub catalog: &'input Catalog,
    /// The period; open at the start, it begins with the first entry, and
    /// open at the end, it ends now.
    pub period: &'input TimeRange,
    /// Which entries count.
    pub filter: &'input ReportFilter,
    /// Now: running entries count up to it.
    pub now: &'input ZonedDateTime,
    /// The report's zone, whose midnights split the days.
    pub zone: &'input TimeZone,
    /// The state of every event, for the budgets; without it, a budget
    /// counts only the entries of `state`.
    pub all_time: Option<&'input State>,
}

/// The period a report covers, in the report's zone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReportPeriod {
    /// Its first moment.
    pub start: ZonedDateTime,
    /// The first moment after it.
    pub end: ZonedDateTime,
    /// The days it touches, each from midnight to midnight.
    pub days: u32,
}

impl ReportPeriod {
    /// The effective period of `range`: an open start takes the first of
    /// `entries`, else now; an open end takes now.
    fn of(range: &TimeRange, entries: &[&Entry], now: &ZonedDateTime, zone: &TimeZone) -> Self {
        let start = range
            .start()
            .or_else(|| entries.iter().map(|entry| &entry.start).min())
            .unwrap_or(now)
            .with_time_zone(zone.clone());
        let end = range.end().unwrap_or(now).with_time_zone(zone.clone());
        let end = end.max(start.clone());
        let stretch = Stretch::new(start.timestamp(), end.timestamp());
        let days = stretch.map_or(0, |stretch| split::per_day(stretch, zone).len());
        Self {
            start,
            end,
            days: u32::try_from(days).unwrap_or(u32::MAX),
        }
    }

    fn stretch(&self) -> Option<Stretch> {
        Stretch::new(self.start.timestamp(), self.end.timestamp())
    }
}

/// Something the report warns about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ReportWarning {
    /// The weekly targets of the roles in use add up to more than the
    /// 168 hours of a week.
    TargetsAboveWeek {
        /// Their sum.
        total: SignedDuration,
    },
    /// Two counted entries share time in the period.
    Overlap {
        /// The entry that starts first.
        first: EntryId,
        /// The other entry.
        second: EntryId,
        /// When the shared time begins, inside the period.
        start: ZonedDateTime,
        /// When it ends, inside the period.
        end: ZonedDateTime,
    },
}

/// The report of a period: plain data that a table, a template or JSON
/// shows. Durations are net time: without holds and pauses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    /// The period.
    pub period: ReportPeriod,
    /// The net time of the counted entries.
    pub tracked: SignedDuration,
    /// The time that passed in the period, up to now, minus `tracked`;
    /// never negative.
    pub untracked: SignedDuration,
    /// The time per effective role, the most first.
    pub roles: Vec<RoleTime>,
    /// The time per project, the most first.
    pub projects: Vec<ProjectTime>,
    /// The time of the entries without a project: the back-flow, the time
    /// no project claims.
    pub back_flow: TimeShare,
    /// The time per day of the period, each day, the earliest first.
    pub days: Vec<DayTime>,
    /// The weekly targets of the roles, scaled to the period.
    pub targets: Vec<RoleTarget>,
    /// The budgets of the projects in the report.
    pub budgets: Vec<BudgetUse>,
    /// What the billable entries that begin in the period earn.
    pub revenue: Revenue,
    /// The interruptions.
    pub holds: HoldReport,
    /// The pauses by kind, of the kinds that occur.
    pub pauses: Vec<PauseShare>,
    /// The catches, by time.
    pub catches: Vec<ReportCatch>,
    /// Who determined the time.
    pub decided: DecidedShares,
    /// The running entries that count up to now.
    pub running: Vec<EntryId>,
    /// What the report warns about.
    pub warnings: Vec<ReportWarning>,
}

impl Report {
    /// The report of the entries of `input.state` that pass the filter,
    /// over the period.
    ///
    /// Each entry counts the part of its net time inside the period, split
    /// exactly at midnight in the report's zone; a running entry counts up
    /// to now. The revenue counts each billable entry that begins in the
    /// period whole, on its start day, as `bill` does.
    ///
    /// # Errors
    ///
    /// If an amount of the revenue is too big to count in minor units.
    pub fn build(input: &ReportInput<'_>) -> PaceResult<Self> {
        let ReportInput {
            state,
            catalog,
            period,
            filter,
            now,
            zone,
            all_time,
        } = *input;
        let entries: Vec<&Entry> = state
            .entries()
            .filter(|entry| filter.matches(entry, catalog))
            .collect();
        let period = ReportPeriod::of(period, &entries, now, zone);
        let span = period.stretch();
        let mut time = TimeTally::over(span, zone);
        let mut breaks = BreakTally::default();
        let mut running = Vec::new();
        let mut begun = Vec::new();
        if let Some(span) = span {
            begun.extend(
                entries
                    .iter()
                    .copied()
                    .filter(|entry| span.contains(entry.start.timestamp())),
            );
            for entry in counted(&entries, span, now) {
                time.add(entry, catalog, span, now, zone);
                breaks.add(entry, span, now, zone);
                if entry.is_open() {
                    running.push(entry.id);
                }
            }
        }

        let tracked = time.tracked();
        let (holds, pauses, catches) = breaks.finish();
        let mut warnings: Vec<ReportWarning> = targets_above_week(catalog).into_iter().collect();
        warnings.extend(overlaps(state, input, span));
        let revenue = Revenue::of(billable_charges(&begun, catalog, zone, now)?, catalog)?;
        Ok(Self {
            untracked: untracked(&period, now, tracked),
            tracked,
            roles: time.role_times(catalog),
            projects: time.project_times(catalog),
            back_flow: time.back_flow,
            days: time.day_times(),
            targets: time.targets(catalog, filter, period.days),
            budgets: time.budgets(catalog, all_time.unwrap_or(state), now),
            revenue,
            holds,
            pauses,
            catches,
            decided: time.decided,
            running,
            warnings,
            period,
        })
    }

    /// Whether a time of the report counts a running entry up to now: an
    /// entry the period counts runs, or an entry of a project with a
    /// budget runs, as a budget counts every entry, also one outside the
    /// period or the filter.
    #[must_use]
    pub fn marks_running(&self) -> bool {
        !self.running.is_empty() || self.budgets.iter().any(|budget| budget.running)
    }

    /// Whether the report of `state` needs every event, not only the
    /// period's: an entry that passes `filter` has a project with a budget.
    #[must_use]
    pub fn needs_all_events(state: &State, catalog: &Catalog, filter: &ReportFilter) -> bool {
        state.entries().any(|entry| {
            filter.matches(entry, catalog)
                && entry
                    .project
                    .as_ref()
                    .and_then(|project| catalog.resolve(project))
                    .is_some_and(|record| record.budget.is_some())
        })
    }
}

/// The entries whose time touches `span`; an open entry lasts until `now`.
fn counted<'state>(
    entries: &[&'state Entry],
    span: Stretch,
    now: &ZonedDateTime,
) -> Vec<&'state Entry> {
    entries
        .iter()
        .copied()
        .filter(|entry| {
            Stretch::new(entry.start.timestamp(), entry.until(now).timestamp())
                .and_then(|stretch| stretch.within(span))
                .is_some()
        })
        .collect()
}

/// The time that passed in `period` up to `now`, minus `tracked`.
fn untracked(
    period: &ReportPeriod,
    now: &ZonedDateTime,
    tracked: SignedDuration,
) -> SignedDuration {
    let passed = Stretch::new(
        period.start.timestamp(),
        period.end.timestamp().min(now.timestamp()),
    )
    .map_or(SignedDuration::ZERO, Stretch::length);
    (passed - tracked).max(SignedDuration::ZERO)
}

/// The warning that the weekly targets of the roles in use add up to more
/// than a week.
fn targets_above_week(catalog: &Catalog) -> Option<ReportWarning> {
    let total: SignedDuration = catalog
        .roles()
        .filter(|role| role.archived.is_none())
        .filter_map(|role| role.weekly_target)
        .sum();
    (total > WEEK).then_some(ReportWarning::TargetsAboveWeek { total })
}

/// The overlaps of two entries that pass the filter, cut to `span`.
fn overlaps(state: &State, input: &ReportInput<'_>, span: Option<Stretch>) -> Vec<ReportWarning> {
    let Some(span) = span else {
        return Vec::new();
    };
    state
        .overlaps(input.now)
        .into_iter()
        .filter(|overlap| {
            input.filter.matches(overlap.first, input.catalog)
                && input.filter.matches(overlap.second, input.catalog)
        })
        .filter_map(|overlap| {
            let shared =
                Stretch::new(overlap.start.timestamp(), overlap.end.timestamp())?.within(span)?;
            Some(ReportWarning::Overlap {
                first: overlap.first.id,
                second: overlap.second.id,
                start: shared.start.to_zoned_datetime(input.zone.clone()),
                end: shared.end.to_zoned_datetime(input.zone.clone()),
            })
        })
        .collect()
}
