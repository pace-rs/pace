//! A written invoice shown again: the bill of the entries billed on one
//! reference, from their frozen amounts; and the client a reference names.

use pace_core::{
    Bill, BillInput, Billing, Catalog, EntryId, InvoiceInput, InvoiceRef, Rounding, RoundingPer,
    UNKNOWN_INVOICE, invoice_client,
};

use crate::{
    TestResult,
    bill::{BETA_SITE, LATER, WEBSHOP, catalog, eur, mark_billed, one_hour_on, quarter_hours},
    report::{Fallible, Log, berlin, days, hours, mins, on, zone},
};

/// The invoice `invoice` of acme, as `bill --invoice` shows it at `LATER`.
fn acme_invoice(log: &Log, catalog: &Catalog, invoice: &str) -> Fallible<Bill> {
    let client = catalog.client(&"acme".parse()?).ok_or("a known client")?;
    Ok(Bill::of_invoice(&InvoiceInput {
        state: &log.state(),
        catalog,
        client,
        invoice: &invoice.parse()?,
        zone: &zone()?,
        now: &berlin(LATER)?,
    })?)
}

/// Mark acme's November 2026 on `invoice` as `bill --mark-billed` freezes
/// it, one billing per entry.
fn mark_november(log: &mut Log, catalog: &Catalog, invoice: &str) -> Fallible<()> {
    let client = catalog.client(&"acme".parse()?).ok_or("a known client")?;
    let bill = Bill::build(&BillInput {
        state: &log.state(),
        catalog,
        client,
        period: &days("2026-11-01", "2026-12-01")?,
        zone: &zone()?,
        now: &berlin(LATER)?,
    })?;
    for (entry, billing) in bill.billings(&invoice.parse()?) {
        mark_billed(log, entry, "2026-12-01T09:00", billing)?;
    }
    Ok(())
}

fn line_entries(bill: &Bill) -> Vec<Vec<EntryId>> {
    bill.projects
        .iter()
        .flat_map(|project| &project.lines)
        .map(|line| line.entries.iter().map(|entry| entry.id).collect())
        .collect()
}

#[test]
fn invoice_shows_frozen_amounts_not_todays_rates() -> TestResult {
    let mut log = Log::new(1)?;
    let entry = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    let frozen = Billing {
        rate: eur("90")?,
        amount: eur("90.00")?,
        ..one_hour_on("2026-0042")?
    };
    mark_billed(&mut log, entry, "2026-12-01T09:00", frozen)?;

    let bill = acme_invoice(&log, &catalog(Rounding::NONE)?, "2026-0042")?;

    let line = bill.projects[0].lines.first().ok_or("a line")?;
    assert_eq!(line.rate.to_string(), "90.00");
    assert_eq!(line.amount.to_string(), "90.00");
    assert_eq!(bill.totals, vec![eur("90.00")?]);
    assert_eq!(bill.invoice, Some("2026-0042".parse::<InvoiceRef>()?));
    assert_eq!(bill.running, Vec::<EntryId>::new());
    Ok(())
}

#[test]
fn invoice_of_client_rounding_per_day_has_its_bills_lines() -> TestResult {
    let mut log = Log::new(1)?;
    let first = log.log("2026-11-20T09:00", "2026-11-20T09:05", on(WEBSHOP)?)?;
    let second = log.log("2026-11-20T14:00", "2026-11-20T14:05", on(WEBSHOP)?)?;
    let next_day = log.log("2026-11-21T09:00", "2026-11-21T09:05", on(WEBSHOP)?)?;
    let catalog = catalog(quarter_hours(RoundingPer::Day)?)?;
    mark_november(&mut log, &catalog, "2026-0042")?;

    let bill = acme_invoice(&log, &catalog, "2026-0042")?;

    assert_eq!(
        line_entries(&bill),
        vec![vec![first, second], vec![next_day]]
    );
    let line = bill.projects[0].lines.first().ok_or("a line")?;
    assert_eq!((line.net, line.billed), (mins(10), mins(15)));
    assert_eq!(line.amount.to_string(), "30.00");
    assert_eq!((bill.net, bill.billed), (mins(15), mins(30)));
    Ok(())
}

#[test]
fn invoice_of_client_rounding_per_entry_has_line_per_entry() -> TestResult {
    let mut log = Log::new(1)?;
    let first = log.log("2026-11-20T09:00", "2026-11-20T09:05", on(WEBSHOP)?)?;
    let second = log.log("2026-11-20T14:00", "2026-11-20T14:05", on(WEBSHOP)?)?;
    let catalog = catalog(quarter_hours(RoundingPer::Entry)?)?;
    mark_november(&mut log, &catalog, "2026-0042")?;

    let bill = acme_invoice(&log, &catalog, "2026-0042")?;

    assert_eq!(line_entries(&bill), vec![vec![first], vec![second]]);
    Ok(())
}

