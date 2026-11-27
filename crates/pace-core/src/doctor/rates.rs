//! The billing checks of `pace doctor`: billed entries whose amount would
//! differ with today's rates, and billable entries before the first rate
//! of their client.

use jiff::civil::Date;

use crate::{
    Billing, ClientKey, Entry, EntryId, Money, ProjectRecord,
    doctor::{DataArea, DataFinding, DoctorInput},
};

/// A warning for each billed entry whose amount today's rates would
/// change, then for each billable entry, not billed yet, on whose day no
/// rate of its client holds.
#[must_use]
pub fn billing_findings(input: &DoctorInput<'_>) -> Vec<DataFinding> {
    let mut findings: Vec<DataFinding> = input
        .state
        .billings()
        .filter_map(|(id, billing)| billed_at_other_amount(input, id, billing))
        .collect();
    findings.extend(
        input
            .state
            .entries()
            .filter_map(|entry| before_first_rate(input, entry)),
    );
    findings
}

/// An amount with its currency: `100.00 EUR`.
fn with_currency(money: Money) -> String {
    format!("{money} {}", money.currency())
}

/// The start day of `entry` in the reports' zone, which picks its rate.
fn day_of(input: &DoctorInput<'_>, entry: &Entry) -> Date {
    entry.start.with_time_zone(input.zone.clone()).date()
}

/// The record of the project of `entry`, if the catalog holds one.
fn record_of<'input>(input: &DoctorInput<'input>, entry: &Entry) -> Option<&'input ProjectRecord> {
    entry
        .project
        .as_ref()
        .and_then(|project| input.catalog.resolve(project))
}

/// The finding for the billed entry `id` if today's rate on its day
/// differs from its frozen rate and bills its billed time at another
/// amount, or no rate holds.
fn billed_at_other_amount(
    input: &DoctorInput<'_>,
    id: EntryId,
    billing: &Billing,
) -> Option<DataFinding> {
    let entry = input.state.entry(id)?;
    let day = day_of(input, entry);
    let record = record_of(input, entry);
    let today = match record.and_then(|record| input.catalog.rate_on(record, day)) {
        // A bill line splits its amount among its entries, so a frozen
        // amount can be a cent off its rate times its billed time: only a
        // changed rate changes it.
        Some(rate) if rate.hourly == billing.rate => return None,
        Some(rate) => {
            let amount = rate.hourly.amount_for(billing.billed).ok()?;
            if amount == billing.amount {
                return None;
            }
            format!("today's rates make it {}", with_currency(amount))
        }
        None => format!("today no rate holds on {day}"),
    };
    let short = input.state.short_id(id);
    let client = record.and_then(|record| record.client.as_ref());
    let mend = client.map_or_else(String::new, |client| {
        format!("; if the rate was wrong, mend it with `pace client edit {client}`")
    });
    Some(
        DataFinding::warning(
            DataArea::Billing,
            format!(
                "{short} went on the invoice {} with {}; {today}",
                billing.invoice,
                with_currency(billing.amount)
            ),
            format!("a written invoice keeps its amounts{mend}"),
        )
        .about(short),
    )
}

/// The finding for `entry` if it is billable, not billed yet, and no
/// rate of its project's client holds on its day: `pace bill` refuses it.
fn before_first_rate(input: &DoctorInput<'_>, entry: &Entry) -> Option<DataFinding> {
    if input.state.billing(entry.id).is_some() || !entry.effective_billable(input.catalog) {
        return None;
    }
    let record = record_of(input, entry)?;
    let client: &ClientKey = record.client.as_ref()?;
    _ = input.catalog.client(client)?;
    let day = day_of(input, entry);
    if input.catalog.rate_on(record, day).is_some() {
        return None;
    }
    let short = input.state.short_id(entry.id);
    Some(
        DataFinding::warning(
            DataArea::Billing,
            format!(
                "{short} is billable, but no rate of the client `{client}` holds on its day {day}"
            ),
            format!("add an earlier rate with `pace client edit {client}`"),
        )
        .about(short),
    )
}
