//! The entries of `export` as rows of the schema, of kind `export`: one row
//! per entry, with its times, its project, client and role, its own
//! fields, and its rate and amount where it has them.

use pace_core::{DeterminedBy, ExportEntry};

use crate::render::schema::{Column, Length, Row};

/// One row per entry of `entries`, in their order.
#[must_use]
pub fn export_rows(entries: &[ExportEntry]) -> Vec<Row> {
    entries.iter().map(export_row).collect()
}

/// The row of `entry`. A cell without a value stays out: the end of a
/// running entry, the project of an entry without one, the price of an
/// entry without a rate.
fn export_row(entry: &ExportEntry) -> Row {
    let mut row = Row::default()
        .text(Column::Id, entry.id.to_string())
        .text(Column::Short, entry.short.clone())
        .maybe_text(
            Column::Project,
            entry.project.as_ref().map(ToString::to_string),
        )
        .maybe_text(Column::ProjectId, entry.project_id.map(|id| id.to_string()))
        .maybe_text(
            Column::Client,
            entry.client.as_ref().map(ToString::to_string),
        )
        .maybe_text(Column::Role, entry.role.as_ref().map(ToString::to_string))
        .time(Column::Start, &entry.start)
        .duration(Length::Net, entry.net)
        .duration(Length::Gross, entry.gross)
        .duration(Length::Held, entry.held)
        .duration(Length::Paused, entry.paused)
        .maybe_text(Column::Note, entry.note.clone())
        .list(Column::Tags, entry.tags.iter().map(ToString::to_string))
        .maybe_text(Column::DeterminedBy, entry.determined_by.map(decided_name))
        .flag(Column::Billable, entry.billable)
        .maybe_text(
            Column::Invoice,
            entry.invoice.as_ref().map(ToString::to_string),
        )
        .flag(Column::Running, entry.running);
    if let Some(end) = &entry.end {
        row = row.time(Column::End, end);
    }
    if let Some(price) = &entry.price {
        row = row.rate(&price.rate).money(&price.amount);
    }
    row
}

/// Who determined the time, as the log writes it: `self` or `other`.
const fn decided_name(decided: DeterminedBy) -> &'static str {
    match decided {
        DeterminedBy::Me => "self",
        DeterminedBy::Others => "other",
    }
}
