//! The report of `reflect` as rows of the schema, for `--format json` and
//! `--format csv`: one row per line of the report, each under its part.
//!
//! Unlike the table, the rows hide nothing: a day, a source of holds or a
//! share of who decided without time is a row with zero seconds. With
//! `--compare`, a row that the comparison covers gets the cells of the
//! previous period and its change, and the part `highlights` comes first.

use jiff::SignedDuration;
use pace_core::{
    BudgetUse, Catalog, Charge, ClientRevenue, Compared, ComparedBreaks, Highlights, Money,
    PauseKind, ProjectRef, ProjectRevenue, Report, ReportComparison, ReportWarning, RoleId,
    RoleTarget, State, TimeShare,
};

use crate::config::hold_source_name;
use crate::render::{
    percent,
    report_compare::{signed_duration, signed_money},
    schema::{Column, Length, Row},
};

/// What the rows show: the report, and what names its parts.
#[derive(Debug, Clone, Copy)]
pub struct ReportRows<'report> {
    /// The report.
    pub report: &'report Report,
    /// The report beside the previous period's, for `--compare`.
    pub comparison: Option<&'report ReportComparison>,
    /// The roles, clients and project records, for their keys and names.
    pub catalog: &'report Catalog,
    /// The state of the period, for the short IDs of entries.
    pub state: &'report State,
}

/// The rows of the report, part by part in the table's order:
/// `highlights` with `--compare`, `time`, `target`, `budget`, `revenue`,
/// `interruptions`, `pauses`, `decided` and `overlaps`.
#[must_use]
pub fn report_rows(rows: &ReportRows<'_>) -> Vec<Row> {
    let mut all = rows.comparison.map_or_else(Vec::new, |comparison| {
        highlight_rows(comparison, rows.catalog)
    });
    all.extend(time_rows(rows));
    all.extend(target_rows(rows));
    all.extend(budget_rows(rows));
    all.extend(revenue_rows(rows));
    all.extend(interruption_rows(rows.report, rows.comparison));
    all.extend(pause_rows(rows));
    all.extend(decided_rows(rows.report, rows.comparison));
    all.extend(overlap_rows(rows));
    all
}

/// The row with the net time of `share`, its share of `whole` and whether
/// it counts a running entry.
fn with_share(row: Row, share: TimeShare, whole: SignedDuration) -> Row {
    let row = row
        .duration(Length::Net, share.net)
        .flag(Column::Running, share.running);
    match percent(share.net, whole) {
        Some(percent) => row.number(Column::Percent, clamped(percent)),
        None => row,
    }
}

/// The row with the previous value of a time under `length` and its
/// change, in seconds, readable and in percent.
fn with_time_change(row: Row, length: Length, change: Option<&Compared<SignedDuration>>) -> Row {
    let (Some(change), Some((seconds, readable))) = (change, length.change_columns()) else {
        return row;
    };
    let row = row
        .with_previous(Row::default().duration(length, change.previous))
        .number(seconds, change.difference.as_secs())
        .text(readable, signed_duration(change.difference));
    with_change_percent(row, change.percent)
}

/// The row with the previous count and its change; the change in percent
/// only for a row that counts and has no time.
fn with_count_change(row: Row, change: &Compared<i64>, percent: bool) -> Row {
    let row = row
        .with_previous(Row::default().number(Column::Count, change.previous))
        .number(Column::ChangeCount, change.difference);
    if percent {
        with_change_percent(row, change.percent)
    } else {
        row
    }
}

/// The row with the count and time of the previous period and their
/// change; the percent is the time's.
fn with_breaks_change(row: Row, length: Length, change: Option<&ComparedBreaks>) -> Row {
    match change {
        Some(change) => with_time_change(
            with_count_change(row, &change.count, false),
            length,
            Some(&change.time),
        ),
        None => row,
    }
}

/// The row with the previous amount and its change; nothing for amounts
/// in two currencies.
fn with_money_change(row: Row, change: Option<&Compared<Money>>) -> Row {
    let Some(change) = change else {
        return row;
    };
    let row = row
        .with_previous(Row::default().money(&change.previous))
        .text(Column::ChangeAmount, signed_money(&change.difference));
    with_change_percent(row, change.percent)
}

