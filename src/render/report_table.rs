//! The report of `reflect` as a table for the terminal: its parts in the
//! plan's order, with bars for the time per role, project and day.

use jiff::{SignedDuration, ZonedDateTime, civil::Date, tz::TimeZone};
use pace_core::{
    BudgetUse, Catalog, Charge, ClientRevenue, PauseKind, ProjectRevenue, Report, ReportWarning,
    RoleTarget, State,
};
use pace_time::{TimeFrame, format_duration, format_moment};

use crate::config::hold_source_name;
use crate::output::{counted, project_label, role_label};
use crate::render::{Canvas, percent};

/// The widest label a row shows; a longer one is cut.
const LABEL_WIDTH: usize = 24;

/// The narrowest a label gets on a narrow canvas.
const SHORTEST_LABEL: usize = 8;

/// The widest bar; a wider terminal leaves the rest empty.
const BAR_WIDTH: usize = 40;

/// The longest period whose days the table lists one by one.
const MOST_DAYS: u32 = 31;

/// The mark of a time that counts a running entry up to now.
const RUNNING: &str = "*";

/// What the table shows: the report, and what names its parts.
#[derive(Debug, Clone, Copy)]
pub struct ReportTable<'report> {
    /// The report.
    pub report: &'report Report,
    /// The roles, clients and project records, for their names.
    pub catalog: &'report Catalog,
    /// The state of the period, for the short IDs of entries.
    pub state: &'report State,
    /// The period as the user asked for it.
    pub frame: &'report TimeFrame,
    /// Now, in the report's zone.
    pub now: &'report ZonedDateTime,
    /// The report's zone.
    pub zone: &'report TimeZone,
}

/// The sentence for a period without entries: `Nothing tracked this
/// week.`
#[must_use]
pub fn nothing_tracked(frame: &TimeFrame) -> String {
    format!("Nothing tracked {}.", period_words(frame))
}

/// The period in words, as a sentence ends with it: `this week`, `from
/// 2026-11-02`; `yet` for `all`.
fn period_words(frame: &TimeFrame) -> String {
    match frame {
        TimeFrame::All => "yet".to_owned(),
        TimeFrame::Between {
            from: Some(from),
            to: Some(to),
        } => format!("from {from} to {to}"),
        TimeFrame::Between { .. } => frame.to_string(),
        named => named.to_string().replace('-', " "),
    }
}

/// The lines of the table, fitted to `canvas`.
#[must_use]
pub fn report_table(table: &ReportTable<'_>, canvas: Canvas) -> Vec<String> {
    let parts = [
        time_part(table, canvas),
        targets_part(table, canvas),
        budgets_part(table, canvas),
        revenue_part(table, canvas),
        holds_part(table, canvas),
        pauses_part(table, canvas),
        decided_part(table, canvas),
        overlaps_part(table, canvas),
    ];
    let mut lines = canvas.wrap(&title(table), "");
    lines.push(summary(table.report));
    for part in parts.into_iter().filter(|part| !part.is_empty()) {
        lines.push(String::new());
        lines.extend(part);
    }
    if table.report.marks_running() {
        lines.push(String::new());
        lines.push(format!("{RUNNING} counts a running entry up to now."));
    }
    lines
}

/// `This week: Mon 2026-11-23 to Sun 2026-11-29, Europe/Berlin`.
fn title(table: &ReportTable<'_>) -> String {
    let period = &table.report.period;
    let first = period.start.date();
    let last = period
        .end
        .checked_sub(SignedDuration::from_nanos(1))
        .map_or(first, |end| end.date())
        .max(first);
    let days = if first == last {
        day(first)
    } else {
        format!("{} to {}", day(first), day(last))
    };
    let zone = table.zone.iana_name().unwrap_or("the system's time zone");
    match table.frame {
        TimeFrame::Between { .. } => format!("{days}, {zone}"),
        TimeFrame::All => format!("All time: {days}, {zone}"),
        named => format!("{}: {days}, {zone}", capitalised(&period_words(named))),
    }
}

/// `Mon 2026-11-23`.
fn day(date: Date) -> String {
    date.strftime("%a %Y-%m-%d").to_string()
}

fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// `Tracked 6h 15m, untracked 80h 45m.`
fn summary(report: &Report) -> String {
    format!(
        "Tracked {}, untracked {}.",
        format_duration(report.tracked),
        format_duration(report.untracked)
    )
}

