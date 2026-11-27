//! Billing: billable time, rounded by the client's rule per entry or per
//! day and project, priced at the rate of each entry's start day.

use std::collections::BTreeMap;

use jiff::SignedDuration;
use pace_core::{
    Catalog, Charge, Client, Entry, EntryId, Money, Rate, Rates, Rounding, RoundingMode,
    RoundingPer, billable_charges,
};

use crate::{
    TestResult,
    report::{Fallible, LATER, Log, berlin, date, hours, mins, on, project, record, role, zone},
};

const WORK: &str = "0193a7c4-0000-7000-8000-00000000000a";
const WEBSHOP: &str = "0193a7c4-0000-7000-8000-000000000001";
const SHOP_APP: &str = "0193a7c4-0000-7000-8000-000000000003";
const GARDEN: &str = "0193a7c4-0000-7000-8000-000000000002";

fn rates(currency: &str, list: &[(&str, &str)]) -> Fallible<Rates> {
    let currency = currency.parse()?;
    Ok(Rates::new(
        list.iter()
            .map(|(hourly, from)| {
                Ok(Rate {
                    hourly: Money::parse_decimal(hourly, currency)?,
                    from: date(from)?,
                })
            })
            .collect::<Fallible<Vec<Rate>>>()?,
    )?)
}

/// The client acme in EUR: 100 an hour from 1 January 2026, 120 from
/// 15 November; its project acme-app has 150 from 1 November.
fn acme(rounding: Rounding) -> Fallible<Client> {
    Ok(Client {
        key: "acme".parse()?,
        name: "ACME GmbH".to_owned(),
        currency: "EUR".parse()?,
        rounding,
        rates: rates("EUR", &[("100", "2026-01-01"), ("120", "2026-11-15")])?,
        project_rates: BTreeMap::from([(
            "acme-app".parse()?,
            rates("EUR", &[("150", "2026-11-01")])?,
        )]),
    })
}

/// The webshop and the app of acme, both billable, and the garden, not
/// billable and without a client.
fn catalog(rounding: Rounding) -> Fallible<Catalog> {
    let mut webshop = record(WEBSHOP, "acme-webshop", "work", Some("acme"))?;
    webshop.billable = true;
    let mut app = record(SHOP_APP, "acme-app", "work", Some("acme"))?;
    app.billable = true;
    Ok(Catalog::new(
        vec![role(WORK, "work", None)?],
        vec![acme(rounding)?],
        vec![webshop, app, record(GARDEN, "garden", "work", None)?],
    )?)
}

fn quarter_hours(per: RoundingPer) -> Fallible<Rounding> {
    Ok(Rounding::new(per, mins(15), RoundingMode::Up)?)
}

fn charges_of(log: &Log, catalog: &Catalog) -> Fallible<Vec<Charge>> {
    let state = log.state();
    let entries: Vec<&Entry> = state.entries().collect();
    Ok(billable_charges(
        &entries,
        catalog,
        &zone()?,
        &berlin(LATER)?,
    )?)
}

fn billed(charges: &[Charge]) -> Vec<SignedDuration> {
    charges.iter().map(|charge| charge.billed).collect()
}

fn amounts(charges: &[Charge]) -> Vec<Option<String>> {
    charges
        .iter()
        .map(|charge| charge.price.as_ref().map(|price| price.amount.to_string()))
        .collect()
}

#[test]
fn two_five_minute_entries_round_up_per_entry_to_thirty_minutes() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-20T09:00", "2026-11-20T09:05", on(WEBSHOP)?)?;
    _ = log.log("2026-11-20T14:00", "2026-11-20T14:05", on(WEBSHOP)?)?;

    let charges = charges_of(&log, &catalog(quarter_hours(RoundingPer::Entry)?)?)?;

    assert_eq!(billed(&charges), vec![mins(15), mins(15)]);
    assert_eq!(amounts(&charges), vec![Some("30.00".to_owned()); 2]);
    Ok(())
}

