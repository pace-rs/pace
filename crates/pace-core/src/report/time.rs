//! The time parts of a report: net time per role, project and day, the
//! back-flow, targets against actual time, budgets and who decided.

use std::{cmp::Reverse, collections::BTreeMap};

use jiff::{SignedDuration, ZonedDateTime, civil::Date, tz::TimeZone};
use serde::Serialize;

use crate::{
    Catalog, DeterminedBy, Entry, ProjectRef, ReportFilter, RoleId, State,
    report::{
        filter::canonical_project,
        split::{Stretch, per_day, worked},
    },
};

/// A sum of net time, and whether a running entry adds to it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct TimeShare {
    /// The net time.
    pub net: SignedDuration,
    /// Whether a running entry counts in it, up to now.
    pub running: bool,
}

impl TimeShare {
    fn add(&mut self, net: SignedDuration, running: bool) {
        self.net += net;
        self.running |= running;
    }
}

/// The net time of a role: the effective role of its entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoleTime {
    /// The role; `None` for the entries without one.
    pub role: Option<RoleId>,
    /// The net time.
    pub net: SignedDuration,
    /// Whether a running entry counts in it, up to now.
    pub running: bool,
}

/// The net time of a project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectTime {
    /// The project: the ID of its record, else the key its entries name.
    pub project: ProjectRef,
    /// The net time.
    pub net: SignedDuration,
    /// Whether a running entry counts in it, up to now.
    pub running: bool,
}

/// The net time of a day in the report's zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DayTime {
    /// The day.
    pub date: Date,
    /// The net time that lies on it.
    pub net: SignedDuration,
    /// Whether a running entry counts in it, up to now.
    pub running: bool,
}

/// A role's weekly target, scaled to the period, against its actual time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RoleTarget {
    /// The role.
    pub role: RoleId,
    /// A seventh of its weekly target for each day of the period.
    pub target: SignedDuration,
    /// Its net time in the period.
    pub actual: SignedDuration,
    /// Whether a running entry counts in `actual`, up to now.
    pub running: bool,
}

/// How much of a project's budget its entries used, over all time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BudgetUse {
    /// The project.
    pub project: ProjectRef,
    /// The time it may take.
    pub budget: SignedDuration,
    /// The net time of all its entries up to now.
    pub used: SignedDuration,
    /// Whether a running entry counts in `used`, up to now.
    pub running: bool,
    /// `used` in percent of `budget`, rounded; `None` for a budget of zero.
    pub percent: Option<i64>,
}

/// Who determined the time: the shares of the net time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct DecidedShares {
    /// Self-determined.
    pub by_me: TimeShare,
    /// Other-determined.
    pub by_others: TimeShare,
    /// Not marked.
    pub unmarked: TimeShare,
}

impl DecidedShares {
    fn add(&mut self, determined_by: Option<DeterminedBy>, net: SignedDuration, running: bool) {
        let share = match determined_by {
            Some(DeterminedBy::Me) => &mut self.by_me,
            Some(DeterminedBy::Others) => &mut self.by_others,
            None => &mut self.unmarked,
        };
        share.add(net, running);
    }
}

/// The sums of net time, entry by entry.
#[derive(Debug, Default)]
pub(crate) struct TimeTally {
    pub(crate) roles: BTreeMap<Option<RoleId>, TimeShare>,
    pub(crate) projects: BTreeMap<ProjectRef, TimeShare>,
    pub(crate) back_flow: TimeShare,
    pub(crate) days: BTreeMap<Date, TimeShare>,
    pub(crate) decided: DecidedShares,
}

impl TimeTally {
    /// A tally with every day of `period` at zero.
    pub(crate) fn over(period: Option<Stretch>, zone: &TimeZone) -> Self {
        let days = period
            .map_or_else(Vec::new, |period| per_day(period, zone))
            .into_iter()
            .map(|(date, _)| (date, TimeShare::default()))
            .collect();
        Self {
            days,
            ..Self::default()
        }
    }

    /// Add the net time `entry` has inside `period`.
    pub(crate) fn add(
        &mut self,
        entry: &Entry,
        catalog: &Catalog,
        period: Stretch,
        now: &ZonedDateTime,
        zone: &TimeZone,
    ) {
        let parts: Vec<(Date, SignedDuration)> = worked(entry, now)
            .into_iter()
            .filter_map(|stretch| stretch.within(period))
            .flat_map(|stretch| per_day(stretch, zone))
            .collect();
        let net: SignedDuration = parts.iter().map(|(_, net)| *net).sum();
        let running = entry.is_open();
        for (date, part) in parts {
            self.days.entry(date).or_default().add(part, running);
        }
        self.roles
            .entry(entry.effective_role(catalog))
            .or_default()
            .add(net, running);
        match &entry.project {
            Some(project) => self
                .projects
                .entry(canonical_project(project, catalog))
                .or_default()
                .add(net, running),
            None => self.back_flow.add(net, running),
        }
        self.decided.add(entry.determined_by, net, running);
    }

