//! The bill of a client, decided from values: its ended, billable entries
//! of a period that are not billed yet, priced line by line, or a refusal
//! that names the entries and the fix.
//!
//! The app reads the period's events, builds their [`State`] and hands it
//! to [`Bill::build`] with the [`Catalog`], the client, the period, the
//! report's zone and now. The amounts come from [`billable_charges`], as
//! the revenue of `reflect` does, so a bill and a report agree.
//!
//! A written invoice comes back from the amounts its `billing.marked`
//! events froze: [`invoice_client`] finds the client of a reference, and
//! [`Bill::of_invoice`] builds its bill (`bill/invoice.rs`).

use std::collections::{BTreeMap, BTreeSet};

use jiff::{SignedDuration, ZonedDateTime, civil::Date, tz::TimeZone};
use pace_error::{ErrorKind, PaceError, PaceResult};
use pace_time::TimeRange;
use serde::Serialize;

use crate::{
    Billing, Catalog, Charge, Client, ClientKey, Entry, EntryId, InvoiceRef, Money, ProjectRef,
    State, Totals, billable_charges,
};

mod invoice;

pub use invoice::{InvoiceInput, UNKNOWN_INVOICE, invoice_client};

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

/// The bill of a client over a period, or a written invoice shown again
/// ([`Bill::of_invoice`]): plain data that a table, a template or JSON
/// shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Bill {
    /// The client.
    pub client: ClientKey,
    /// The invoice the bill was written on; `None` for a bill that is not
    /// marked yet.
    pub invoice: Option<InvoiceRef>,
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

/// What [`Bill::of_lines`] puts together.
struct BillParts<'parts> {
    client: &'parts Client,
    catalog: &'parts Catalog,
    /// Each line with its project.
    lines: Vec<(ProjectRef, BillLine)>,
    hold_share: HoldTimeShare,
    running: Vec<EntryId>,
    invoice: Option<InvoiceRef>,
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
        let lines = charges
            .iter()
            .filter_map(|charge| Some((charge.project.clone()?, bill_line(charge, &notes, now)?)))
            .collect();
        Self::of_lines(BillParts {
            client,
            catalog,
            lines,
            hold_share: HoldTimeShare::of(&ended, now),
            running: running.iter().map(|entry| entry.id).collect(),
            invoice: None,
        })
    }

    /// The bill of `parts`: its lines per project, by the key of the
    /// project's record, with the totals of each project and of the bill.
    fn of_lines(parts: BillParts<'_>) -> PaceResult<Self> {
        let BillParts {
            client, catalog, ..
        } = parts;
        let mut projects: BTreeMap<String, ProjectBill> = BTreeMap::new();
        let mut totals = Totals::default();
        for (project, line) in parts.lines {
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
            invoice: parts.invoice,
            net: projects.iter().map(|project| project.net).sum(),
            billed: projects.iter().map(|project| project.billed).sum(),
            projects,
            totals: totals.iter().collect(),
            hold_share: parts.hold_share,
            running: parts.running,
        })
    }

    /// Whether the bill has no line: nothing to bill.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.projects.is_empty()
    }

    /// The billing of each entry of the bill on `invoice`, as
    /// `billing.marked` freezes it, line by line: see
    /// [`BillLine::billings`].
    #[must_use]
    pub fn billings(&self, invoice: &InvoiceRef) -> Vec<(EntryId, Billing)> {
        self.projects
            .iter()
            .flat_map(|project| &project.lines)
            .flat_map(|line| line.billings(invoice))
            .collect()
    }
}

impl BillLine {
    /// The billing of each of its entries on `invoice`: the entry's own
    /// net time, the line's rate, and a share of the line's billed time
    /// and amount by net time. The shares add up to the line to the
    /// second and to the minor unit, so the marked entries sum to the
    /// bill; a remainder goes to the entries with the largest fractions,
    /// the earlier first.
    #[must_use]
    pub fn billings(&self, invoice: &InvoiceRef) -> Vec<(EntryId, Billing)> {
        let weights: Vec<i64> = self
            .entries
            .iter()
            .map(|entry| entry.net.as_secs())
            .collect();
        let billed = shares_by_weight(self.billed.as_secs(), &weights);
        let amounts = shares_by_weight(self.amount.minor(), &weights);
        self.entries
            .iter()
            .zip(billed.into_iter().zip(amounts))
            .map(|(entry, (billed, amount))| {
                (
                    entry.id,
                    Billing {
                        invoice: invoice.clone(),
                        net: entry.net,
                        billed: SignedDuration::from_secs(billed),
                        rate: self.rate,
                        amount: Money::from_minor(amount, self.amount.currency()),
                    },
                )
            })
            .collect()
    }
}

/// `total` split by `weights`, in whole units, by the largest remainder:
/// each share is the floor of its part, and the units left over go one
/// each to the largest fractions, the earlier first. The shares add up
/// to `total`. Without any positive weight, the first share takes it all.
fn shares_by_weight(total: i64, weights: &[i64]) -> Vec<i64> {
    let weights: Vec<i128> = weights
        .iter()
        .map(|weight| i128::from(*weight).max(0))
        .collect();
    let sum: i128 = weights.iter().sum();
    if sum == 0 {
        return (0..weights.len())
            .map(|index| if index == 0 { total } else { 0 })
            .collect();
    }
    let total = i128::from(total);
    let mut shares: Vec<i128> = weights
        .iter()
        .map(|weight| (total * weight).div_euclid(sum))
        .collect();
    let mut by_fraction: Vec<usize> = (0..weights.len()).collect();
    by_fraction.sort_by_key(|index| {
        std::cmp::Reverse(
            weights
                .get(*index)
                .map_or(0, |weight| (total * weight).rem_euclid(sum)),
        )
    });
    let left_over = total - shares.iter().sum::<i128>();
    for index in by_fraction
        .into_iter()
        .take(usize::try_from(left_over).unwrap_or(0))
    {
        if let Some(share) = shares.get_mut(index) {
            *share += 1;
        }
    }
    shares
        .into_iter()
        .map(|share| i64::try_from(share).unwrap_or(i64::MAX))
        .collect()
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
            .and_then(|project| input.catalog.project_client(project))
            .is_some_and(|client| *client == input.client.key)
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
            !overlap.first.allows_overlap(input.catalog)
                && !overlap.second.allows_overlap(input.catalog)
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
pub(crate) fn shared_time(start: &ZonedDateTime, end: &ZonedDateTime, zone: &TimeZone) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shares_add_up_with_remainder_to_largest_fractions() {
        assert_eq!(shares_by_weight(2500, &[240, 480]), vec![833, 1667]);
        assert_eq!(shares_by_weight(2500, &[1, 1, 1]), vec![834, 833, 833]);
    }

    #[test]
    fn shares_without_positive_weight_go_to_first() {
        assert_eq!(shares_by_weight(900, &[0, 0]), vec![900, 0]);
        assert_eq!(shares_by_weight(900, &[]), Vec::<i64>::new());
    }

    #[test]
    fn negative_weight_counts_as_none() {
        assert_eq!(shares_by_weight(900, &[-60, 60]), vec![0, 900]);
    }
}
