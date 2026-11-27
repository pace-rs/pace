//! The bill of `bill` as rows of the schema, for `--format json`, `csv`
//! and its templates: the client and the period, the invoice once
//! written, the lines of each project with their entries, the project's
//! totals, the bill's totals, the share of hold time, and the running
//! entries it leaves out.

use pace_core::{Bill, BillLine, Catalog, Client, ProjectBill, State};
use pace_time::TimeRange;

use crate::render::{
    bill_period::BillPeriod,
    schema::{Column, Length, Row},
};

/// What the rows show: the bill, and what names its parts.
#[derive(Debug, Clone, Copy)]
pub struct BillRows<'bill> {
    /// The bill.
    pub bill: &'bill Bill,
    /// The client the bill is for.
    pub client: &'bill Client,
    /// The bill's period.
    pub range: &'bill TimeRange,
    /// The period in words.
    pub period: BillPeriod,
    /// The project records, for their keys and names.
    pub catalog: &'bill Catalog,
    /// The state the bill was built from, for short IDs and notes.
    pub state: &'bill State,
}

/// The rows of the bill, in the order of its table.
///
/// First `bill` `client`, then `bill` `invoice` with the reference of a
/// written invoice; per project its `line` rows, each followed by
/// its `entry` rows, then its `project` row; `bill` `total` per currency;
/// `interruptions` `hold_share`; and `left_out` `running` per running
/// entry.
#[must_use]
pub fn bill_rows(rows: &BillRows<'_>) -> Vec<Row> {
    let BillRows {
        bill,
        client,
        range,
        period,
        state,
        ..
    } = *rows;
    let mut client_row = Row::of("bill", "client")
        .text(Column::Key, client.key.to_string())
        .text(Column::Name, client.name.clone())
        .text(Column::Period, period.label());
    if let Some(start) = range.start() {
        client_row = client_row.time(Column::Start, start);
    }
    if let Some(end) = range.end() {
        client_row = client_row.time(Column::End, end);
    }
    let mut all = vec![client_row];
    all.extend(
        bill.invoice
            .iter()
            .map(|invoice| Row::of("bill", "invoice").text(Column::Key, invoice.to_string())),
    );
    for project in &bill.projects {
        all.extend(project_rows(project, rows));
    }
    all.extend(bill.totals.iter().map(|total| {
        Row::of("bill", "total")
            .text(Column::Client, client.key.to_string())
            .duration(Length::Net, bill.net)
            .duration(Length::Billed, bill.billed)
            .money(total)
    }));
    all.push(
        Row::of("interruptions", "hold_share")
            .duration(Length::Gross, bill.hold_share.gross)
            .duration(Length::Held, bill.hold_share.hold)
            .number(Column::Percent, bill.hold_share.percent),
    );
    all.extend(bill.running.iter().filter_map(|id| {
        let entry = state.entry(*id)?;
        Some(
            Row::of("left_out", "running")
                .text(Column::Id, id.to_string())
                .text(Column::Short, state.short_id(*id))
                .time(Column::Start, &entry.start)
                .maybe_text(Column::Note, entry.note.clone()),
        )
    }));
    all
}

/// The project's cells: its key, the ID of its record, its name and its
/// client.
fn with_project(row: Row, project: &ProjectBill, rows: &BillRows<'_>) -> Row {
    let record = rows.catalog.resolve(&project.project);
    row.maybe_text(Column::Key, record.map(|record| record.key.to_string()))
        .maybe_text(Column::Id, record.map(|record| record.id.to_string()))
        .maybe_text(Column::Name, record.map(|record| record.name.clone()))
        .text(Column::Client, rows.client.key.to_string())
}

/// The rows of one project: each line and its entries, then the totals.
fn project_rows(project: &ProjectBill, rows: &BillRows<'_>) -> Vec<Row> {
    let mut all = Vec::new();
    for line in &project.lines {
        all.push(line_row(line, project, rows));
        let key = rows
            .catalog
            .resolve(&project.project)
            .map(|record| record.key.to_string());
        all.extend(line.entries.iter().map(|entry| {
            Row::of("bill", "entry")
                .maybe_text(Column::Key, key.clone())
                .text(Column::Id, entry.id.to_string())
                .text(Column::Short, rows.state.short_id(entry.id))
                .text(Column::Client, rows.client.key.to_string())
                .text(Column::Date, line.date.to_string())
                .duration(Length::Net, entry.net)
                .maybe_text(Column::Note, entry.note.clone())
        }));
    }
    all.push(
        with_project(Row::of("bill", "project"), project, rows)
            .count(project.lines.iter().map(|line| line.entries.len()).sum())
            .duration(Length::Net, project.net)
            .duration(Length::Billed, project.billed)
            .money(&project.amount),
    );
    all
}

/// The row of one line: its day, its entries' count and notes, its time,
/// rate and amount.
fn line_row(line: &BillLine, project: &ProjectBill, rows: &BillRows<'_>) -> Row {
    let notes: Vec<&str> = line
        .entries
        .iter()
        .filter_map(|entry| entry.note.as_deref())
        .collect();
    let row = with_project(Row::of("bill", "line"), project, rows)
        .text(Column::Date, line.date.to_string())
        .count(line.entries.len())
        .duration(Length::Net, line.net)
        .duration(Length::Billed, line.billed)
        .rate(&line.rate)
        .money(&line.amount);
    if notes.is_empty() {
        row
    } else {
        row.text(Column::Note, notes.join("; "))
    }
}