    /// The net time of all entries.
    pub(crate) fn tracked(&self) -> SignedDuration {
        self.days.values().map(|share| share.net).sum()
    }

    /// The net time per role, the most first; of two with one time, by key,
    /// and the entries without a role last.
    pub(crate) fn role_times(&self, catalog: &Catalog) -> Vec<RoleTime> {
        let mut roles: Vec<RoleTime> = self
            .roles
            .iter()
            .map(|(role, share)| RoleTime {
                role: *role,
                net: share.net,
                running: share.running,
            })
            .collect();
        roles.sort_by_key(|time| {
            let key = time
                .role
                .and_then(|role| catalog.role_by_id(role))
                .map(|role| role.key.as_str().to_owned());
            (Reverse(time.net), key.is_none(), key)
        });
        roles
    }

    /// The net time per project, the most first; of two with one time, by
    /// key.
    pub(crate) fn project_times(&self, catalog: &Catalog) -> Vec<ProjectTime> {
        let mut projects: Vec<ProjectTime> = self
            .projects
            .iter()
            .map(|(project, share)| ProjectTime {
                project: project.clone(),
                net: share.net,
                running: share.running,
            })
            .collect();
        projects.sort_by_key(|time| (Reverse(time.net), project_key(&time.project, catalog)));
        projects
    }

    /// The net time per day of the period, the earliest first.
    pub(crate) fn day_times(&self) -> Vec<DayTime> {
        self.days
            .iter()
            .map(|(date, share)| DayTime {
                date: *date,
                net: share.net,
                running: share.running,
            })
            .collect()
    }

    /// The targets of the roles in use that have one and pass the role
    /// filter, by key, against their time.
    pub(crate) fn targets(
        &self,
        catalog: &Catalog,
        filter: &ReportFilter,
        days: u32,
    ) -> Vec<RoleTarget> {
        catalog
            .roles()
            .filter(|role| role.archived.is_none())
            .filter(|role| filter.roles.is_empty() || filter.roles.contains(&role.id))
            .filter_map(|role| {
                let weekly = role.weekly_target?;
                let actual = self.roles.get(&Some(role.id)).copied().unwrap_or_default();
                Some(RoleTarget {
                    role: role.id,
                    target: scaled_to_days(weekly, days),
                    actual: actual.net,
                    running: actual.running,
                })
            })
            .collect()
    }

    /// The budgets of the projects in the report that have one, the most
    /// used first; their time counts every entry of `all_time`.
    pub(crate) fn budgets(
        &self,
        catalog: &Catalog,
        all_time: &State,
        now: &ZonedDateTime,
    ) -> Vec<BudgetUse> {
        let mut budgets: Vec<BudgetUse> = self
            .project_times(catalog)
            .into_iter()
            .filter_map(|time| {
                let budget = catalog.resolve(&time.project)?.budget?;
                let used = used_by(&time.project, catalog, all_time, now);
                Some(BudgetUse {
                    percent: percent_of(used.net, budget),
                    project: time.project,
                    budget,
                    used: used.net,
                    running: used.running,
                })
            })
            .collect();
        budgets.sort_by_key(|budget| Reverse(budget.percent));
        budgets
    }
}

/// The key a report sorts a project by: its record's, else its own.
pub(crate) fn project_key(project: &ProjectRef, catalog: &Catalog) -> String {
    match (catalog.resolve(project), project) {
        (Some(record), _) => record.key.to_string(),
        (None, ProjectRef::Key(key)) => key.to_string(),
        (None, ProjectRef::Id(id)) => id.to_string(),
    }
}

/// A seventh of `weekly` for each of `days`; the largest duration if that
/// leaves jiff's range.
fn scaled_to_days(weekly: SignedDuration, days: u32) -> SignedDuration {
    i32::try_from(days)
        .ok()
        .and_then(|days| weekly.checked_mul(days))
        .map_or(SignedDuration::MAX, |total| total / 7)
}

/// The net time of every entry of `project` in `state`, up to `now`, and
/// whether one of them runs.
fn used_by(
    project: &ProjectRef,
    catalog: &Catalog,
    state: &State,
    now: &ZonedDateTime,
) -> TimeShare {
    let mut used = TimeShare::default();
    for entry in state.entries().filter(|entry| {
        entry
            .project
            .as_ref()
            .is_some_and(|own| canonical_project(own, catalog) == *project)
    }) {
        let net = worked(entry, now).into_iter().map(Stretch::length).sum();
        used.add(net, entry.is_open());
    }
    used
}

/// `part` in percent of `whole`, rounded to the nearest whole percent;
/// `None` when `whole` isn't positive.
fn percent_of(part: SignedDuration, whole: SignedDuration) -> Option<i64> {
    let whole = whole.as_nanos();
    (whole > 0).then(|| {
        let percent = (part.as_nanos() * 200 + whole) / (whole * 2);
        i64::try_from(percent).unwrap_or(i64::MAX)
    })
}