fn with_change_percent(row: Row, percent: Option<i64>) -> Row {
    match percent {
        Some(percent) => row.number(Column::ChangePercent, percent),
        None => row,
    }
}

/// The busiest day and the top project of the period, each with the one
/// of the previous period.
fn highlight_rows(comparison: &ReportComparison, catalog: &Catalog) -> Vec<Row> {
    let day = |row: Row, highlights: &Highlights| match highlights.busiest_day {
        Some(day) => row
            .text(Column::Date, day.date.to_string())
            .duration(Length::Net, day.net)
            .flag(Column::Running, day.running),
        None => row,
    };
    let project = |row: Row, highlights: &Highlights| match &highlights.top_project {
        Some(top) => with_project(row, &top.project, catalog)
            .duration(Length::Net, top.net)
            .flag(Column::Running, top.running),
        None => row,
    };
    let (now, before) = (&comparison.highlights, &comparison.previous_highlights);
    vec![
        day(Row::of("highlights", "busiest_day"), now)
            .with_previous(day(Row::default(), before).without_running()),
        project(Row::of("highlights", "top_project"), now)
            .with_previous(project(Row::default(), before).without_running()),
    ]
}

fn clamped(number: i128) -> i64 {
    i64::try_from(number).unwrap_or(i64::MAX)
}

/// The row with the ID of `role`, and its key and name when the catalog
/// holds it.
fn with_role(row: Row, role: RoleId, catalog: &Catalog) -> Row {
    let known = catalog.role_by_id(role);
    row.maybe_text(Column::Key, known.map(|known| known.key.to_string()))
        .text(Column::Id, role.to_string())
        .maybe_text(Column::Name, known.map(|known| known.name.clone()))
}

/// The row with the key, ID and name of `project`, those it has.
fn with_project(row: Row, project: &ProjectRef, catalog: &Catalog) -> Row {
    let record = catalog.resolve(project);
    let (key, id) = match project {
        ProjectRef::Key(key) => (Some(key.to_string()), record.map(|record| record.id)),
        ProjectRef::Id(id) => (record.map(|record| record.key.to_string()), Some(*id)),
    };
    row.maybe_text(Column::Key, key)
        .maybe_text(Column::Id, id.map(|id| id.to_string()))
        .maybe_text(Column::Name, record.map(|record| record.name.clone()))
}

/// `tracked` over the period, `untracked`, then the time per role, per
/// project (the back-flow without a key) and per day.
fn time_rows(rows: &ReportRows<'_>) -> Vec<Row> {
    let ReportRows {
        report,
        comparison,
        catalog,
        ..
    } = *rows;
    let whole = report.tracked;
    let tracked = Row::of("time", "tracked")
        .time(Column::Start, &report.period.start)
        .time(Column::End, &report.period.end)
        .duration(Length::Net, report.tracked)
        .flag(Column::Running, !report.running.is_empty());
    let tracked = match comparison {
        Some(comparison) => with_time_change(tracked, Length::Net, Some(&comparison.tracked))
            .with_previous(
                Row::default()
                    .time(Column::Start, &comparison.period.start)
                    .time(Column::End, &comparison.period.end),
            ),
        None => tracked,
    };
    let mut all = vec![
        tracked,
        with_time_change(
            Row::of("time", "untracked").duration(Length::Net, report.untracked),
            Length::Net,
            comparison.map(|comparison| &comparison.untracked),
        ),
    ];
    all.extend(report.roles.iter().enumerate().map(|(index, role)| {
        let row = Row::of("time", "role");
        let row = match role.role {
            Some(id) => with_role(row, id, catalog),
            None => row,
        };
        let change = comparison.and_then(|comparison| comparison.roles.get(index));
        let row = with_share(row, share(role.net, role.running), whole);
        with_time_change(row, Length::Net, change)
    }));
    all.extend(report.projects.iter().enumerate().map(|(index, project)| {
        let row = with_project(Row::of("time", "project"), &project.project, catalog);
        let change = comparison.and_then(|comparison| comparison.projects.get(index));
        let row = with_share(row, share(project.net, project.running), whole);
        with_time_change(row, Length::Net, change)
    }));
    all.push(with_time_change(
        with_share(Row::of("time", "project"), report.back_flow, whole),
        Length::Net,
        comparison.map(|comparison| &comparison.back_flow),
    ));
    all.extend(report.days.iter().map(|day| {
        let row = Row::of("time", "day").text(Column::Date, day.date.to_string());
        with_share(row, share(day.net, day.running), whole)
    }));
    all
}

