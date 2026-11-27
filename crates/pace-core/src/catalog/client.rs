//! Clients: who gets billed, in which currency, at which dated rates, and
//! how their billable time rounds.

use std::collections::BTreeMap;

use jiff::civil::Date;
use pace_error::{ErrorKind, PaceError, PaceResult};

use crate::{ClientKey, Currency, Money, ProjectKey, Rounding};

/// An hourly rate, valid from its day until the next rate's `from`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rate {
    /// The amount per hour.
    pub hourly: Money,
    /// The first day it holds.
    pub from: Date,
}

/// The dated rates of a client or of one of its projects, sorted by their
/// `from` day. A rate holds until the next one begins, so after the first
/// rate the list has no gap.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rates(Vec<Rate>);

impl Rates {
    /// The rates, sorted by their `from` day.
    ///
    /// # Errors
    ///
    /// If two rates begin on the same day: pace can't tell which one holds.
    pub fn new(mut rates: Vec<Rate>) -> PaceResult<Self> {
        rates.sort_by_key(|rate| rate.from);
        if let Some(pair) = rates.windows(2).find(|pair| pair[0].from == pair[1].from) {
            return Err(PaceError::new(
                ErrorKind::InvalidInput,
                "two rates begin on {from}: {first} and {second}",
            )
            .attach_context("from", pair[0].from.to_string())
            .attach_context("first", pair[0].hourly.to_string())
            .attach_context("second", pair[1].hourly.to_string())
            .attach_hint("give each rate of a list its own `from` day")
            .attach_error_code("pace::core::rate-day"));
        }
        Ok(Self(rates))
    }

    /// The rate that holds on `day`: the last one that begins on it or
    /// before. `None` before the first rate.
    #[must_use]
    pub fn on(&self, day: Date) -> Option<&Rate> {
        self.0.iter().rev().find(|rate| rate.from <= day)
    }

    /// The rates, the earliest first.
    pub fn iter(&self) -> impl Iterator<Item = &Rate> {
        self.0.iter()
    }
}

/// A client: who gets billed for the time of its projects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Client {
    /// What people type: `acme`.
    pub key: ClientKey,
    /// What a bill shows: `ACME GmbH`.
    pub name: String,
    /// The currency of its rates and bills.
    pub currency: Currency,
    /// How its billable time rounds; [`Rounding::NONE`] by default.
    pub rounding: Rounding,
    /// Its hourly rates.
    pub rates: Rates,
    /// The rates of some of its projects, which come before its own.
    pub project_rates: BTreeMap<ProjectKey, Rates>,
}

impl Client {
    /// The rate for `project` on `day`: the project's rates first, then the
    /// client's. The first list that has a rate for that day wins.
    #[must_use]
    pub fn rate_on(&self, project: &ProjectKey, day: Date) -> Option<&Rate> {
        self.project_rates
            .get(project)
            .and_then(|rates| rates.on(day))
            .or_else(|| self.rates.on(day))
    }

    /// Every rate of the client and of its projects.
    pub(crate) fn all_rates(&self) -> impl Iterator<Item = &Rate> {
        self.rates
            .iter()
            .chain(self.project_rates.values().flat_map(Rates::iter))
    }
}
