//! The comparison of `reflect --compare`: each number of a report beside
//! its value in the previous period, with the change, and the highlights
//! of both periods.

use jiff::SignedDuration;
use serde::Serialize;

use crate::{
    DayTime, Money, ProjectTime, Report, ReportPeriod, TimeShare,
    report::breaks::{HoldReport, HoldShare, PauseShare},
};

/// A number of the report beside its value in the previous period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Compared<T> {
    /// The value in the previous period.
    pub previous: T,
    /// The value now minus the previous value.
    pub difference: T,
    /// `difference` in percent of `previous`, rounded half away from
    /// zero; `None` when the previous value is zero.
    pub percent: Option<i64>,
}

impl Compared<SignedDuration> {
    /// The change from `previous` to `current`.
    #[must_use]
    pub fn of_time(current: SignedDuration, previous: SignedDuration) -> Self {
        Self {
            previous,
            difference: current.saturating_sub(previous),
            percent: change_percent(
                current.as_nanos().saturating_sub(previous.as_nanos()),
                previous.as_nanos(),
            ),
        }
    }
}

impl Compared<i64> {
    /// The change from `previous` to `current`, two counts.
    #[must_use]
    pub fn of_count(current: usize, previous: usize) -> Self {
        let current = i64::try_from(current).unwrap_or(i64::MAX);
        let previous = i64::try_from(previous).unwrap_or(i64::MAX);
        Self {
            previous,
            difference: current.saturating_sub(previous),
            percent: change_percent(
                i128::from(current) - i128::from(previous),
                i128::from(previous),
            ),
        }
    }
}

impl Compared<Money> {
    /// The change from `previous` to `current`; `None` when they are in
    /// two currencies, which never compare. Without `previous`, the
    /// previous value is zero of `current`'s currency.
    #[must_use]
    pub fn of_money(current: &Money, previous: Option<&Money>) -> Option<Self> {
        let currency = current.currency();
        let previous = previous.map_or_else(|| Money::zero(currency), |money| *money);
        (previous.currency() == currency).then(|| Self {
            previous,
            difference: Money::from_minor(
                current.minor().saturating_sub(previous.minor()),
                currency,
            ),
            percent: change_percent(
                i128::from(current.minor()) - i128::from(previous.minor()),
                i128::from(previous.minor()),
            ),
        })
    }
}

/// `difference` in percent of `previous`, rounded half away from zero;
/// `None` when `previous` isn't positive.
fn change_percent(difference: i128, previous: i128) -> Option<i64> {
    (previous > 0).then(|| {
        let doubled = difference.saturating_mul(200);
        let rounded = (doubled + doubled.signum() * previous) / (previous * 2);
        i64::try_from(rounded).unwrap_or(if rounded < 0 { i64::MIN } else { i64::MAX })
    })
}

/// A count and a time beside their previous values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ComparedBreaks {
    /// How many.
    pub count: Compared<i64>,
    /// Their time.
    pub time: Compared<SignedDuration>,
}

impl ComparedBreaks {
    fn of(
        (count, time): (usize, SignedDuration),
        previous: Option<(usize, SignedDuration)>,
    ) -> Self {
        let (previous_count, previous_time) = previous.unwrap_or((0, SignedDuration::ZERO));
        Self {
            count: Compared::of_count(count, previous_count),
            time: Compared::of_time(time, previous_time),
        }
    }
}

/// The revenue beside the previous period's, each in the order of the
/// report's [`crate::Revenue`]; a client or project whose amounts are in
/// two currencies has `None`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ComparedRevenue {
    /// The amount of each project.
    pub projects: Vec<Option<Compared<Money>>>,
    /// The amount of each client.
    pub clients: Vec<Option<Compared<Money>>>,
    /// The total of each currency.
    pub totals: Vec<Compared<Money>>,
}

/// The holds beside the previous period's, in the order of the report's
/// [`HoldReport`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComparedHolds {
    /// Every hold.
    pub total: ComparedBreaks,
    /// The holds of each source.
    pub by_source: Vec<ComparedBreaks>,
    /// How often each of the most frequent reasons was given.
    pub reasons: Vec<Compared<i64>>,
}

/// Who decided, beside the previous period's shares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ComparedDecided {
    /// Self-determined.
    pub by_me: Compared<SignedDuration>,
    /// Other-determined.
    pub by_others: Compared<SignedDuration>,
    /// Not marked.
    pub unmarked: Compared<SignedDuration>,
}

/// The highlights of a period: its busiest day and its top project.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Highlights {
    /// The day with the most net time; of two with one time, the earlier.
    /// `None` without time.
    pub busiest_day: Option<DayTime>,
    /// The project with the most net time; `None` without time on a
    /// project.
    pub top_project: Option<ProjectTime>,
}

/// A report beside the report of the previous period.
///
/// Each list holds its changes in the order of the report's list it
/// belongs to. The days are not compared one by one, as their dates
/// differ, and the budgets count all time; the highlights name the
/// busiest day of both periods.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReportComparison {
    /// The previous period.
    pub period: ReportPeriod,
    /// The tracked time.
    pub tracked: Compared<SignedDuration>,
    /// The untracked time.
    pub untracked: Compared<SignedDuration>,
    /// The time of each role of [`Report::roles`].
    pub roles: Vec<Compared<SignedDuration>>,
    /// The time of each project of [`Report::projects`].
    pub projects: Vec<Compared<SignedDuration>>,
    /// The time without a project.
    pub back_flow: Compared<SignedDuration>,
    /// The actual time of each target of [`Report::targets`].
    pub targets: Vec<Compared<SignedDuration>>,
    /// The revenue.
    pub revenue: ComparedRevenue,
    /// The holds.
    pub holds: ComparedHolds,
    /// Each kind of [`Report::pauses`].
    pub pauses: Vec<ComparedBreaks>,
    /// Who decided.
    pub decided: ComparedDecided,
    /// The highlights of the report's period.
    pub highlights: Highlights,
    /// The highlights of the previous period.
    pub previous_highlights: Highlights,
}

