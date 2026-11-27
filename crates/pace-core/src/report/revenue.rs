//! The revenue of a report: the billable time and its amount per project
//! and per client, the totals per currency, and the time without a rate.

use std::collections::BTreeMap;

use jiff::SignedDuration;
use pace_error::PaceResult;
use serde::Serialize;

use crate::{
    Catalog, Charge, ClientKey, Currency, Money, ProjectRef, Totals, report::time::project_key,
};

/// The billable time of a project that has a rate, and its amount.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectRevenue {
    /// The project, by the ID of its record.
    pub project: ProjectRef,
    /// Its client, whose currency the amount is in.
    pub client: ClientKey,
    /// The billable time, rounded by the client's rule.
    pub billed: SignedDuration,
    /// The amount.
    pub amount: Money,
    /// Whether a running entry counts in it, up to now.
    pub running: bool,
}

/// The billable time of a client that has a rate, and its amount.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClientRevenue {
    /// The client.
    pub client: ClientKey,
    /// The billable time, rounded by its rule.
    pub billed: SignedDuration,
    /// The amount, in its currency.
    pub amount: Money,
    /// Whether a running entry counts in it, up to now.
    pub running: bool,
}

/// What the billable entries of a report earn, as `bill` would charge it:
/// each entry counts whole, on its start day, at the rate of that day.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Revenue {
    /// Per project, by client key, then by project key.
    pub projects: Vec<ProjectRevenue>,
    /// Per client, by key.
    pub clients: Vec<ClientRevenue>,
    /// The sums per currency, by code; never across currencies.
    pub totals: Vec<Money>,
    /// The billable time without a rate on its day, the earliest first:
    /// pace shows its time and never guesses a rate.
    pub no_rate: Vec<Charge>,
}

/// The billable time and the amount of one project or client.
#[derive(Debug)]
struct Earned {
    billed: SignedDuration,
    amount: Totals,
    currency: Currency,
    running: bool,
}

impl Earned {
    fn new(currency: Currency) -> Self {
        Self {
            billed: SignedDuration::ZERO,
            amount: Totals::default(),
            currency,
            running: false,
        }
    }

    fn add(&mut self, charge: &Charge, amount: Money) -> PaceResult<()> {
        self.billed += charge.billed;
        self.running |= charge.running;
        self.amount.add(amount)
    }

    fn amount(&self) -> Money {
        self.amount.get(self.currency)
    }
}

impl Revenue {
    /// Whether a row of the revenue counts a running entry up to now: a
    /// project, a client or a charge without a rate.
    #[must_use]
    pub fn marks_running(&self) -> bool {
        self.projects.iter().any(|project| project.running)
            || self.clients.iter().any(|client| client.running)
            || self.no_rate.iter().any(|charge| charge.running)
    }

    /// The revenue of `charges`, as [`billable_charges`](crate::billable_charges)
    /// gives them.
    ///
    /// # Errors
    ///
    /// If a sum grows too big to count in minor units.
    pub fn of(charges: Vec<Charge>, catalog: &Catalog) -> PaceResult<Self> {
        let mut projects: BTreeMap<(ClientKey, String), (ProjectRef, Earned)> = BTreeMap::new();
        let mut clients: BTreeMap<ClientKey, Earned> = BTreeMap::new();
        let mut totals = Totals::default();
        let mut no_rate = Vec::new();
        for charge in charges {
            let (Some(project), Some(client), Some(price)) =
                (charge.project.clone(), charge.client.clone(), charge.price)
            else {
                no_rate.push(charge);
                continue;
            };
            let amount = price.amount;
            totals.add(amount)?;
            let key = (client.clone(), project_key(&project, catalog));
            let currency = amount.currency();
            let (_, earned) = projects
                .entry(key)
                .or_insert_with(|| (project, Earned::new(currency)));
            earned.add(&charge, amount)?;
            clients
                .entry(client)
                .or_insert_with(|| Earned::new(currency))
                .add(&charge, amount)?;
        }
        Ok(Self {
            projects: projects
                .into_iter()
                .map(|((client, _), (project, earned))| ProjectRevenue {
                    project,
                    client,
                    billed: earned.billed,
                    amount: earned.amount(),
                    running: earned.running,
                })
                .collect(),
            clients: clients
                .into_iter()
                .map(|(client, earned)| ClientRevenue {
                    client,
                    billed: earned.billed,
                    amount: earned.amount(),
                    running: earned.running,
                })
                .collect(),
            totals: totals.iter().collect(),
            no_rate,
        })
    }
}