const fn share(net: SignedDuration, running: bool) -> TimeShare {
    TimeShare { net, running }
}

/// The target of each role against its time, and the sum of the weekly
/// targets when it is more than a week.
fn target_rows(rows: &ReportRows<'_>) -> Vec<Row> {
    let mut all: Vec<Row> = rows
        .report
        .targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            let change = rows
                .comparison
                .and_then(|comparison| comparison.targets.get(index));
            with_time_change(target_row(target, rows.catalog), Length::Net, change)
        })
        .collect();
    all.extend(
        rows.report
            .warnings
            .iter()
            .filter_map(|warning| match warning {
                ReportWarning::TargetsAboveWeek { total } => {
                    Some(Row::of("target", "targets_above_week").duration(Length::Target, *total))
                }
                ReportWarning::Overlap { .. } => None,
            }),
    );
    all
}

fn target_row(target: &RoleTarget, catalog: &Catalog) -> Row {
    let row = with_role(Row::of("target", "role"), target.role, catalog)
        .duration(Length::Net, target.actual)
        .duration(Length::Target, target.target)
        .flag(Column::Running, target.running);
    match percent(target.actual, target.target) {
        Some(percent) => row.number(Column::Percent, clamped(percent)),
        None => row,
    }
}

/// Each budget: the time used to date against it.
fn budget_rows(rows: &ReportRows<'_>) -> Vec<Row> {
    rows.report
        .budgets
        .iter()
        .map(|budget| budget_row(budget, rows.catalog))
        .collect()
}

fn budget_row(budget: &BudgetUse, catalog: &Catalog) -> Row {
    let row = with_project(Row::of("budget", "project"), &budget.project, catalog)
        .duration(Length::Net, budget.used)
        .duration(Length::Budget, budget.budget)
        .flag(Column::Running, budget.running);
    match budget.percent {
        Some(percent) => row.number(Column::Percent, percent),
        None => row,
    }
}

/// Each client with its projects, the totals per currency, and the time
/// without a rate.
fn revenue_rows(rows: &ReportRows<'_>) -> Vec<Row> {
    let revenue = &rows.report.revenue;
    let changes = rows.comparison.map(|comparison| &comparison.revenue);
    let mut all = Vec::new();
    for (index, client) in revenue.clients.iter().enumerate() {
        let change = changes.and_then(|changes| changes.clients.get(index)?.as_ref());
        all.push(with_money_change(client_row(client, rows.catalog), change));
        all.extend(
            revenue
                .projects
                .iter()
                .enumerate()
                .filter(|(_, project)| project.client == client.client)
                .map(|(index, project)| {
                    let change = changes.and_then(|changes| changes.projects.get(index)?.as_ref());
                    with_money_change(revenue_project_row(project, rows.catalog), change)
                }),
        );
    }
    all.extend(revenue.totals.iter().enumerate().map(|(index, total)| {
        let change = changes.and_then(|changes| changes.totals.get(index));
        with_money_change(Row::of("revenue", "total").money(total), change)
    }));
    all.extend(
        revenue
            .no_rate
            .iter()
            .map(|charge| no_rate_row(charge, rows.catalog)),
    );
    all
}

fn client_row(client: &ClientRevenue, catalog: &Catalog) -> Row {
    Row::of("revenue", "client")
        .text(Column::Key, client.client.to_string())
        .maybe_text(
            Column::Name,
            catalog
                .client(&client.client)
                .map(|known| known.name.clone()),
        )
        .duration(Length::Billed, client.billed)
        .money(&client.amount)
        .flag(Column::Running, client.running)
}

fn revenue_project_row(project: &ProjectRevenue, catalog: &Catalog) -> Row {
    with_project(Row::of("revenue", "project"), &project.project, catalog)
        .text(Column::Client, project.client.to_string())
        .duration(Length::Billed, project.billed)
        .money(&project.amount)
        .flag(Column::Running, project.running)
}