impl ReportComparison {
    /// The comparison of `current` with `previous`, the report of the
    /// period before it. A role, project or kind that only `previous`
    /// has is not listed; one that only `current` has is compared with
    /// zero.
    #[must_use]
    pub fn of(current: &Report, previous: &Report) -> Self {
        Self {
            period: previous.period.clone(),
            tracked: Compared::of_time(current.tracked, previous.tracked),
            untracked: Compared::of_time(current.untracked, previous.untracked),
            roles: matched(
                &current.roles,
                &previous.roles,
                |role| role.role,
                |role| role.net,
            ),
            projects: matched(
                &current.projects,
                &previous.projects,
                |project| project.project.clone(),
                |project| project.net,
            ),
            back_flow: Compared::of_time(current.back_flow.net, previous.back_flow.net),
            targets: matched(
                &current.targets,
                &previous.targets,
                |target| target.role,
                |target| target.actual,
            ),
            revenue: revenue_changes(current, previous),
            holds: hold_changes(&current.holds, &previous.holds),
            pauses: current
                .pauses
                .iter()
                .map(|share| {
                    let earlier = previous
                        .pauses
                        .iter()
                        .find(|other| other.kind == share.kind);
                    ComparedBreaks::of(pause_pair(share), earlier.map(pause_pair))
                })
                .collect(),
            decided: ComparedDecided {
                by_me: shares(current.decided.by_me, previous.decided.by_me),
                by_others: shares(current.decided.by_others, previous.decided.by_others),
                unmarked: shares(current.decided.unmarked, previous.decided.unmarked),
            },
            highlights: current.highlights(),
            previous_highlights: previous.highlights(),
        }
    }
}

impl Report {
    /// The busiest day and the top project of the report.
    #[must_use]
    pub fn highlights(&self) -> Highlights {
        let busiest_day = self
            .days
            .iter()
            .filter(|day| day.net > SignedDuration::ZERO)
            .fold(None::<&DayTime>, |busiest, day| match busiest {
                Some(busiest) if busiest.net >= day.net => Some(busiest),
                _ => Some(day),
            })
            .copied();
        let top_project = self
            .projects
            .first()
            .filter(|project| project.net > SignedDuration::ZERO)
            .cloned();
        Highlights {
            busiest_day,
            top_project,
        }
    }
}

/// The time of each of `current` beside the time of the item of
/// `previous` with the same key, or zero.
fn matched<T, K: PartialEq>(
    current: &[T],
    previous: &[T],
    key: impl Fn(&T) -> K,
    time: impl Fn(&T) -> SignedDuration,
) -> Vec<Compared<SignedDuration>> {
    current
        .iter()
        .map(|item| {
            let own = key(item);
            let earlier = previous
                .iter()
                .find(|other| key(other) == own)
                .map_or(SignedDuration::ZERO, &time);
            Compared::of_time(time(item), earlier)
        })
        .collect()
}

fn shares(current: TimeShare, previous: TimeShare) -> Compared<SignedDuration> {
    Compared::of_time(current.net, previous.net)
}

const fn pause_pair(share: &PauseShare) -> (usize, SignedDuration) {
    (share.count, share.time)
}

const fn hold_pair(share: &HoldShare) -> (usize, SignedDuration) {
    (share.count, share.time)
}

fn revenue_changes(current: &Report, previous: &Report) -> ComparedRevenue {
    let (now, before) = (&current.revenue, &previous.revenue);
    ComparedRevenue {
        projects: now
            .projects
            .iter()
            .map(|share| {
                let earlier = before
                    .projects
                    .iter()
                    .find(|other| other.project == share.project);
                Compared::of_money(&share.amount, earlier.map(|other| &other.amount))
            })
            .collect(),
        clients: now
            .clients
            .iter()
            .map(|share| {
                let earlier = before
                    .clients
                    .iter()
                    .find(|other| other.client == share.client);
                Compared::of_money(&share.amount, earlier.map(|other| &other.amount))
            })
            .collect(),
        totals: now
            .totals
            .iter()
            .filter_map(|total| {
                let earlier = before
                    .totals
                    .iter()
                    .find(|other| other.currency() == total.currency());
                Compared::of_money(total, earlier)
            })
            .collect(),
    }
}

fn hold_changes(current: &HoldReport, previous: &HoldReport) -> ComparedHolds {
    ComparedHolds {
        total: ComparedBreaks::of(
            (current.count, current.time),
            Some((previous.count, previous.time)),
        ),
        by_source: current
            .by_source
            .iter()
            .map(|share| {
                let earlier = previous
                    .by_source
                    .iter()
                    .find(|other| other.source == share.source);
                ComparedBreaks::of(hold_pair(share), earlier.map(hold_pair))
            })
            .collect(),
        reasons: current
            .reasons
            .iter()
            .map(|reason| {
                let earlier = previous
                    .reasons
                    .iter()
                    .find(|other| other.reason == reason.reason)
                    .map_or(0, |other| other.count);
                Compared::of_count(reason.count, earlier)
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_percent_rounds_half_away_from_zero() {
        assert_eq!(change_percent(1, 200), Some(1));
        assert_eq!(change_percent(-1, 200), Some(-1));
        assert_eq!(change_percent(1, 3), Some(33));
        assert_eq!(change_percent(-2, 3), Some(-67));
        assert_eq!(change_percent(5, 0), None);
    }
}
