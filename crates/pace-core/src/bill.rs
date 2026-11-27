//! The bill of a client, decided from values: its ended, billable entries
//! of a period that are not billed yet, priced line by line, or a refusal
//! that names the entries and the fix.
//!
//! The app reads the period's events, builds their [`State`] and hands it
//! to [`Bill::build`] with the [`Catalog`], the client, the period, the
//! report's zone and now. The amounts come from [`billable_charges`], as
//! the revenue of `reflect` does, so a bill and a report agree.

use std::collections::{BTreeMap, BTreeSet};

use jiff::{SignedDuration, ZonedDateTime, civil::Date, tz::TimeZone};
use pace_error::{ErrorKind, PaceError, PaceResult};
use pace_time::TimeRange;
use serde::Serialize;

use crate::{
    Catalog, Charge, Client, ClientKey, Entry, EntryId, Money, ProjectRef, State, Totals,
    billable_charges,
};

/// The error code of a bill refused for entries that overlap.
pub const BILL_OVERLAP: &str = "pace::core::bill-overlap";

/// The error code of a bill refused for an entry before the client's
/// first rate.
pub const BILL_BEFORE_FIRST_RATE: &str = "pace::core::bill-before-first-rate";

/// What [`Bill::build`] takes.
#[derive(Debug, Clone, Copy)]
pub struct BillInput<'input> {
    /// The state of the period's events.
    pub state: &'input State,
    /// The roles, clients and project records.
    pub catalog: &'input Catalog,
    /// The client to bill, from `catalog`.
    pub client: &'input Client,
    /// The period: an entry belongs to it if it starts in it.
    pub period: &'input TimeRange,
    /// The report's zone, whose midnights give each entry its start day.
    pub zone: &'input TimeZone,
    /// Now.
    pub now: &'input ZonedDateTime,
}

/// One entry of a bill's line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BillEntry {
    /// The entry.
    pub id: EntryId,
    /// Its note.
    pub note: Option<String>,
    /// Its net time.
    pub net: SignedDuration,
}

/// A line of a bill: one entry, or with a client that rounds per day,
/// the entries of one project on one day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BillLine {
    /// The start day of its entries, in the report's zone.
    pub date: Date,
    /// Its entries, the earliest first.
    pub entries: Vec<BillEntry>,
    /// The net time of its entries.
    pub net: SignedDuration,
    /// `net` rounded by the client's rule.
    pub billed: SignedDuration,
    /// The hourly rate valid on `date`.
    pub rate: Money,
    /// The billed time at that rate.
    pub amount: Money,
}

/// The lines of one project, and its totals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectBill {
    /// The project, by the ID of its record.
    pub project: ProjectRef,
    /// Its lines, the earliest first.
    pub lines: Vec<BillLine>,
    /// The net time of its lines.
    pub net: SignedDuration,
    /// The billed time of its lines.
    pub billed: SignedDuration,
    /// The amount of its lines, in the client's currency.
    pub amount: Money,
}

/// The share of hold time: how much of the billed entries' time went to
/// interruptions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct HoldTimeShare {
    /// The hold time of the billed entries.
    pub hold: SignedDuration,
    /// Their gross time: end minus start.
    pub gross: SignedDuration,
    /// `hold` over `gross` in whole percent, rounded half up; 0 without
    /// gross time.
    pub percent: u32,
}

/// The bill of a client over a period: plain data that a table, a
/// template or JSON shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Bill {
    /// The client.
    pub client: ClientKey,
    /// Per project, by the key of its record.
    pub projects: Vec<ProjectBill>,
    /// The net time of all lines.
    pub net: SignedDuration,
    /// The billed time of all lines.
    pub billed: SignedDuration,
    /// The amounts per currency, by code; never across currencies.
    pub totals: Vec<Money>,
    /// The share of hold time of the billed entries.
    pub hold_share: HoldTimeShare,
    /// The running entries the bill leaves out: end them first to bill
    /// them. By start.
    pub running: Vec<EntryId>,
}