#[test]
fn invoice_takes_only_entries_of_its_reference_and_client() -> TestResult {
    let mut log = Log::new(1)?;
    let ours = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    let other_invoice = log.log("2026-11-21T09:00", "2026-11-21T10:00", on(WEBSHOP)?)?;
    let beta = log.log("2026-11-22T09:00", "2026-11-22T10:00", on(BETA_SITE)?)?;
    _ = log.log("2026-11-23T09:00", "2026-11-23T10:00", on(WEBSHOP)?)?;
    for (entry, invoice) in [
        (ours, "2026-0042"),
        (other_invoice, "2026-0043"),
        (beta, "2026-0042"),
    ] {
        mark_billed(&mut log, entry, "2026-12-01T09:00", one_hour_on(invoice)?)?;
    }

    let bill = acme_invoice(&log, &catalog(Rounding::NONE)?, "2026-0042")?;

    assert_eq!(line_entries(&bill), vec![vec![ours]]);
    assert_eq!(bill.net, hours(1));
    Ok(())
}

/// acme's entry billed on 2026-0042, and beta's on `beta_invoice`.
fn two_clients_billed(beta_invoice: &str) -> Fallible<Log> {
    let mut log = Log::new(1)?;
    let acme = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    let beta = log.log("2026-11-22T09:00", "2026-11-22T10:00", on(BETA_SITE)?)?;
    mark_billed(
        &mut log,
        acme,
        "2026-12-01T09:00",
        one_hour_on("2026-0042")?,
    )?;
    mark_billed(
        &mut log,
        beta,
        "2026-12-01T09:00",
        one_hour_on(beta_invoice)?,
    )?;
    Ok(log)
}

#[test]
fn reference_alone_finds_client_of_invoice() -> TestResult {
    let log = two_clients_billed("B-7")?;
    let catalog = catalog(Rounding::NONE)?;

    let client = invoice_client(&log.state(), &catalog, &"B-7".parse()?, None)?;

    assert_eq!(client.key.as_str(), "beta");
    Ok(())
}

#[test]
fn reference_of_two_clients_asks_for_client() -> TestResult {
    let log = two_clients_billed("2026-0042")?;
    let catalog = catalog(Rounding::NONE)?;
    let state = log.state();
    let invoice = "2026-0042".parse()?;

    let error = invoice_client(&state, &catalog, &invoice, None)
        .err()
        .ok_or("two clients are no one")?;
    let beta = invoice_client(&state, &catalog, &invoice, Some(&"beta".parse()?))?;

    assert_eq!(
        error.to_string(),
        "the invoice `2026-0042` is of 2 clients: acme, beta"
    );
    assert_eq!(error.hint(), Some("name one with `--client <key>`"));
    assert!(error.is_code(UNKNOWN_INVOICE));
    assert_eq!(beta.key.as_str(), "beta");
    Ok(())
}

#[test]
fn unknown_reference_names_closest() -> TestResult {
    let log = two_clients_billed("B-7")?;
    let catalog = catalog(Rounding::NONE)?;

    let error = invoice_client(&log.state(), &catalog, &"2026-0024".parse()?, None)
        .err()
        .ok_or("an unknown reference")?;

    assert_eq!(
        error.to_string(),
        "pace doesn't know the invoice `2026-0024`"
    );
    assert_eq!(error.hint(), Some("did you mean `2026-0042`?"));
    assert!(error.is_code(UNKNOWN_INVOICE));
    Ok(())
}

#[test]
fn unknown_reference_far_from_any_lists_written_ones() -> TestResult {
    let log = two_clients_billed("B-7")?;
    let catalog = catalog(Rounding::NONE)?;
    let state = log.state();

    let error = invoice_client(&state, &catalog, &"zzz".parse()?, None)
        .err()
        .ok_or("an unknown reference")?;
    let none = invoice_client(&Log::new(2)?.state(), &catalog, &"zzz".parse()?, None)
        .err()
        .ok_or("no invoice at all")?;

    assert_eq!(
        error.hint(),
        Some("the written invoices are `2026-0042`, `B-7`")
    );
    assert_eq!(
        none.hint(),
        Some("no invoice is written yet; `pace bill --mark-billed <invoice>` writes one")
    );
    Ok(())
}

#[test]
fn reference_of_other_client_is_unknown_to_client() -> TestResult {
    let log = two_clients_billed("B-7")?;
    let catalog = catalog(Rounding::NONE)?;

    let error = invoice_client(
        &log.state(),
        &catalog,
        &"B-7".parse()?,
        Some(&"acme".parse()?),
    )
    .err()
    .ok_or("acme has no invoice B-7")?;

    assert_eq!(
        error.to_string(),
        "the invoice `B-7` is not of the client `acme`"
    );
    assert_eq!(
        error.hint(),
        Some("it is of beta: `--client beta` shows it")
    );
    assert!(error.is_code(UNKNOWN_INVOICE));
    Ok(())
}

#[test]
fn unknown_reference_with_client_names_closest_of_all() -> TestResult {
    let log = two_clients_billed("B-7")?;
    let catalog = catalog(Rounding::NONE)?;

    let error = invoice_client(
        &log.state(),
        &catalog,
        &"B-8".parse()?,
        Some(&"acme".parse()?),
    )
    .err()
    .ok_or("nobody has invoice B-8")?;

    assert_eq!(error.to_string(), "pace doesn't know the invoice `B-8`");
    assert_eq!(error.hint(), Some("did you mean `B-7`?"));
    Ok(())
}
