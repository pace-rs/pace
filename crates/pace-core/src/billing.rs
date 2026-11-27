//! Billing: the billable time of entries, rounded by their client's rule
//! and priced at the rate of each entry's start day; the one place that
//! `reflect`'s revenue and `bill` take their amounts from.
//!
//! Each entry counts whole, on its start day in the report's zone, so an
//! entry over midnight takes the rate of the day it began (restart.md 3.7).

use std::collections::BTreeMap;

use jiff::{SignedDuration, ZonedDateTime, civil::Date, tz::TimeZone};
use pace_error::PaceResult;
use serde::Serialize;

use crate::{
    Catalog, Client, ClientKey, Entry, EntryId, Money, ProjectRecord, ProjectRef, Rate, Rounding,
    RoundingPer, report::canonical_project,
};

/// The rate a charge takes and the amount it comes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Price {
    /// The rate valid on the charge's day.
    pub rate: Rate,
    /// The billed time at that rate, rounded half up to the minor unit.
    pub amount: Money,
}

/// Billable time that rounds as one: an entry, or with a client that
/// rounds per day, the entries of one project on one day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Charge {
    /// The project: the ID of its record, else the key its entries name;
    /// `None` for billable entries without a project.
    pub project: Option<ProjectRef>,
    /// The client of the project's record.
    pub client: Option<ClientKey>,
    /// The start day of its entries, in the report's zone.
    pub date: Date,
    /// Its entries, the earliest first.
    pub entries: Vec<EntryId>,
    /// The net time of its entries.
    pub net: SignedDuration,
    /// `net` rounded by the client's rule; `net` without a client.
    pub billed: SignedDuration,
    /// The rate and the amount; `None` when no rate is valid on `date`,
    /// for pace never guesses a rate.
    pub price: Option<Price>,
    /// Whether a running entry counts in it, up to now.
    pub running: bool,
}

/// The charges for the billable entries among `entries`: those whose
/// effective billable ([`Entry::effective_billable`]) is yes, each with its
/// net time up to `now`, the earliest first.
///
/// A client's [`Rounding`] rounds each entry, or the time of each day and
/// project, as it says; without a client nothing rounds. Each charge takes
/// the rate valid on its day: the project's rates first, then the client's
/// ([`Catalog::rate_on`]).
///
/// # Errors
///
/// If an amount is too big to count in minor units.
pub fn billable_charges(
    entries: &[&Entry],
    catalog: &Catalog,
    zone: &TimeZone,
    now: &ZonedDateTime,
) -> PaceResult<Vec<Charge>> {
    let mut billable: Vec<&Entry> = entries
        .iter()
        .copied()
        .filter(|entry| entry.effective_billable(catalog))
        .collect();
    billable.sort_by(|first, second| first.start.cmp(&second.start));

    let mut pending: Vec<Pending<'_>> = Vec::new();
    let mut per_day: BTreeMap<(Option<ProjectRef>, Date), usize> = BTreeMap::new();
    for entry in billable {
        let next = Pending::of(entry, catalog, zone, now);
        let shared = (next.rounding().per() == Some(RoundingPer::Day))
            .then(|| (next.project.clone(), next.date));
        let index = shared.as_ref().and_then(|key| per_day.get(key)).copied();
        if let Some(charge) = index.and_then(|index| pending.get_mut(index)) {
            charge.join(next);
        } else {
            if let Some(key) = shared {
                _ = per_day.insert(key, pending.len());
            }
            pending.push(next);
        }
    }
    pending
        .into_iter()
        .map(|charge| charge.priced(catalog))
        .collect()
}

/// A charge before its rounding and its price.
struct Pending<'catalog> {
    project: Option<ProjectRef>,
    record: Option<&'catalog ProjectRecord>,
    client: Option<&'catalog Client>,
    date: Date,
    entries: Vec<EntryId>,
    net: SignedDuration,
    running: bool,
}

impl<'catalog> Pending<'catalog> {
    /// The charge of `entry` alone.
    fn of(entry: &Entry, catalog: &'catalog Catalog, zone: &TimeZone, now: &ZonedDateTime) -> Self {
        let record = entry
            .project
            .as_ref()
            .and_then(|project| catalog.resolve(project));
        Self {
            project: entry
                .project
                .as_ref()
                .map(|project| canonical_project(project, catalog)),
            record,
            client: record
                .and_then(|record| record.client.as_ref())
                .and_then(|client| catalog.client(client)),
            date: entry.start.with_time_zone(zone.clone()).date(),
            entries: vec![entry.id],
            net: entry.net(now),
            running: entry.is_open(),
        }
    }

    /// The client's rounding; none without a client.
    fn rounding(&self) -> Rounding {
        self.client.map_or(Rounding::NONE, |client| client.rounding)
    }

    /// Add the entries of `other`, of the same project and day.
    fn join(&mut self, other: Self) {
        self.entries.extend(other.entries);
        self.net += other.net;
        self.running |= other.running;
    }

    /// The charge, rounded and priced at the rate of its day.
    fn priced(self, catalog: &Catalog) -> PaceResult<Charge> {
        let billed = self.rounding().round(self.net);
        let price = self
            .record
            .and_then(|record| catalog.rate_on(record, self.date))
            .map(|rate| {
                Ok::<_, Box<pace_error::PaceError>>(Price {
                    rate: *rate,
                    amount: rate.hourly.amount_for(billed)?,
                })
            })
            .transpose()?;
        Ok(Charge {
            project: self.project,
            client: self.client.map(|client| client.key.clone()),
            date: self.date,
            entries: self.entries,
            net: self.net,
            billed,
            price,
            running: self.running,
        })
    }
}