impl Bill {
    /// The bill of `input.client` over `input.period`.
    ///
    /// It takes the entries of the client's projects that start in the
    /// period, whose effective billable is yes
    /// ([`Entry::effective_billable`]) and that have no billing yet
    /// ([`State::billing`]). Each counts whole, on its start day in the
    /// report's zone. A running entry is left out and named in
    /// [`Bill::running`].
    ///
    /// # Errors
    ///
    /// - [`BILL_OVERLAP`] if one of its entries overlaps another entry of
    ///   the state, unless one of the two or its project's record allows
    ///   it. The other entry may be running, billed already, of another
    ///   client or not billable.
    /// - [`BILL_BEFORE_FIRST_RATE`] if no rate holds on the start day of an
    ///   entry.
    /// - If an amount is too big to count in minor units.
    pub fn build(input: &BillInput<'_>) -> PaceResult<Self> {
        let BillInput {
            state,
            catalog,
            client,
            zone,
            now,
            ..
        } = *input;
        let (running, ended): (Vec<&Entry>, Vec<&Entry>) = state
            .entries()
            .filter(|entry| to_bill(entry, input))
            .partition(|entry| entry.is_open());
        refuse_overlaps(input, &ended)?;
        let charges = billable_charges(&ended, catalog, zone, now)?;
        refuse_unpriced(state, client, &charges)?;

        let notes: BTreeMap<EntryId, &Entry> =
            ended.iter().map(|entry| (entry.id, *entry)).collect();
        let mut projects: BTreeMap<String, ProjectBill> = BTreeMap::new();
        let mut totals = Totals::default();
        for charge in charges {
            let Some(line) = bill_line(&charge, &notes, now) else {
                continue;
            };
            let Some(project) = charge.project else {
                continue;
            };
            totals.add(line.amount)?;
            let key = catalog
                .resolve(&project)
                .map_or_else(String::new, |record| record.key.to_string());
            let project_bill = projects.entry(key).or_insert_with(|| ProjectBill {
                project,
                lines: Vec::new(),
                net: SignedDuration::ZERO,
                billed: SignedDuration::ZERO,
                amount: Money::zero(client.currency),
            });
            project_bill.add(line)?;
        }
        let projects: Vec<ProjectBill> = projects.into_values().collect();
        Ok(Self {
            client: client.key.clone(),
            net: projects.iter().map(|project| project.net).sum(),
            billed: projects.iter().map(|project| project.billed).sum(),
            projects,
            totals: totals.iter().collect(),
            hold_share: HoldTimeShare::of(&ended, now),
            running: running.iter().map(|entry| entry.id).collect(),
        })
    }

    /// Whether the bill has no line: nothing to bill.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.projects.is_empty()
    }
}

impl ProjectBill {
    /// Add `line`; [`Totals`] refuses an amount too big to count.
    fn add(&mut self, line: BillLine) -> PaceResult<()> {
        let mut amount = Totals::default();
        amount.add(self.amount)?;
        amount.add(line.amount)?;
        self.amount = amount.get(self.amount.currency());
        self.net += line.net;
        self.billed += line.billed;
        self.lines.push(line);
        Ok(())
    }
}

impl HoldTimeShare {
    /// The share of hold time of `entries`, which have ended.
    fn of(entries: &[&Entry], now: &ZonedDateTime) -> Self {
        let hold: SignedDuration = entries.iter().map(|entry| entry.hold_time(now)).sum();
        let gross: SignedDuration = entries.iter().map(|entry| entry.gross(now)).sum();
        let (hold_secs, gross_secs) = (i128::from(hold.as_secs()), i128::from(gross.as_secs()));
        let percent = if gross_secs > 0 {
            (hold_secs * 100 + gross_secs / 2) / gross_secs
        } else {
            0
        };
        Self {
            hold,
            gross,
            percent: u32::try_from(percent).unwrap_or(u32::MAX),
        }
    }
}

/// Whether `entry` goes on the bill of `input`: it starts in the period,
/// belongs to a project of the client, is billable and not billed yet.
fn to_bill(entry: &Entry, input: &BillInput<'_>) -> bool {
    let period = input.period;
    period.start().is_none_or(|start| entry.start >= *start)
        && period.end().is_none_or(|end| entry.start < *end)
        && input.state.billing(entry.id).is_none()
        && entry.effective_billable(input.catalog)
        && entry
            .project
            .as_ref()
            .and_then(|project| input.catalog.resolve(project))
            .and_then(|record| record.client.as_ref())
            .is_some_and(|client| *client == input.client.key)
}