/// A row with a bar: a label, its time, whether that counts a running
/// entry, and its share of the tracked time.
struct BarRow {
    label: String,
    net: SignedDuration,
    running: bool,
}

impl BarRow {
    fn new(label: impl Into<String>, net: SignedDuration, running: bool) -> Self {
        Self {
            label: label.into(),
            net,
            running,
        }
    }
}

/// `rows` under `heading`, each with its time, its share of `whole` and a
/// bar scaled to the largest row, fitted to `canvas`.
fn bar_part(heading: &str, rows: &[BarRow], whole: SignedDuration, canvas: Canvas) -> Vec<String> {
    let longest = rows
        .iter()
        .map(|row| row.label.chars().count())
        .max()
        .unwrap_or(0);
    let none = if canvas.symbols { "–" } else { "-" };
    let times: Vec<String> = rows
        .iter()
        .map(|row| {
            if row.net.is_zero() && !row.running {
                none.to_owned()
            } else {
                marked(row.net, row.running)
            }
        })
        .collect();
    let time_width = times
        .iter()
        .map(|time| time.chars().count())
        .max()
        .unwrap_or(0);
    let columns = 2 + 2 + time_width + 2 + "100%".len() + 2;
    // a narrow canvas cuts the labels, down to the shortest a label keeps
    let room = canvas.width.saturating_sub(columns).max(SHORTEST_LABEL);
    let label_width = longest.min(LABEL_WIDTH).min(room);
    let fixed = columns + label_width;
    let bar_width = canvas.width.saturating_sub(fixed).min(BAR_WIDTH);
    let largest = rows.iter().map(|row| row.net.as_secs()).max().unwrap_or(0);

    let mut lines = vec![canvas.heading(heading)];
    for (row, time) in rows.iter().zip(&times) {
        let label = canvas.fit(&row.label, label_width);
        let share = percent(row.net, whole)
            .filter(|_| !row.net.is_zero())
            .map_or_else(String::new, |share| format!("{share}%"));
        let bar = canvas.bar(row.net.as_secs(), largest, bar_width);
        let line = format!("  {label:<label_width$}  {time:>time_width$}  {share:>4}  {bar}");
        lines.push(line.trim_end().to_owned());
    }
    lines
}

/// Time per role, per project with the back-flow, and per day.
fn time_part(table: &ReportTable<'_>, canvas: Canvas) -> Vec<String> {
    let ReportTable {
        report, catalog, ..
    } = *table;
    let roles: Vec<BarRow> = report
        .roles
        .iter()
        .map(|role| {
            let label = role
                .role
                .map_or_else(|| "No role".to_owned(), |id| role_label(id, catalog));
            BarRow::new(label, role.net, role.running)
        })
        .collect();
    let mut projects: Vec<BarRow> = report
        .projects
        .iter()
        .map(|project| {
            BarRow::new(
                project_label(&project.project, catalog),
                project.net,
                project.running,
            )
        })
        .collect();
    let back_flow = &report.back_flow;
    if !back_flow.net.is_zero() || back_flow.running {
        projects.push(BarRow::new("No project", back_flow.net, back_flow.running));
    }

    let mut lines = bar_part("Time per role", &roles, report.tracked, canvas);
    lines.push(String::new());
    lines.extend(bar_part(
        "Time per project",
        &projects,
        report.tracked,
        canvas,
    ));
    if report.period.days > 1 && report.period.days <= MOST_DAYS {
        let days: Vec<BarRow> = report
            .days
            .iter()
            .map(|day_time| BarRow::new(day(day_time.date), day_time.net, day_time.running))
            .collect();
        lines.push(String::new());
        lines.extend(bar_part("Time per day", &days, report.tracked, canvas));
    }
    lines
}

/// The indent of the lines of a part.
const INDENT: &str = "  ";

/// `text` as lines of a part, wrapped to `canvas` after the indent.
fn sentence(text: &str, canvas: Canvas) -> Vec<String> {
    canvas.wrap(text, INDENT)
}

