//! The rule of `bill --mark-billed`: the bill of a client, decided on the
//! state as it is, frozen as one `billing.marked` per entry.

use pace_error::{ErrorKind, PaceError, PaceResult};
use pace_time::TimeRange;

use crate::{
    Bill, BillInput, Catalog, Client, Event, EventKind, InvoiceRef, State,
    command::MarkBilled,
    rules::{Rules, nothing_to_do},
};

/// The error code of a marking refused because the client's entries
/// carry its invoice reference already.
pub const INVOICE_USED: &str = "pace::core::invoice-used";

impl Rules<'_> {
    /// One `billing.marked` per entry of the bill that [`Bill::build`]
    /// decides on the current state, filed under the entry, so it lies
    /// with the entry's events.
    ///
    /// # Errors
    ///
    /// - The bill's refusals: an overlap, an entry before the first rate.
    /// - Nothing to do if the bill has no line: every entry of the period
    ///   is billed already, running or not billable.
    /// - If the client's entries carry the invoice reference already.
    /// - If the client is unknown, or the period ends before it starts.
    pub(super) fn mark_billed(&self, mark: &MarkBilled) -> PaceResult<Vec<Event>> {
        let client = self.catalog.client(&mark.client).ok_or_else(|| {
            PaceError::new(ErrorKind::InvalidInput, "pace knows no client {client}")
                .attach_context("client", mark.client.to_string())
                .attach_hint("`pace client list` shows the clients")
        })?;
        let period = TimeRange::new(mark.start.clone(), mark.end.clone())?;
        let bill = Bill::build(&BillInput {
            state: self.state,
            catalog: self.catalog,
            client,
            period: &period,
            zone: &self.policy.reports_time_zone,
            now: self.now,
        })?;
        if bill.is_empty() {
            return Err(nothing_to_do(
                "nothing to bill for {client} in the period",
                "a bill takes the ended, billable entries that are not billed yet",
            )
            .attach_context("client", client.key.to_string()));
        }
        refuse_used_invoice(self.state, self.catalog, client, &mark.invoice)?;
        bill.billings(&mark.invoice)
            .into_iter()
            .map(|(entry, billing)| {
                let filed = &self.known_entry(entry)?.filed;
                self.event(self.now, filed, EventKind::BillingMarked { entry, billing })
            })
            .collect()
    }
}

/// Refuse `invoice` if an entry of `client` carries it already: one
/// reference names one invoice of a client.
fn refuse_used_invoice(
    state: &State,
    catalog: &Catalog,
    client: &Client,
    invoice: &InvoiceRef,
) -> PaceResult<()> {
    let carrying = state
        .billings()
        .filter(|(_, billing)| billing.invoice == *invoice)
        .filter_map(|(entry, _)| state.entry(entry))
        .filter(|entry| {
            entry
                .project
                .as_ref()
                .and_then(|project| catalog.project_client(project))
                .is_some_and(|key| *key == client.key)
        })
        .count();
    if carrying == 0 {
        return Ok(());
    }
    let entries = if carrying == 1 {
        "1 entry".to_owned()
    } else {
        format!("{carrying} entries")
    };
    Err(PaceError::new(
        ErrorKind::InvalidInput,
        "{client}'s invoice {invoice} is marked already, on {entries}",
    )
    .attach_context("client", client.key.to_string())
    .attach_context("invoice", invoice.to_string())
    .attach_context("entries", entries)
    .attach_hint("give the new invoice a reference of its own")
    .attach_error_code(INVOICE_USED))
}
