//! The bill of `bill` as a table for the terminal: per project its lines
//! with date, note, hours, rate and amount, and its total; then the
//! bill's totals, the share of hold time and the running entries it
//! leaves out.

use pace_core::{Bill, BillLine, Catalog, Client, Money, ProjectBill, State};
use pace_time::format_duration;

use crate::output::project_label;
use crate::render::{Canvas, bill_period::BillPeriod};

/// The indent of a line under its heading.
const INDENT: &str = "  ";

/// The narrowest the note column gets on a narrow canvas.
const SHORTEST_NOTE: usize = 8;

/// What the table shows: the bill, and what names its parts.
#[derive(Debug, Clone, Copy)]
pub struct BillTable<'bill> {
    /// The bill.
    pub bill: &'bill Bill,
    /// The client the bill is for.
    pub client: &'bill Client,
    /// The bill's period in words.
    pub period: BillPeriod,
    /// The name of the report's zone.
    pub zone: &'bill str,
    /// The project records, for their names.
    pub catalog: &'bill Catalog,
    /// The state the bill was built from, for short IDs and notes.
    pub state: &'bill State,
}

/// The lines of the table, fitted to `canvas`.
#[must_use]
pub fn bill_table(table: &BillTable<'_>, canvas: Canvas) -> Vec<String> {
    let BillTable {
        bill,
        client,
        period,
        zone,
        ..
    } = *table;
    let mut lines = canvas.wrap(
        &format!("Bill for {}: {}, {zone}", client.name, period.label()),
        "",
    );
    for project in &bill.projects {
        lines.push(String::new());
        lines.extend(project_part(project, table, canvas));
    }
    lines.push(String::new());
    lines.extend(canvas.wrap(&totals_sentence(bill), ""));
    let share = bill.hold_share;
    lines.extend(canvas.wrap(
        &format!(
            "Hold time: {} of {} ({}%).",
            format_duration(share.hold),
            format_duration(share.gross),
            share.percent
        ),
        "",
    ));
    for line in running_sentences(table) {
        lines.extend(canvas.wrap(&line, ""));
    }
    lines
}

/// `Total: 6h 15m, 712.50 EUR.`, with the net time when rounding changed
/// it: `Total: 6h 15m billed of 5h 55m net, 712.50 EUR.`
fn totals_sentence(bill: &Bill) -> String {
    let time = if bill.billed == bill.net {
        format_duration(bill.billed)
    } else {
        format!(
            "{} billed of {} net",
            format_duration(bill.billed),
            format_duration(bill.net)
        )
    };
    let amounts: Vec<String> = bill.totals.iter().map(amount).collect();
    format!("Total: {time}, {}.", amounts.join(", "))
}

/// `Left out ab12 "Write the offer", which is running; end it to bill
/// it.`, one per running entry.
fn running_sentences(table: &BillTable<'_>) -> Vec<String> {
    table
        .bill
        .running
        .iter()
        .filter_map(|id| table.state.entry(*id))
        .map(|entry| {
            format!(
                "Left out {}, which is running; end it to bill it.",
                table.state.entry_name(entry)
            )
        })
        .collect()
}

/// `95.50 EUR`.
fn amount(money: &Money) -> String {
    format!("{money} {}", money.currency().code())
}

/// The heading of `project`, its lines under a row of column names, and
/// its total.
fn project_part(project: &ProjectBill, table: &BillTable<'_>, canvas: Canvas) -> Vec<String> {
    let mut rows = vec![
        ["Date", "Note", "Hours", "Rate", "Amount"]
            .map(str::to_owned)
            .to_vec(),
    ];
    rows.extend(project.lines.iter().map(line_cells));
    rows.push(vec![
        "Total".to_owned(),
        String::new(),
        format_duration(project.billed),
        String::new(),
        amount(&project.amount),
    ]);
    let mut lines = vec![canvas.heading(&project_label(&project.project, table.catalog))];
    lines.extend(bill_columns(&rows, canvas));
    lines
}

/// The cells of `line`: its day, the notes of its entries, its billed
/// time, its rate and its amount.
fn line_cells(line: &BillLine) -> Vec<String> {
    let notes: Vec<&str> = line
        .entries
        .iter()
        .filter_map(|entry| entry.note.as_deref())
        .collect();
    vec![
        line.date.to_string(),
        notes.join("; "),
        format_duration(line.billed),
        amount(&line.rate),
        amount(&line.amount),
    ]
}

/// Cells in columns, each line indented: the date and the note
/// left-aligned, the numbers right, each column as wide as its widest
/// cell. On a canvas too narrow for the row, the note is cut, down to the
/// shortest a note keeps.
fn bill_columns(rows: &[Vec<String>], canvas: Canvas) -> Vec<String> {
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
    if let Some(note) = widths.get_mut(1) {
        let over = row_width.saturating_sub(canvas.width);
        *note = note.saturating_sub(over).max(SHORTEST_NOTE.min(*note));
    }
    rows.iter()
        .map(|row| {
            let cells: Vec<String> = row
                .iter()
                .zip(&widths)
                .enumerate()
                .map(|(column, (cell, width))| match column {
                    0 => format!("{cell:<width$}"),
                    1 => format!("{:<width$}", canvas.fit(cell, *width)),
                    _ => format!("{cell:>width$}"),
                })
                .collect();
            format!("{INDENT}{}", cells.join("  ").trim_end())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NARROW: Canvas = Canvas {
        width: 40,
        colour: false,
        symbols: true,
    };

    fn row(cells: [&str; 5]) -> Vec<String> {
        cells.map(str::to_owned).to_vec()
    }

    #[test]
    fn columns_align_text_left_and_numbers_right() {
        let rows = [
            row(["Date", "Note", "Hours", "Rate", "Amount"]),
            row(["2026-10-05", "Plan", "2h 15m", "1.00", "2.25"]),
        ];

        let lines = bill_columns(
            &rows,
            Canvas {
                width: 80,
                ..NARROW
            },
        );

        assert_eq!(
            lines,
            [
                "  Date        Note   Hours  Rate  Amount",
                "  2026-10-05  Plan  2h 15m  1.00    2.25",
            ]
        );
    }

    #[test]
    fn narrow_canvas_cuts_note_with_mark() {
        let rows = [row([
            "2026-10-05",
            "Plan the relaunch of the webshop",
            "2h",
            "9.00",
            "18.00",
        ])];

        let lines = bill_columns(&rows, NARROW);

        assert_eq!(lines, ["  2026-10-05  Plan the…  2h  9.00  18.00"]);
    }
}
