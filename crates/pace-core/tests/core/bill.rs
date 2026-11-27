//! The bill of a client: its billable, ended entries that are not billed
//! yet, refused on an overlap or an entry without a rate; and the billing
//! that a `billing.marked` event gives an entry.

use pace_core::{Billing, EntryId, EventKind, Money, State};

use crate::{
    TestResult,
    report::{Fallible, Log, hours, on},
};

const WEBSHOP: &str = "0193a7c4-0000-7000-8000-000000000001";

fn eur(amount: &str) -> Fallible<Money> {
    Ok(Money::parse_decimal(amount, "EUR".parse()?)?)
}

/// The billing of one hour at 100 EUR on `invoice`.
fn one_hour_on(invoice: &str) -> Fallible<Billing> {
    Ok(Billing {
        invoice: invoice.parse()?,
        net: hours(1),
        billed: hours(1),
        rate: eur("100")?,
        amount: eur("100")?,
    })
}

/// Mark `entry` billed with `billing` at `at`, as a batch of its own.
fn mark_billed(log: &mut Log, entry: EntryId, at: &str, billing: Billing) -> Fallible<()> {
    log.record(at, EventKind::BillingMarked { entry, billing })
}

/// Undo the last batch of `log` at `at`.
fn undo_last(log: &mut Log, at: &str) -> Fallible<()> {
    let batch = log.events().last().ok_or("a batch to undo")?.batch;
    log.record(at, EventKind::BatchUndone { batch })
}

#[test]
fn billing_marked_gives_entry_its_frozen_billing() -> TestResult {
    let mut log = Log::new(1)?;
    let entry = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    mark_billed(
        &mut log,
        entry,
        "2026-12-01T09:00",
        one_hour_on("2026-0042")?,
    )?;

    let state = log.state();

    assert_eq!(state.billing(entry), Some(&one_hour_on("2026-0042")?));
    Ok(())
}

#[test]
fn first_marking_of_entry_stays() -> TestResult {
    let mut log = Log::new(1)?;
    let entry = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    mark_billed(
        &mut log,
        entry,
        "2026-12-01T09:00",
        one_hour_on("2026-0042")?,
    )?;
    mark_billed(
        &mut log,
        entry,
        "2026-12-02T09:00",
        one_hour_on("2026-0043")?,
    )?;

    let state = log.state();

    let invoice = state.billing(entry).map(|billing| billing.invoice.as_str());
    assert_eq!(invoice, Some("2026-0042"));
    Ok(())
}

#[test]
fn undo_of_marking_takes_billing_back() -> TestResult {
    let mut log = Log::new(1)?;
    let entry = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    mark_billed(
        &mut log,
        entry,
        "2026-12-01T09:00",
        one_hour_on("2026-0042")?,
    )?;
    undo_last(&mut log, "2026-12-01T09:05")?;

    let state = log.state();

    assert_eq!(state.billing(entry), None);
    assert!(state.entry(entry).is_some());
    Ok(())
}

#[test]
fn undo_of_marking_on_top_takes_billing_back() -> TestResult {
    let mut log = Log::new(1)?;
    let entry = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    mark_billed(
        &mut log,
        entry,
        "2026-12-01T09:00",
        one_hour_on("2026-0042")?,
    )?;
    undo_last(&mut log, "2026-12-01T09:05")?;
    let (read, undo) = log.events().split_at(log.events().len() - 1);

    let before = State::from_events(read);
    let state = before.clone().followed_by(undo, read);

    assert!(before.billing(entry).is_some());
    assert_eq!(state.billing(entry), None);
    Ok(())
}

#[test]
fn forgotten_entry_has_no_billing() -> TestResult {
    let mut log = Log::new(1)?;
    let entry = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    mark_billed(
        &mut log,
        entry,
        "2026-12-01T09:00",
        one_hour_on("2026-0042")?,
    )?;
    log.record("2026-12-02T09:00", EventKind::EntryForgotten { entry })?;
    let (read, forget) = log.events().split_at(log.events().len() - 1);

    let rebuilt = log.state();
    let on_top = State::from_events(read).followed_by(forget, read);

    assert_eq!(rebuilt.billing(entry), None);
    assert_eq!(on_top.billing(entry), None);
    assert_eq!(on_top, rebuilt);
    Ok(())
}

#[test]
fn billing_of_unknown_entry_changes_nothing() -> TestResult {
    let mut log = Log::new(1)?;
    let entry = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    let unknown: EntryId = "0193a7c4-0000-7000-8000-0000deadbeef".parse()?;
    mark_billed(
        &mut log,
        unknown,
        "2026-12-01T09:00",
        one_hour_on("2026-0042")?,
    )?;

    let state = log.state();

    assert_eq!(state.billing(unknown), None);
    assert_eq!(state.billing(entry), None);
    Ok(())
}