fn no_rate_row(charge: &Charge, catalog: &Catalog) -> Row {
    let row = Row::of("revenue", "no_rate");
    let row = match &charge.project {
        Some(project) => with_project(row, project, catalog),
        None => row,
    };
    row.maybe_text(
        Column::Client,
        charge.client.as_ref().map(ToString::to_string),
    )
    .text(Column::Date, charge.date.to_string())
    .duration(Length::Net, charge.net)
    .duration(Length::Billed, charge.billed)
    .flag(Column::Running, charge.running)
}

/// The holds in total, by source, and the most frequent reasons.
fn interruption_rows(report: &Report, comparison: Option<&ReportComparison>) -> Vec<Row> {
    let holds = &report.holds;
    let changes = comparison.map(|comparison| &comparison.holds);
    let mut all = vec![with_breaks_change(
        Row::of("interruptions", "total")
            .count(holds.count)
            .duration(Length::Held, holds.time),
        Length::Held,
        changes.map(|changes| &changes.total),
    )];
    all.extend(holds.by_source.iter().enumerate().map(|(index, share)| {
        let source = hold_source_name(share.source);
        with_breaks_change(
            Row::of("interruptions", "source")
                .text(Column::Key, source)
                .count(share.count)
                .duration(Length::Held, share.time),
            Length::Held,
            changes.and_then(|changes| changes.by_source.get(index)),
        )
    }));
    all.extend(holds.reasons.iter().enumerate().map(|(index, reason)| {
        let row = Row::of("interruptions", "reason")
            .text(Column::Reason, reason.reason.clone())
            .count(reason.count);
        match changes.and_then(|changes| changes.reasons.get(index)) {
            Some(change) => with_count_change(row, change, true),
            None => row,
        }
    }));
    all
}

/// The pauses by kind, and each catch with its entry.
fn pause_rows(rows: &ReportRows<'_>) -> Vec<Row> {
    let report = rows.report;
    let mut all: Vec<Row> = report
        .pauses
        .iter()
        .enumerate()
        .map(|(index, share)| {
            let kind = match share.kind {
                PauseKind::Break => "break",
                PauseKind::Lunch => "lunch",
                PauseKind::Pomodoro => "pomodoro",
                PauseKind::Recovery => "recovery",
            };
            with_breaks_change(
                Row::of("pauses", "kind")
                    .text(Column::Key, kind)
                    .count(share.count)
                    .duration(Length::Paused, share.time),
                Length::Paused,
                rows.comparison
                    .and_then(|comparison| comparison.pauses.get(index)),
            )
        })
        .collect();
    all.extend(report.catches.iter().map(|catch| {
        Row::of("pauses", "catch")
            .text(Column::Id, catch.entry.to_string())
            .text(Column::Short, rows.state.short_id(catch.entry))
            .time(Column::Start, &catch.at)
            .text(Column::Note, catch.text.clone())
    }));
    all
}

/// The shares of the time by me (`self`), by others (`other`) and
/// unmarked, as `determined_by` names them.
fn decided_rows(report: &Report, comparison: Option<&ReportComparison>) -> Vec<Row> {
    let decided = &report.decided;
    let changes = comparison.map(|comparison| comparison.decided);
    [
        ("self", decided.by_me, changes.map(|changes| changes.by_me)),
        (
            "other",
            decided.by_others,
            changes.map(|changes| changes.by_others),
        ),
        (
            "unmarked",
            decided.unmarked,
            changes.map(|changes| changes.unmarked),
        ),
    ]
    .into_iter()
    .map(|(key, share, change)| {
        let row = with_share(
            Row::of("decided", "determined_by").text(Column::Key, key),
            share,
            report.tracked,
        );
        with_time_change(row, Length::Net, change.as_ref())
    })
    .collect()
}

/// Each overlap: the two entries and the time they share.
fn overlap_rows(rows: &ReportRows<'_>) -> Vec<Row> {
    rows.report
        .warnings
        .iter()
        .filter_map(|warning| match warning {
            ReportWarning::Overlap {
                first,
                second,
                start,
                end,
            } => Some(
                Row::of("overlaps", "overlap")
                    .text(Column::Id, first.to_string())
                    .text(Column::Short, rows.state.short_id(*first))
                    .text(Column::OtherId, second.to_string())
                    .text(Column::OtherShort, rows.state.short_id(*second))
                    .time(Column::Start, start)
                    .time(Column::End, end),
            ),
            ReportWarning::TargetsAboveWeek { .. } => None,
        })
        .collect()
}