/// Cells in columns: the first column left-aligned, the others right, each
/// as wide as its widest cell; each line indented by two spaces. On a
/// canvas too narrow for the row, the first column is cut, down to the
/// shortest a label keeps.
fn columns(rows: &[Vec<String>], canvas: Canvas) -> Vec<String> {
    let count = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths: Vec<usize> = (0..count)
        .map(|column| {
            rows.iter()
                .filter_map(|row| row.get(column))
                .map(|cell| cell.chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    let row_width = INDENT.len() + widths.iter().sum::<usize>() + 2 * count.saturating_sub(1);
    if let Some(first) = widths.first_mut() {
        let over = row_width.saturating_sub(canvas.width);
        *first = first.saturating_sub(over).max(SHORTEST_LABEL.min(*first));
    }
    rows.iter()
        .map(|row| {
            let cells: Vec<String> = row
                .iter()
                .zip(&widths)
                .enumerate()
                .map(|(column, (cell, width))| {
                    if column == 0 {
                        let cell = canvas.fit(cell, *width);
                        format!("{cell:<width$}")
                    } else {
                        format!("{cell:>width$}")
                    }
                })
                .collect();
            format!("{INDENT}{}", cells.join("  "))
                .trim_end()
                .to_owned()
        })
        .collect()
}

/// The weekly target of each role, scaled to the period, against its
/// time; and the warning when the targets add up to more than a week.
fn targets_part(table: &ReportTable<'_>, canvas: Canvas) -> Vec<String> {
    let report = table.report;
    let above_week = report.warnings.iter().find_map(|warning| match warning {
        ReportWarning::TargetsAboveWeek { total } => Some(*total),
        ReportWarning::Overlap { .. } => None,
    });
    if report.targets.is_empty() && above_week.is_none() {
        return Vec::new();
    }
    let rows: Vec<Vec<String>> = report
        .targets
        .iter()
        .map(|target| target_row(target, table.catalog, canvas))
        .collect();
    let mut lines = vec![canvas.heading("Target vs. actual")];
    lines.extend(columns(&rows, canvas));
    if let Some(total) = above_week {
        lines.extend(sentence(
            &format!(
                "The weekly targets add up to {}, more than the 168h of a week.",
                format_duration(total)
            ),
            canvas,
        ));
    }
    lines
}

fn target_row(target: &RoleTarget, catalog: &Catalog, canvas: Canvas) -> Vec<String> {
    let share =
        percent(target.actual, target.target).map_or_else(String::new, |share| format!("{share}%"));
    vec![
        canvas.fit(&role_label(target.role, catalog), LABEL_WIDTH),
        marked(target.actual, target.running),
        format!("of {}", format_duration(target.target)),
        share,
    ]
}

/// Each budget: the time used to date, over all time, against it.
fn budgets_part(table: &ReportTable<'_>, canvas: Canvas) -> Vec<String> {
    let budgets = &table.report.budgets;
    if budgets.is_empty() {
        return Vec::new();
    }
    let rows: Vec<Vec<String>> = budgets
        .iter()
        .map(|budget| budget_row(budget, table.catalog, canvas))
        .collect();
    let mut lines = vec![canvas.heading("Budgets, used to date")];
    lines.extend(columns(&rows, canvas));
    lines
}

fn budget_row(budget: &BudgetUse, catalog: &Catalog, canvas: Canvas) -> Vec<String> {
    let mut share = budget
        .percent
        .map_or_else(String::new, |share| format!("{share}%"));
    if budget.used > budget.budget {
        share = format!(
            "{share}, {} over",
            format_duration(budget.used - budget.budget)
        );
    }
    vec![
        canvas.fit(&project_label(&budget.project, catalog), LABEL_WIDTH),
        marked(budget.used, budget.running),
        format!("of {}", format_duration(budget.budget)),
        share,
    ]
}

/// What the billable time earns, per client and its projects, the totals
/// per currency, and the time without a rate.
fn revenue_part(table: &ReportTable<'_>, canvas: Canvas) -> Vec<String> {
    let revenue = &table.report.revenue;
    if revenue.clients.is_empty() && revenue.no_rate.is_empty() {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for client in &revenue.clients {
        rows.push(client_row(client, table.catalog, canvas));
        rows.extend(
            revenue
                .projects
                .iter()
                .filter(|project| project.client == client.client)
                .map(|project| project_row(project, table.catalog, canvas)),
        );
    }
    for total in &revenue.totals {
        rows.push(vec![
            "Total".to_owned(),
            String::new(),
            format!("{total} {}", total.currency().code()),
        ]);
    }
    let mut lines = vec![canvas.heading("Revenue")];
    lines.extend(columns(&rows, canvas));
    for charge in &revenue.no_rate {
        lines.extend(sentence(&no_rate_line(charge, table.catalog), canvas));
    }
    lines
}

fn client_row(client: &ClientRevenue, catalog: &Catalog, canvas: Canvas) -> Vec<String> {
    let name = catalog
        .client(&client.client)
        .map_or_else(|| client.client.to_string(), |known| known.name.clone());
    vec![
        canvas.fit(&name, LABEL_WIDTH),
        marked(client.billed, client.running),
        format!("{} {}", client.amount, client.amount.currency().code()),
    ]
}

fn project_row(project: &ProjectRevenue, catalog: &Catalog, canvas: Canvas) -> Vec<String> {
    vec![
        format!(
            "  {}",
            canvas.fit(&project_label(&project.project, catalog), LABEL_WIDTH)
        ),
        marked(project.billed, project.running),
        format!("{} {}", project.amount, project.amount.currency().code()),
    ]
}

/// A duration with the running mark when it counts a running entry.
fn marked(duration: SignedDuration, running: bool) -> String {
    let mark = if running { RUNNING } else { "" };
    format!("{}{mark}", format_duration(duration))
}

/// `No rate on 2026-11-24 for 1h 30m of Webshop; `pace client edit acme`
/// gives it one.`
fn no_rate_line(charge: &Charge, catalog: &Catalog) -> String {
    let project = charge.project.as_ref().map_or_else(
        || "entries without a project".to_owned(),
        |project| project_label(project, catalog),
    );
    let fix = charge.client.as_ref().map_or_else(
        || "no client bills it".to_owned(),
        |client| format!("`pace client edit {client}` gives it a rate"),
    );
    format!(
        "No rate on {} for {} of {project}; {fix}.",
        charge.date,
        marked(charge.billed, charge.running)
    )
}

/// The holds: how many, how long, from where, and the most frequent
/// reasons.
fn holds_part(table: &ReportTable<'_>, canvas: Canvas) -> Vec<String> {
    let holds = &table.report.holds;
    if holds.count == 0 {
        return Vec::new();
    }
    let sources: Vec<String> = holds
        .by_source
        .iter()
        .map(|share| {
            let name = hold_source_name(share.source);
            if share.count == 0 {
                format!("{name} 0")
            } else {
                format!("{name} {} ({})", share.count, format_duration(share.time))
            }
        })
        .collect();
    let mut lines = vec![canvas.heading("Interruptions")];
    lines.extend(sentence(
        &format!(
            "{}, {}: {}.",
            counted(holds.count, "hold"),
            format_duration(holds.time),
            sources.join(", ")
        ),
        canvas,
    ));
    if !holds.reasons.is_empty() {
        let reasons: Vec<String> = holds
            .reasons
            .iter()
            .map(|reason| format!("{} {}", reason.reason, reason.count))
            .collect();
        lines.extend(sentence(
            &format!("Most frequent reasons: {}.", reasons.join(", ")),
            canvas,
        ));
    }
    lines
}

/// The pauses by kind, and the number of catches.
fn pauses_part(table: &ReportTable<'_>, canvas: Canvas) -> Vec<String> {
    let report = table.report;
    if report.pauses.is_empty() && report.catches.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![canvas.heading("Pauses and catches")];
    if !report.pauses.is_empty() {
        let pauses: Vec<String> = report
            .pauses
            .iter()
            .map(|share| {
                let kind = match share.kind {
                    PauseKind::Break => "break",
                    PauseKind::Lunch => "lunch",
                    PauseKind::Pomodoro => "Pomodoro",
                    PauseKind::Recovery => "recovery",
                };
                format!("{kind} {} ({})", share.count, format_duration(share.time))
            })
            .collect();
        lines.extend(sentence(&format!("Pauses: {}.", pauses.join(", ")), canvas));
    }
    if !report.catches.is_empty() {
        lines.extend(sentence(
            &format!("{}.", capitalised(&counted(report.catches.len(), "catch"))),
            canvas,
        ));
    }
    lines
}

/// The shares of the time by me, by others and unmarked.
fn decided_part(table: &ReportTable<'_>, canvas: Canvas) -> Vec<String> {
    let report = table.report;
    let decided = &report.decided;
    let rows = [
        BarRow::new("By me", decided.by_me.net, decided.by_me.running),
        BarRow::new(
            "By others",
            decided.by_others.net,
            decided.by_others.running,
        ),
        BarRow::new("Unmarked", decided.unmarked.net, decided.unmarked.running),
    ];
    bar_part("Who decided", &rows, report.tracked, canvas)
}

/// The overlaps, as `now` names them.
fn overlaps_part(table: &ReportTable<'_>, canvas: Canvas) -> Vec<String> {
    let overlaps: Vec<String> = table
        .report
        .warnings
        .iter()
        .filter_map(|warning| match warning {
            ReportWarning::Overlap {
                first,
                second,
                start,
                end,
            } => Some(format!(
                "{} and {} both cover {}–{}; change one with `pace adjust <id>`.",
                table.state.short_id(*first),
                table.state.short_id(*second),
                format_moment(start, table.now),
                format_moment(end, table.now)
            )),
            ReportWarning::TargetsAboveWeek { .. } => None,
        })
        .collect();
    if overlaps.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![canvas.heading("Overlaps")];
    for overlap in &overlaps {
        lines.extend(sentence(overlap, canvas));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn canvas(width: usize, symbols: bool) -> Canvas {
        Canvas {
            width,
            colour: false,
            symbols,
        }
    }

    fn rows() -> Vec<BarRow> {
        vec![
            BarRow::new(
                "A project with a very long name indeed",
                SignedDuration::from_hours(3),
                false,
            ),
            BarRow::new("Short", SignedDuration::from_mins(90), true),
            BarRow::new("Idle", SignedDuration::ZERO, false),
        ]
    }

    #[test]
    fn bar_rows_fit_narrow_canvas_and_cut_long_labels() {
        let lines = bar_part(
            "Time per project",
            &rows(),
            SignedDuration::from_mins(270),
            canvas(50, true),
        );

        assert_eq!(
            lines,
            [
                "Time per project",
                "  A project with a very l…       3h   67%  ███████",
                "  Short                     1h 30m*   33%  ███▌",
                "  Idle                            –",
            ]
        );
        assert!(lines.iter().all(|line| line.chars().count() <= 50));
    }

    #[test]
    fn bar_rows_on_plain_canvas_use_ascii() {
        let lines = bar_part(
            "Time",
            &rows(),
            SignedDuration::from_mins(270),
            canvas(60, false),
        );

        assert_eq!(
            lines[1],
            "  A project with a very l~       3h   67%  #################"
        );
        assert_eq!(lines[3], "  Idle                            -");
    }

    #[test]
    fn canvas_narrower_than_columns_drops_bars() {
        let lines = bar_part(
            "Time",
            &rows(),
            SignedDuration::from_mins(270),
            canvas(20, true),
        );

        assert_eq!(lines[1], "  A proje…       3h   67%");
        assert_eq!(lines[2], "  Short     1h 30m*   33%");
    }

    #[test]
    fn bars_never_grow_past_their_widest() {
        let lines = bar_part(
            "Time",
            &rows()[..1],
            SignedDuration::from_hours(3),
            canvas(300, true),
        );

        assert_eq!(lines[1].chars().filter(|c| *c == '█').count(), BAR_WIDTH);
    }

    #[test]
    fn columns_cut_first_column_on_narrow_canvas() {
        let rows = vec![
            vec![
                "Relaunch of the webshop".to_owned(),
                "5h 15m".to_owned(),
                "of 10h".to_owned(),
                "53%".to_owned(),
            ],
            vec![
                "Blog".to_owned(),
                "1h".to_owned(),
                "of 2h".to_owned(),
                "50%".to_owned(),
            ],
        ];

        let lines = columns(&rows, canvas(36, true));

        assert_eq!(
            lines,
            [
                "  Relaunch of …  5h 15m  of 10h  53%",
                "  Blog               1h   of 2h  50%",
            ]
        );
        assert!(
            lines.iter().all(|line| line.chars().count() <= 36),
            "{lines:#?}"
        );
        assert_eq!(
            columns(&rows, canvas(80, true))[0],
            "  Relaunch of the webshop  5h 15m  of 10h  53%"
        );
    }

    #[test]
    fn nothing_tracked_names_period() {
        assert_eq!(
            nothing_tracked(&TimeFrame::ThisWeek),
            "Nothing tracked this week."
        );
        assert_eq!(nothing_tracked(&TimeFrame::All), "Nothing tracked yet.");
        let from = jiff::civil::date(2026, 11, 2);
        assert_eq!(
            nothing_tracked(&TimeFrame::Between {
                from: Some(from),
                to: None
            }),
            "Nothing tracked from 2026-11-02."
        );
        assert_eq!(
            nothing_tracked(&TimeFrame::Between {
                from: None,
                to: Some(from)
            }),
            "Nothing tracked until 2026-11-02."
        );
    }
}
