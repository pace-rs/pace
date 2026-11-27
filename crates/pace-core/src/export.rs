//! The export of `pace export`: one row per entry that starts in a period
//! and passes the report's filters, unsplit, as plain data the app
//! renders.
//!
//! A billed entry carries the rate and the amount its `billing.marked`
//! event froze. An unbilled billable entry carries the rate of its start
//! day in the report's zone, and the amount of its net time, rounded half
//! up to the minor unit. The client's step doesn't round it, as it rounds
//! a bill and the revenue of `pace reflect` (`billable_charges`). A running
//! entry counts up to now.

use std::collections::BTreeSet;

use jiff::{SignedDuration, ZonedDateTime, tz::TimeZone};
use pace_error::PaceResult;
use pace_time::TimeRange;
use serde::Serialize;

use crate::{
    Catalog, ClientKey, DeterminedBy, Entry, EntryId, InvoiceRef, Money, ProjectId, ProjectKey,
    ProjectRecord, ProjectRef, ReportFilter, RoleKey, State, Tag,
};

/// What [`export_entries`] takes.
#[derive(Debug, Clone, Copy)]
pub struct ExportInput<'input> {
    /// The state of the period's events, with every later billing of its
    /// entries; [`State::resolve_projects`] has moved its entries to their
    /// project records.
    pub state: &'input State,
    /// The roles, clients and project records.
    pub catalog: &'input Catalog,
    /// The period: an entry belongs to it if it starts in it.
    pub period: &'input TimeRange,
    /// Which entries the export takes.
    pub filter: &'input ReportFilter,
    /// The report's zone, whose midnights give each entry its start day.
    pub zone: &'input TimeZone,
    /// Now: a running entry counts up to it.
    pub now: &'input ZonedDateTime,
}

/// The rate of an exported entry and the amount it comes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ExportPrice {
    /// The hourly rate, in the currency of the amount.
    pub rate: Money,
    /// The amount: frozen for a billed entry, else the net time at the
    /// rate.
    pub amount: Money,
}

/// One entry of the export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportEntry {
    /// The entry.
    pub id: EntryId,
    /// Its short ID among the entries of the state.
    pub short: String,
    /// When it began, in the zone it was recorded in.
    pub start: ZonedDateTime,
    /// When it ended; `None` while it runs.
    pub end: Option<ZonedDateTime>,
    /// Whether it has no end yet; its times count up to now.
    pub running: bool,
    /// End minus start.
    pub gross: SignedDuration,
    /// Gross time minus holds and pauses.
    pub net: SignedDuration,
    /// The time in its holds.
    pub held: SignedDuration,
    /// The time in its pauses.
    pub paused: SignedDuration,
    /// The key of its project: the record's, else the one it names.
    pub project: Option<ProjectKey>,
    /// The ID of its project's record.
    pub project_id: Option<ProjectId>,
    /// The client of its project's record.
    pub client: Option<ClientKey>,
    /// The key of the role its time goes to ([`Entry::effective_role`]).
    pub role: Option<RoleKey>,
    /// Its tags.
    pub tags: BTreeSet<Tag>,
    /// Its note.
    pub note: Option<String>,
    /// Who determined its time; `None` while unmarked.
    pub determined_by: Option<DeterminedBy>,
    /// Whether its time is billable ([`Entry::effective_billable`]).
    pub billable: bool,
    /// The invoice it went on, once billed.
    pub invoice: Option<InvoiceRef>,
    /// Its rate and amount; `None` where no rate is valid, and for an
    /// entry that is not billable.
    pub price: Option<ExportPrice>,
}

/// The entries of `input.state` that start in `input.period` and pass
/// `input.filter`, the earliest first, each whole.
///
/// # Errors
///
/// If an amount is too big to count in minor units.
pub fn export_entries(input: &ExportInput<'_>) -> PaceResult<Vec<ExportEntry>> {
    input
        .state
        .entries()
        .filter(|entry| {
            input
                .period
                .start()
                .is_none_or(|start| entry.start >= *start)
        })
        .filter(|entry| input.period.end().is_none_or(|end| entry.start < *end))
        .filter(|entry| input.filter.matches(entry, input.catalog))
        .map(|entry| export_entry(entry, input))
        .collect()
}

/// The row of `entry`.
fn export_entry(entry: &Entry, input: &ExportInput<'_>) -> PaceResult<ExportEntry> {
    let ExportInput {
        state,
        catalog,
        now,
        ..
    } = *input;
    let record = entry
        .project
        .as_ref()
        .and_then(|project| catalog.resolve(project));
    let billing = state.billing(entry.id);
    let (project, project_id) = project_names(entry.project.as_ref(), record);
    Ok(ExportEntry {
        id: entry.id,
        short: state.short_id(entry.id),
        start: entry.start.clone(),
        end: entry.end.clone(),
        running: entry.is_open(),
        gross: entry.gross(now),
        net: entry.net(now),
        held: entry.hold_time(now),
        paused: entry.pause_time(now),
        project,
        project_id,
        client: record.and_then(|record| record.client.clone()),
        role: entry
            .effective_role(catalog)
            .and_then(|role| catalog.role_by_id(role))
            .map(|role| role.key.clone()),
        tags: entry.tags.clone(),
        note: entry.note.clone(),
        determined_by: entry.determined_by,
        billable: entry.effective_billable(catalog),
        invoice: billing.map(|billing| billing.invoice.clone()),
        price: match billing {
            Some(billing) => Some(ExportPrice {
                rate: billing.rate,
                amount: billing.amount,
            }),
            None => current_price(entry, input)?,
        },
    })
}

/// The key and the ID of `project`: the ones of its `record`, else what
/// the entry names.
fn project_names(
    project: Option<&ProjectRef>,
    record: Option<&ProjectRecord>,
) -> (Option<ProjectKey>, Option<ProjectId>) {
    match (record, project) {
        (Some(record), _) => (Some(record.key.clone()), Some(record.id)),
        (None, Some(ProjectRef::Key(key))) => (Some(key.clone()), None),
        (None, Some(ProjectRef::Id(id))) => (None, Some(*id)),
        (None, None) => (None, None),
    }
}

/// The price of an unbilled `entry`: the rate of its start day and its
/// net time at that rate; `None` if it is not billable or no rate holds.
fn current_price(entry: &Entry, input: &ExportInput<'_>) -> PaceResult<Option<ExportPrice>> {
    let catalog = input.catalog;
    if !entry.effective_billable(catalog) {
        return Ok(None);
    }
    let day = entry.start.with_time_zone(input.zone.clone()).date();
    let Some(rate) = entry
        .project
        .as_ref()
        .and_then(|project| catalog.resolve(project))
        .and_then(|record| catalog.rate_on(record, day))
    else {
        return Ok(None);
    };
    Ok(Some(ExportPrice {
        rate: rate.hourly,
        amount: rate.hourly.amount_for(entry.net(input.now))?,
    }))
}
