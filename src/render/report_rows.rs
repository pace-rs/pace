//! The report of `reflect` as rows of the schema, for `--format json` and
//! `--format csv`: one row per line of the report, each under its part.
//!
//! Unlike the table, the rows hide nothing: a day, a source of holds or a
//! share of who decided without time is a row with zero seconds.

use jiff::SignedDuration;
use pace_core::{
    BudgetUse, Catalog, Charge, ClientRevenue, HoldSource, PauseKind, ProjectRef, ProjectRevenue,
    Report, ReportWarning, RoleId, RoleTarget, State, TimeShare,
};

use crate::render::{
    percent,
    schema::{Column, Length, Row},
};

/// What the rows show: the report, and what names its parts.
#[derive(Debug, Clone, Copy)]
pub struct ReportRows<'report> {
    /// The report.
    pub report: &'report Report,
    /// The roles, clients and project records, for their keys and names.
    pub catalog: &'report Catalog,
    /// The state of the period, for the short IDs of entries.
    pub state: &'report State,
}

/// The rows of the report, part by part in the table's order: `time`,
/// `target`, `budget`, `revenue`, `interruptions`, `pauses`, `decided` and
/// `overlaps`.
#[must_use]
pub fn report_rows(rows: &ReportRows<'_>) -> Vec<Row> {
    let mut all = time_rows(rows);
    all.extend(target_rows(rows));
    all.extend(budget_rows(rows));
    all.extend(revenue_rows(rows));
    all.extend(interruption_rows(rows.report));
    all.extend(pause_rows(rows));
    all.extend(decided_rows(rows.report));
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
        report, catalog, ..
    } = *rows;
    let whole = report.tracked;
    let mut all = vec![
        Row::of("time", "tracked")
            .time(Column::Start, &report.period.start)
            .time(Column::End, &report.period.end)
            .duration(Length::Net, report.tracked)
            .flag(Column::Running, !report.running.is_empty()),
        Row::of("time", "untracked").duration(Length::Net, report.untracked),
    ];
    all.extend(report.roles.iter().map(|role| {
        let row = Row::of("time", "role");
        let row = match role.role {
            Some(id) => with_role(row, id, catalog),
            None => row,
        };
        with_share(row, share(role.net, role.running), whole)
    }));
    all.extend(report.projects.iter().map(|project| {
        let row = with_project(Row::of("time", "project"), &project.project, catalog);
        with_share(row, share(project.net, project.running), whole)
    }));
    all.push(with_share(
        Row::of("time", "project"),
        report.back_flow,
        whole,
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
        .map(|target| target_row(target, rows.catalog))
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
    let mut all = Vec::new();
    for client in &revenue.clients {
        all.push(client_row(client, rows.catalog));
        all.extend(
            revenue
                .projects
                .iter()
                .filter(|project| project.client == client.client)
                .map(|project| revenue_project_row(project, rows.catalog)),
        );
    }
    all.extend(
        revenue
            .totals
            .iter()
            .map(|total| Row::of("revenue", "total").money(total)),
    );
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
fn interruption_rows(report: &Report) -> Vec<Row> {
    let holds = &report.holds;
    let mut all = vec![
        Row::of("interruptions", "total")
            .count(holds.count)
            .duration(Length::Held, holds.time),
    ];
    all.extend(holds.by_source.iter().map(|share| {
        let source = match share.source {
            HoldSource::Inner => "inner",
            HoldSource::Outer => "outer",
            HoldSource::Unknown => "unknown",
        };
        Row::of("interruptions", "source")
            .text(Column::Key, source)
            .count(share.count)
            .duration(Length::Held, share.time)
    }));
    all.extend(holds.reasons.iter().map(|reason| {
        Row::of("interruptions", "reason")
            .text(Column::Reason, reason.reason.clone())
            .count(reason.count)
    }));
    all
}

/// The pauses by kind, and each catch with its entry.
fn pause_rows(rows: &ReportRows<'_>) -> Vec<Row> {
    let report = rows.report;
    let mut all: Vec<Row> = report
        .pauses
        .iter()
        .map(|share| {
            let kind = match share.kind {
                PauseKind::Break => "break",
                PauseKind::Lunch => "lunch",
                PauseKind::Pomodoro => "pomodoro",
                PauseKind::Recovery => "recovery",
            };
            Row::of("pauses", "kind")
                .text(Column::Key, kind)
                .count(share.count)
                .duration(Length::Paused, share.time)
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
fn decided_rows(report: &Report) -> Vec<Row> {
    let decided = &report.decided;
    [
        ("self", decided.by_me),
        ("other", decided.by_others),
        ("unmarked", decided.unmarked),
    ]
    .into_iter()
    .map(|(key, share)| {
        with_share(
            Row::of("decided", "determined_by").text(Column::Key, key),
            share,
            report.tracked,
        )
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
