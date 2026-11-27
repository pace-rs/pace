//! A written invoice shown again: the bill of a client's entries billed on
//! one invoice reference, from the amounts their `billing.marked` events
//! froze.

use std::collections::{BTreeMap, BTreeSet};

use jiff::{ZonedDateTime, civil::Date, tz::TimeZone};
use pace_error::{ErrorKind, PaceError, PaceResult, closest_name};

use crate::{
    Bill, BillEntry, BillLine, Billing, Catalog, Client, ClientKey, Entry, InvoiceRef, ProjectRef,
    RoundingPer, State, Totals,
    bill::{BillParts, HoldTimeShare},
    report::canonical_project,
};

/// The error code of an invoice reference that no billed entry carries,
/// or that names invoices of several clients.
pub const UNKNOWN_INVOICE: &str = "pace::core::unknown-invoice";

/// What [`Bill::of_invoice`] takes.
#[derive(Debug, Clone, Copy)]
pub struct InvoiceInput<'input> {
    /// The state of every event.
    pub state: &'input State,
    /// The roles, clients and project records.
    pub catalog: &'input Catalog,
    /// The client of the invoice, from [`invoice_client`].
    pub client: &'input Client,
    /// The invoice's reference.
    pub invoice: &'input InvoiceRef,
    /// The report's zone, whose midnights give each entry its start day.
    pub zone: &'input TimeZone,
    /// Now.
    pub now: &'input ZonedDateTime,
}

impl Bill {
    /// The invoice `input.invoice` of `input.client` as it was written:
    /// the client's entries billed on it, each with the net time, billed
    /// time, rate and amount its `billing.marked` event froze. A later
    /// change of a rate, a rounding or the entry itself changes none of
    /// them.
    ///
    /// A client that rounds per day has one line per project and day, as
    /// its bill had; any other client has one line per entry. The share
    /// of hold time is that of the entries as they are now. An entry that
    /// was forgotten, or whose `entry.begun` was undone, has left the
    /// invoice.
    ///
    /// # Errors
    ///
    /// If an amount is too big to count in minor units.
    pub fn of_invoice(input: &InvoiceInput<'_>) -> PaceResult<Self> {
        let InvoiceInput {
            state,
            catalog,
            client,
            zone,
            now,
            ..
        } = *input;
        let billed = billed_on(state, catalog, input.invoice, &client.key);
        let per_day = client.rounding.per() == Some(RoundingPer::Day);
        let mut lines: Vec<(ProjectRef, BillLine)> = Vec::new();
        let mut line_of_day: BTreeMap<(ProjectRef, Date), usize> = BTreeMap::new();
        for (entry, billing) in &billed {
            let Some(project) = entry.project.as_ref() else {
                continue;
            };
            let project = canonical_project(project, catalog);
            let date = entry.start.with_time_zone(zone.clone()).date();
            let day = (project.clone(), date);
            match line_of_day
                .get(&day)
                .and_then(|&index| lines.get_mut(index))
            {
                Some((_, line)) if per_day && line.rate == billing.rate => {
                    line.add_billed(entry, billing)?;
                }
                _ => {
                    _ = line_of_day.insert(day, lines.len());
                    lines.push((project, frozen_line(date, entry, billing)));
                }
            }
        }
        let entries: Vec<&Entry> = billed.iter().map(|(entry, _)| *entry).collect();
        Self::of_lines(BillParts {
            client,
            catalog,
            lines,
            hold_share: HoldTimeShare::of(&entries, now),
            running: Vec::new(),
            invoice: Some(input.invoice.clone()),
        })
    }
}

impl BillLine {
    /// Add `entry` with its frozen `billing` to the line.
    fn add_billed(&mut self, entry: &Entry, billing: &Billing) -> PaceResult<()> {
        let mut amount = Totals::default();
        amount.add(self.amount)?;
        amount.add(billing.amount)?;
        self.amount = amount.get(self.amount.currency());
        self.net += billing.net;
        self.billed += billing.billed;
        self.entries.push(billed_entry(entry, billing));
        Ok(())
    }
}

/// The line of `entry` alone, from its frozen `billing`.
fn frozen_line(date: Date, entry: &Entry, billing: &Billing) -> BillLine {
    BillLine {
        date,
        entries: vec![billed_entry(entry, billing)],
        net: billing.net,
        billed: billing.billed,
        rate: billing.rate,
        amount: billing.amount,
    }
}