/// Whether `entry` may overlap others: its own mark, or its project's
/// record.
fn allows_overlap(entry: &Entry, catalog: &Catalog) -> bool {
    entry.allow_overlap
        || entry
            .project
            .as_ref()
            .is_some_and(|project| catalog.project_allows_overlap(project))
}

/// Refuse the bill if one of `entries` overlaps any entry of the state,
/// and neither of the two allows it: pace bills no time twice. The other
/// entry may be running, billed already, of another client or project,
/// not billable or outside the period.
fn refuse_overlaps(input: &BillInput<'_>, entries: &[&Entry]) -> PaceResult<()> {
    let on_bill: BTreeSet<EntryId> = entries.iter().map(|entry| entry.id).collect();
    let state = input.state;
    let pairs: Vec<String> = state
        .overlaps(input.now)
        .into_iter()
        .filter(|overlap| {
            on_bill.contains(&overlap.first.id) || on_bill.contains(&overlap.second.id)
        })
        .filter(|overlap| {
            !allows_overlap(overlap.first, input.catalog)
                && !allows_overlap(overlap.second, input.catalog)
        })
        .map(|overlap| {
            format!(
                "{} and {} share {}",
                state.short_id(overlap.first.id),
                state.short_id(overlap.second.id),
                shared_time(&overlap.start, &overlap.end, input.zone)
            )
        })
        .collect();
    if pairs.is_empty() {
        return Ok(());
    }
    Err(PaceError::new(
        ErrorKind::InvalidInput,
        "pace bills no time twice, but these entries overlap: {pairs}",
    )
    .attach_context("pairs", pairs.join("; "))
    .attach_hint(
        "move one entry of each pair with `pace adjust <id> --start` or `--end`, or allow the \
         overlap with `pace adjust <id> --allow-overlap`",
    )
    .attach_error_code(BILL_OVERLAP))
}

/// The time two entries share, in `zone`: `09:30–10:00 on 2026-11-20`,
/// with both dates if it runs over midnight.
fn shared_time(start: &ZonedDateTime, end: &ZonedDateTime, zone: &TimeZone) -> String {
    let (start, end) = (
        start.with_time_zone(zone.clone()),
        end.with_time_zone(zone.clone()),
    );
    if start.date() == end.date() {
        format!(
            "{}–{} on {}",
            start.strftime("%H:%M"),
            end.strftime("%H:%M"),
            start.date()
        )
    } else {
        format!(
            "{}–{}",
            start.strftime("%Y-%m-%d %H:%M"),
            end.strftime("%Y-%m-%d %H:%M")
        )
    }
}

/// Refuse the bill if a charge has no rate: pace never guesses one.
fn refuse_unpriced(state: &State, client: &Client, charges: &[Charge]) -> PaceResult<()> {
    let unpriced: Vec<String> = charges
        .iter()
        .filter(|charge| charge.price.is_none())
        .flat_map(|charge| {
            charge
                .entries
                .iter()
                .map(|entry| format!("{} ({})", state.short_id(*entry), charge.date))
        })
        .collect();
    if unpriced.is_empty() {
        return Ok(());
    }
    Err(PaceError::new(
        ErrorKind::InvalidInput,
        "no rate of the client {client} holds on the start day of {entries}",
    )
    .attach_context("client", client.key.to_string())
    .attach_context("entries", unpriced.join(", "))
    .attach_hint("add an earlier rate to clients.toml")
    .attach_error_code(BILL_BEFORE_FIRST_RATE))
}

/// The line of a priced `charge`, with the note and the net time of each
/// of its entries.
fn bill_line(
    charge: &Charge,
    entries: &BTreeMap<EntryId, &Entry>,
    now: &ZonedDateTime,
) -> Option<BillLine> {
    let price = charge.price?;
    Some(BillLine {
        date: charge.date,
        entries: charge
            .entries
            .iter()
            .filter_map(|id| entries.get(id))
            .map(|entry| BillEntry {
                id: entry.id,
                note: entry.note.clone(),
                net: entry.net(now),
            })
            .collect(),
        net: charge.net,
        billed: charge.billed,
        rate: price.rate.hourly,
        amount: price.amount,
    })
}