#[test]
fn two_five_minute_entries_round_up_per_day_and_project_to_fifteen_minutes() -> TestResult {
    let mut log = Log::new(1)?;
    let first = log.log("2026-11-20T09:00", "2026-11-20T09:05", on(WEBSHOP)?)?;
    let second = log.log("2026-11-20T14:00", "2026-11-20T14:05", on(WEBSHOP)?)?;
    let other_project = log.log("2026-11-20T15:00", "2026-11-20T15:05", on(SHOP_APP)?)?;

    let charges = charges_of(&log, &catalog(quarter_hours(RoundingPer::Day)?)?)?;

    let entries: Vec<&[EntryId]> = charges.iter().map(|charge| &charge.entries[..]).collect();
    assert_eq!(entries, vec![&[first, second][..], &[other_project][..]]);
    assert_eq!(
        charges.iter().map(|charge| charge.net).collect::<Vec<_>>(),
        vec![mins(10), mins(5)]
    );
    assert_eq!(billed(&charges), vec![mins(15), mins(15)]);
    assert_eq!(
        amounts(&charges),
        vec![Some("30.00".to_owned()), Some("37.50".to_owned())]
    );
    Ok(())
}

#[test]
fn no_rounding_keeps_net_time() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-20T09:00", "2026-11-20T09:20", on(WEBSHOP)?)?;

    let charges = charges_of(&log, &catalog(Rounding::NONE)?)?;

    assert_eq!(billed(&charges), vec![mins(20)]);
    assert_eq!(amounts(&charges), vec![Some("40.00".to_owned())]);
    Ok(())
}

#[test]
fn rate_change_in_middle_of_period_prices_each_entry_by_its_day() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-14T09:00", "2026-11-14T10:00", on(WEBSHOP)?)?;
    _ = log.log("2026-11-15T09:00", "2026-11-15T10:00", on(WEBSHOP)?)?;

    let charges = charges_of(&log, &catalog(Rounding::NONE)?)?;

    let rates: Vec<Option<String>> = charges
        .iter()
        .map(|charge| {
            charge
                .price
                .as_ref()
                .map(|price| price.rate.hourly.to_string())
        })
        .collect();
    assert_eq!(
        rates,
        vec![Some("100.00".to_owned()), Some("120.00".to_owned())]
    );
    assert_eq!(
        amounts(&charges),
        vec![Some("100.00".to_owned()), Some("120.00".to_owned())]
    );
    Ok(())
}

#[test]
fn entry_over_midnight_counts_whole_at_rate_of_its_start_day() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-14T23:00", "2026-11-15T01:00", on(WEBSHOP)?)?;

    let charges = charges_of(&log, &catalog(Rounding::NONE)?)?;

    let charge = charges.first().ok_or("one charge")?;
    assert_eq!(charge.date, date("2026-11-14")?);
    assert_eq!(charge.billed, hours(2));
    assert_eq!(amounts(&charges), vec![Some("200.00".to_owned())]);
    Ok(())
}

#[test]
fn project_rates_come_before_client_rates() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(SHOP_APP)?)?;

    let charges = charges_of(&log, &catalog(Rounding::NONE)?)?;

    assert_eq!(amounts(&charges), vec![Some("150.00".to_owned())]);
    Ok(())
}

#[test]
fn entry_before_first_rate_has_no_price() -> TestResult {
    let mut log = Log::new(1)?;
    let early = log.log("2025-12-31T09:00", "2025-12-31T10:00", on(WEBSHOP)?)?;

    let charges = charges_of(&log, &catalog(quarter_hours(RoundingPer::Entry)?)?)?;

    let charge = charges.first().ok_or("one charge")?;
    assert_eq!(charge.entries, vec![early]);
    assert_eq!(charge.net, hours(1));
    assert_eq!(charge.price, None);
    Ok(())
}

#[test]
fn entries_that_are_not_billable_get_no_charge() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(GARDEN)?)?;

    let charges = charges_of(&log, &catalog(Rounding::NONE)?)?;

    assert_eq!(charges, Vec::new());
    Ok(())
}

#[test]
fn charge_names_project_by_record_and_its_client() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;

    let charges = charges_of(&log, &catalog(Rounding::NONE)?)?;

    let charge = charges.first().ok_or("one charge")?;
    assert_eq!(charge.project, Some(project(WEBSHOP)?));
    assert_eq!(charge.client, Some("acme".parse()?));
    Ok(())
}

#[test]
fn hold_time_is_not_billed() -> TestResult {
    let mut log = Log::new(1)?;
    let entry = log.begin("2026-11-20T09:00", on(WEBSHOP)?)?;
    log.hold(
        entry,
        "2026-11-20T09:30",
        "2026-11-20T10:00",
        pace_core::HoldSource::Outer,
    )?;
    log.end(entry, "2026-11-20T11:00")?;

    let charges = charges_of(&log, &catalog(Rounding::NONE)?)?;

    assert_eq!(billed(&charges), vec![mins(90)]);
    Ok(())
}