fn billed_entry(entry: &Entry, billing: &Billing) -> BillEntry {
    BillEntry {
        id: entry.id,
        note: entry.note.clone(),
        net: billing.net,
    }
}

/// The entries of `client` billed on `invoice`, with their billing, by
/// start.
fn billed_on<'state>(
    state: &'state State,
    catalog: &Catalog,
    invoice: &InvoiceRef,
    client: &ClientKey,
) -> Vec<(&'state Entry, &'state Billing)> {
    let mut billed: Vec<(&Entry, &Billing)> = state
        .billings()
        .filter(|(_, billing)| billing.invoice == *invoice)
        .filter_map(|(id, billing)| Some((state.entry(id)?, billing)))
        .filter(|(entry, _)| client_of(entry, catalog) == Some(client))
        .collect();
    billed.sort_by(|(first, _), (second, _)| {
        first
            .start
            .cmp(&second.start)
            .then(first.id.cmp(&second.id))
    });
    billed
}

/// The client of the entry's project, by its record.
fn client_of<'catalog>(entry: &Entry, catalog: &'catalog Catalog) -> Option<&'catalog ClientKey> {
    entry
        .project
        .as_ref()
        .and_then(|project| catalog.project_client(project))
}

/// The client whose entries carry the invoice reference `invoice`.
///
/// With `client`, that client, which must carry it. So the reference
/// alone finds an invoice, and `--client` tells two clients' invoices of
/// one reference apart.
///
/// # Errors
///
/// [`UNKNOWN_INVOICE`]:
///
/// - If no billed entry of a known client (of `client`, if given) carries
///   the reference: the hint names the closest reference, or the known
///   ones.
/// - If billed entries of several clients carry it: the error names
///   them, and the hint names `--client`.
pub fn invoice_client<'catalog>(
    state: &State,
    catalog: &'catalog Catalog,
    invoice: &InvoiceRef,
    client: Option<&ClientKey>,
) -> PaceResult<&'catalog Client> {
    let mut invoices: BTreeMap<&InvoiceRef, BTreeSet<&ClientKey>> = BTreeMap::new();
    for (id, billing) in state.billings() {
        let Some(key) = state.entry(id).and_then(|entry| client_of(entry, catalog)) else {
            continue;
        };
        if client.is_none_or(|client| client == key) {
            _ = invoices.entry(&billing.invoice).or_default().insert(key);
        }
    }
    let clients: Vec<&Client> = invoices
        .get(invoice)
        .into_iter()
        .flatten()
        .filter_map(|key| catalog.client(key))
        .collect();
    match clients.as_slice() {
        [only] => Ok(only),
        [] => Err(unknown_invoice(invoice, invoices.keys().copied())),
        several => {
            let keys: Vec<&str> = several.iter().map(|client| client.key.as_str()).collect();
            Err(PaceError::new(
                ErrorKind::InvalidInput,
                "the invoice `{invoice}` is of {count} clients: {keys}",
            )
            .attach_context("invoice", invoice.to_string())
            .attach_context("count", keys.len().to_string())
            .attach_context("keys", keys.join(", "))
            .attach_hint("name one with `--client <key>`")
            .attach_error_code(UNKNOWN_INVOICE))
        }
    }
}

/// The error of a reference no billed entry carries: its hint names the
/// closest of the `known` references, else all of them.
fn unknown_invoice<'known>(
    invoice: &InvoiceRef,
    known: impl Iterator<Item = &'known InvoiceRef> + Clone,
) -> Box<PaceError> {
    let names = known.clone().map(InvoiceRef::as_str);
    let hint = closest_name(invoice.as_str(), names).map_or_else(
        || {
            let all: Vec<String> = known.map(|known| format!("`{known}`")).collect();
            if all.is_empty() {
                "no invoice is written yet; `pace bill --mark-billed <invoice>` writes one"
                    .to_owned()
            } else {
                format!("the written invoices are {}", all.join(", "))
            }
        },
        |closest| format!("did you mean `{closest}`?"),
    );
    PaceError::new(
        ErrorKind::InvalidInput,
        "pace doesn't know the invoice `{invoice}`",
    )
    .attach_context("invoice", invoice.to_string())
    .attach_hint(hint)
    .attach_error_code(UNKNOWN_INVOICE)
}
