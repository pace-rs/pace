//! The bill of a client: its billable, ended entries that are not billed
//! yet, refused on an overlap or an entry without a rate; and the billing
//! that a `billing.marked` event gives an entry.

use std::collections::BTreeMap;

use jiff::{SignedDuration, civil::Date};
use pace_core::{
    BILL_BEFORE_FIRST_RATE, BILL_OVERLAP, Bill, BillInput, Billing, Catalog, Change, Client,
    EntryChanges, EntryId, EventKind, HoldSource, Money, ProjectRef, Rate, Rates, Rounding,
    RoundingMode, RoundingPer, State,
};
use pace_time::TimeRange;

use crate::{
    TestResult,
    report::{Fallible, Log, berlin, date, days, hours, mins, on, project, record, role, zone},
};

const WORK: &str = "0193a7c4-0000-7000-8000-00000000000a";
pub(crate) const WEBSHOP: &str = "0193a7c4-0000-7000-8000-000000000001";
const SHOP_APP: &str = "0193a7c4-0000-7000-8000-000000000003";
pub(crate) const SUPPORT: &str = "0193a7c4-0000-7000-8000-000000000004";
pub(crate) const BETA_SITE: &str = "0193a7c4-0000-7000-8000-000000000005";
const GARDEN: &str = "0193a7c4-0000-7000-8000-000000000002";

/// Now, after every entry of the tests.
pub(crate) const LATER: &str = "2026-12-31T00:00";

pub(crate) fn eur(amount: &str) -> Fallible<Money> {
    Ok(Money::parse_decimal(amount, "EUR".parse()?)?)
}

/// The billing of one hour at 100 EUR on `invoice`.
pub(crate) fn one_hour_on(invoice: &str) -> Fallible<Billing> {
    Ok(Billing {
        invoice: invoice.parse()?,
        net: hours(1),
        billed: hours(1),
        rate: eur("100")?,
        amount: eur("100")?,
    })
}

/// Mark `entry` billed with `billing` at `at`, as a batch of its own.
pub(crate) fn mark_billed(
    log: &mut Log,
    entry: EntryId,
    at: &str,
    billing: Billing,
) -> Fallible<()> {
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

fn rates(list: &[(&str, &str)]) -> Fallible<Rates> {
    Ok(Rates::new(
        list.iter()
            .map(|(hourly, from)| {
                Ok(Rate {
                    hourly: eur(hourly)?,
                    from: date(from)?,
                })
            })
            .collect::<Fallible<Vec<Rate>>>()?,
    )?)
}

/// acme bills in EUR: 100 an hour from 1 January 2026, 120 from
/// 15 November; its project acme-app has 150 from 1 November. beta bills
/// 80 an hour.
fn clients(rounding: Rounding) -> Fallible<Vec<Client>> {
    Ok(vec![
        Client {
            key: "acme".parse()?,
            name: "ACME GmbH".to_owned(),
            currency: "EUR".parse()?,
            rounding,
            rates: rates(&[("100", "2026-01-01"), ("120", "2026-11-15")])?,
            project_rates: BTreeMap::from([(
                "acme-app".parse()?,
                rates(&[("150", "2026-11-01")])?,
            )]),
        },
        Client {
            key: "beta".parse()?,
            name: "Beta AG".to_owned(),
            currency: "EUR".parse()?,
            rounding: Rounding::NONE,
            rates: rates(&[("80", "2026-01-01")])?,
            project_rates: BTreeMap::new(),
        },
    ])
}

/// The billable webshop, app and support of acme, whose support allows
/// overlaps; the billable site of beta; the garden, without a client.
pub(crate) fn catalog(rounding: Rounding) -> Fallible<Catalog> {
    let billable = |id, key, client| -> Fallible<_> {
        let mut project = record(id, key, "work", Some(client))?;
        project.billable = true;
        Ok(project)
    };
    let mut support = billable(SUPPORT, "acme-support", "acme")?;
    support.allow_overlap = true;
    Ok(Catalog::new(
        vec![role(WORK, "work", None)?],
        clients(rounding)?,
        vec![
            billable(WEBSHOP, "acme-webshop", "acme")?,
            billable(SHOP_APP, "acme-app", "acme")?,
            support,
            billable(BETA_SITE, "beta-site", "beta")?,
            record(GARDEN, "garden", "work", None)?,
        ],
    )?)
}

pub(crate) fn quarter_hours(per: RoundingPer) -> Fallible<Rounding> {
    Ok(Rounding::new(per, mins(15), RoundingMode::Up)?)
}

/// The bill of `client` over `period` in Berlin, at `LATER`.
fn bill_of(log: &Log, catalog: &Catalog, client: &str, period: &TimeRange) -> Fallible<Bill> {
    let client = catalog.client(&client.parse()?).ok_or("a known client")?;
    Ok(Bill::build(&BillInput {
        state: &log.state(),
        catalog,
        client,
        period,
        zone: &zone()?,
        now: &berlin(LATER)?,
    })?)
}

/// The bill of acme for November 2026.
fn november(log: &Log, catalog: &Catalog) -> Fallible<Bill> {
    bill_of(log, catalog, "acme", &days("2026-11-01", "2026-12-01")?)
}

/// The refusal of the bill of acme for November 2026.
fn refused_november(log: &Log, catalog: &Catalog) -> Fallible<Box<pace_error::PaceError>> {
    let client = catalog.client(&"acme".parse()?).ok_or("a known client")?;
    match Bill::build(&BillInput {
        state: &log.state(),
        catalog,
        client,
        period: &days("2026-11-01", "2026-12-01")?,
        zone: &zone()?,
        now: &berlin(LATER)?,
    }) {
        Ok(bill) => Err(format!("the bill must be refused, but is {bill:?}").into()),
        Err(error) => Ok(error),
    }
}

/// The entries of the bill's lines, line by line.
fn line_entries(bill: &Bill) -> Vec<Vec<EntryId>> {
    bill.projects
        .iter()
        .flat_map(|project| &project.lines)
        .map(|line| line.entries.iter().map(|entry| entry.id).collect())
        .collect()
}

/// The amounts of the bill's lines, line by line.
fn line_amounts(bill: &Bill) -> Vec<String> {
    bill.projects
        .iter()
        .flat_map(|project| &project.lines)
        .map(|line| line.amount.to_string())
        .collect()
}

/// Adjust `entry` with `changes` at `at`.
fn adjust(log: &mut Log, entry: EntryId, at: &str, changes: EntryChanges) -> Fallible<()> {
    log.record(at, EventKind::EntryAdjusted { entry, changes })
}

#[test]
fn bill_has_ended_billable_entries_of_client_in_period() -> TestResult {
    let mut log = Log::new(1)?;
    let webshop = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    let app = log.log("2026-11-20T11:00", "2026-11-20T12:00", on(SHOP_APP)?)?;
    _ = log.log("2026-11-20T13:00", "2026-11-20T14:00", on(BETA_SITE)?)?;
    _ = log.log("2026-11-20T15:00", "2026-11-20T16:00", on(GARDEN)?)?;
    _ = log.log("2026-10-20T09:00", "2026-10-20T10:00", on(WEBSHOP)?)?;
    let not_billable = log.log("2026-11-21T09:00", "2026-11-21T10:00", on(WEBSHOP)?)?;
    adjust(
        &mut log,
        not_billable,
        "2026-11-21T10:00",
        EntryChanges {
            billable: Change::Set(false),
            ..EntryChanges::default()
        },
    )?;

    let bill = november(&log, &catalog(Rounding::NONE)?)?;

    let projects: Vec<&ProjectRef> = bill.projects.iter().map(|p| &p.project).collect();
    assert_eq!(projects, vec![&project(SHOP_APP)?, &project(WEBSHOP)?]);
    assert_eq!(line_entries(&bill), vec![vec![app], vec![webshop]]);
    assert_eq!(line_amounts(&bill), vec!["150.00", "120.00"]);
    assert_eq!(bill.client.as_str(), "acme");
    Ok(())
}

#[test]
fn billed_entries_are_left_out() -> TestResult {
    let mut log = Log::new(1)?;
    let billed = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    let open = log.log("2026-11-21T09:00", "2026-11-21T10:00", on(WEBSHOP)?)?;
    mark_billed(
        &mut log,
        billed,
        "2026-11-22T09:00",
        one_hour_on("2026-0042")?,
    )?;

    let bill = november(&log, &catalog(Rounding::NONE)?)?;

    assert_eq!(line_entries(&bill), vec![vec![open]]);
    Ok(())
}

#[test]
fn bill_of_only_billed_entries_is_empty() -> TestResult {
    let mut log = Log::new(1)?;
    let billed = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    mark_billed(
        &mut log,
        billed,
        "2026-11-22T09:00",
        one_hour_on("2026-0042")?,
    )?;

    let bill = november(&log, &catalog(Rounding::NONE)?)?;

    assert!(bill.is_empty());
    assert_eq!(bill.totals, Vec::new());
    Ok(())
}

#[test]
fn running_entry_is_left_out_with_note_to_end_it() -> TestResult {
    let mut log = Log::new(1)?;
    let ended = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    let running = log.begin("2026-11-30T09:00", on(WEBSHOP)?)?;

    let bill = november(&log, &catalog(Rounding::NONE)?)?;

    assert_eq!(line_entries(&bill), vec![vec![ended]]);
    assert_eq!(bill.running, vec![running]);
    Ok(())
}

#[test]
fn overlap_among_entries_is_refused_with_their_short_ids() -> TestResult {
    let mut log = Log::new(1)?;
    let first = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    let second = log.log("2026-11-20T09:30", "2026-11-20T11:00", on(SHOP_APP)?)?;

    let error = refused_november(&log, &catalog(Rounding::NONE)?)?;

    let state = log.state();
    assert!(error.is_code(BILL_OVERLAP));
    assert_eq!(
        error.to_string(),
        format!(
            "pace bills no time twice, but these entries overlap: {} and {} share 09:30–10:00 \
             on 2026-11-20",
            state.short_id(first),
            state.short_id(second)
        )
    );
    assert!(
        error
            .hint()
            .is_some_and(|hint| hint.contains("pace adjust"))
    );
    Ok(())
}

/// Assert that `error` refuses the bill for the overlap of `first` and
/// `second` from 09:30 to `until` on 20 November.
fn assert_overlap_refused(
    error: &pace_error::PaceError,
    state: &State,
    (first, second): (EntryId, EntryId),
    until: &str,
) {
    assert!(error.is_code(BILL_OVERLAP));
    assert_eq!(
        error.to_string(),
        format!(
            "pace bills no time twice, but these entries overlap: {} and {} share 09:30–{until} \
             on 2026-11-20",
            state.short_id(first),
            state.short_id(second)
        )
    );
}

#[test]
fn overlap_with_entry_of_other_client_is_refused() -> TestResult {
    let mut log = Log::new(1)?;
    let acme = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    let beta = log.log("2026-11-20T09:30", "2026-11-20T11:00", on(BETA_SITE)?)?;

    let error = refused_november(&log, &catalog(Rounding::NONE)?)?;

    assert_overlap_refused(&error, &log.state(), (acme, beta), "10:00");
    Ok(())
}

#[test]
fn overlap_with_entry_billed_earlier_is_refused() -> TestResult {
    let mut log = Log::new(1)?;
    let billed = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    mark_billed(
        &mut log,
        billed,
        "2026-11-22T09:00",
        one_hour_on("2026-0042")?,
    )?;
    let synced_late = log.log("2026-11-20T09:30", "2026-11-20T10:00", on(SHOP_APP)?)?;

    let error = refused_november(&log, &catalog(Rounding::NONE)?)?;

    assert_overlap_refused(&error, &log.state(), (billed, synced_late), "10:00");
    Ok(())
}

#[test]
fn overlap_with_running_entry_is_refused() -> TestResult {
    let mut log = Log::new(1)?;
    let ended = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    let running = log.begin("2026-11-20T09:30", on(SHOP_APP)?)?;

    let error = refused_november(&log, &catalog(Rounding::NONE)?)?;

    assert_overlap_refused(&error, &log.state(), (ended, running), "10:00");
    Ok(())
}

#[test]
fn overlap_with_entry_that_is_not_billable_is_refused() -> TestResult {
    let mut log = Log::new(1)?;
    let webshop = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    let garden = log.log("2026-11-20T09:30", "2026-11-20T11:00", on(GARDEN)?)?;

    let error = refused_november(&log, &catalog(Rounding::NONE)?)?;

    assert_overlap_refused(&error, &log.state(), (webshop, garden), "10:00");
    Ok(())
}

#[test]
fn overlap_of_entries_off_bill_does_not_block_it() -> TestResult {
    let mut log = Log::new(1)?;
    let acme = log.log("2026-11-20T08:00", "2026-11-20T09:00", on(WEBSHOP)?)?;
    _ = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(BETA_SITE)?)?;
    _ = log.log("2026-11-20T09:30", "2026-11-20T11:00", on(GARDEN)?)?;

    let bill = november(&log, &catalog(Rounding::NONE)?)?;

    assert_eq!(line_entries(&bill), vec![vec![acme]]);
    Ok(())
}

#[test]
fn overlap_with_other_entry_is_allowed_by_entry_of_bill() -> TestResult {
    let mut log = Log::new(1)?;
    let support = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(SUPPORT)?)?;
    _ = log.log("2026-11-20T09:30", "2026-11-20T11:00", on(BETA_SITE)?)?;

    let bill = november(&log, &catalog(Rounding::NONE)?)?;

    assert_eq!(line_entries(&bill), vec![vec![support]]);
    Ok(())
}

#[test]
fn overlap_is_allowed_by_project_record() -> TestResult {
    let mut log = Log::new(1)?;
    let webshop = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    let support = log.log("2026-11-20T09:30", "2026-11-20T11:00", on(SUPPORT)?)?;

    let bill = november(&log, &catalog(Rounding::NONE)?)?;

    assert_eq!(line_entries(&bill), vec![vec![support], vec![webshop]]);
    Ok(())
}

#[test]
fn overlap_is_allowed_by_entry() -> TestResult {
    let mut log = Log::new(1)?;
    let first = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    let second = log.log("2026-11-20T09:30", "2026-11-20T11:00", on(WEBSHOP)?)?;
    adjust(
        &mut log,
        second,
        "2026-11-20T11:00",
        EntryChanges {
            allow_overlap: true,
            ..EntryChanges::default()
        },
    )?;

    let bill = november(&log, &catalog(Rounding::NONE)?)?;

    assert_eq!(line_entries(&bill), vec![vec![first], vec![second]]);
    Ok(())
}

#[test]
fn entry_before_first_rate_is_refused() -> TestResult {
    let mut log = Log::new(1)?;
    let early = log.log("2025-12-31T09:00", "2025-12-31T10:00", on(WEBSHOP)?)?;
    let catalog = catalog(Rounding::NONE)?;
    let client = catalog.client(&"acme".parse()?).ok_or("acme")?;

    let error = Bill::build(&BillInput {
        state: &log.state(),
        catalog: &catalog,
        client,
        period: &days("2025-12-01", "2026-01-01")?,
        zone: &zone()?,
        now: &berlin(LATER)?,
    })
    .err()
    .ok_or("a refusal")?;

    assert!(error.is_code(BILL_BEFORE_FIRST_RATE));
    assert_eq!(
        error.to_string(),
        format!(
            "no rate of the client acme holds on the start day of {} (2025-12-31)",
            log.state().short_id(early)
        )
    );
    assert_eq!(error.hint(), Some("add an earlier rate to clients.toml"));
    Ok(())
}

#[test]
fn rounding_per_day_and_project_bills_each_day_as_one_line() -> TestResult {
    let mut log = Log::new(1)?;
    let first = log.log("2026-11-20T09:00", "2026-11-20T09:05", on(WEBSHOP)?)?;
    let second = log.log("2026-11-20T14:00", "2026-11-20T14:05", on(WEBSHOP)?)?;
    let next_day = log.log("2026-11-21T09:00", "2026-11-21T09:05", on(WEBSHOP)?)?;

    let bill = november(&log, &catalog(quarter_hours(RoundingPer::Day)?)?)?;

    assert_eq!(
        line_entries(&bill),
        vec![vec![first, second], vec![next_day]]
    );
    let line = bill.projects[0].lines.first().ok_or("a line")?;
    assert_eq!((line.net, line.billed), (mins(10), mins(15)));
    assert_eq!(line_amounts(&bill), vec!["30.00", "30.00"]);
    assert_eq!(bill.projects[0].billed, mins(30));
    Ok(())
}

#[test]
fn rounding_per_entry_bills_each_entry_as_one_line() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-20T09:00", "2026-11-20T09:05", on(WEBSHOP)?)?;
    _ = log.log("2026-11-20T14:00", "2026-11-20T14:05", on(WEBSHOP)?)?;

    let bill = november(&log, &catalog(quarter_hours(RoundingPer::Entry)?)?)?;

    assert_eq!(line_amounts(&bill), vec!["30.00", "30.00"]);
    assert_eq!((bill.net, bill.billed), (mins(10), mins(30)));
    Ok(())
}

#[test]
fn rate_change_in_month_prices_each_entry_by_its_start_day() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-14T09:00", "2026-11-14T10:00", on(WEBSHOP)?)?;
    _ = log.log("2026-11-15T09:00", "2026-11-15T10:00", on(WEBSHOP)?)?;

    let bill = november(&log, &catalog(Rounding::NONE)?)?;

    let lines = &bill.projects[0].lines;
    let rates: Vec<String> = lines.iter().map(|line| line.rate.to_string()).collect();
    assert_eq!(rates, vec!["100.00", "120.00"]);
    assert_eq!(bill.projects[0].amount, eur("220")?);
    assert_eq!(bill.totals, vec![eur("220")?]);
    Ok(())
}

#[test]
fn entry_over_months_end_belongs_to_month_it_starts_in() -> TestResult {
    let mut log = Log::new(1)?;
    let late = log.log("2026-10-31T23:00", "2026-11-01T01:00", on(WEBSHOP)?)?;
    let catalog = catalog(Rounding::NONE)?;

    let october = bill_of(&log, &catalog, "acme", &days("2026-10-01", "2026-11-01")?)?;
    let november = november(&log, &catalog)?;

    assert_eq!(line_entries(&october), vec![vec![late]]);
    let line = october.projects[0].lines.first().ok_or("a line")?;
    assert_eq!(line.date, date("2026-10-31")?);
    assert_eq!(line.billed, hours(2));
    assert!(november.is_empty());
    Ok(())
}

#[test]
fn line_shows_date_note_and_net_time_of_each_entry() -> TestResult {
    let mut log = Log::new(1)?;
    let entry = log.log("2026-11-20T09:00", "2026-11-20T10:30", on(WEBSHOP)?)?;
    adjust(
        &mut log,
        entry,
        "2026-11-20T10:30",
        EntryChanges {
            note: Change::Set("Checkout flow".to_owned()),
            ..EntryChanges::default()
        },
    )?;

    let bill = november(&log, &catalog(Rounding::NONE)?)?;

    let line = bill.projects[0].lines.first().ok_or("a line")?;
    let shown: Vec<(EntryId, Option<&str>, SignedDuration)> = line
        .entries
        .iter()
        .map(|entry| (entry.id, entry.note.as_deref(), entry.net))
        .collect();
    assert_eq!(shown, vec![(entry, Some("Checkout flow"), mins(90))]);
    let expected: (Date, SignedDuration, String) = (date("2026-11-20")?, mins(90), "180.00".into());
    assert_eq!((line.date, line.net, line.amount.to_string()), expected);
    Ok(())
}

#[test]
fn hold_share_is_hold_time_over_gross_time_of_billed_entries() -> TestResult {
    let mut log = Log::new(1)?;
    let held = log.begin("2026-11-20T09:00", on(WEBSHOP)?)?;
    log.hold(
        held,
        "2026-11-20T09:30",
        "2026-11-20T10:00",
        HoldSource::Outer,
    )?;
    log.end(held, "2026-11-20T11:00")?;
    _ = log.log("2026-11-21T09:00", "2026-11-21T11:00", on(WEBSHOP)?)?;

    let bill = november(&log, &catalog(Rounding::NONE)?)?;

    assert_eq!(bill.hold_share.hold, mins(30));
    assert_eq!(bill.hold_share.gross, hours(4));
    assert_eq!(bill.hold_share.percent, 13);
    assert_eq!(bill.net, mins(210));
    Ok(())
}

#[test]
fn bill_without_entries_has_no_hold_share() -> TestResult {
    let log = Log::new(1)?;

    let bill = november(&log, &catalog(Rounding::NONE)?)?;

    assert!(bill.is_empty());
    assert_eq!(bill.hold_share.percent, 0);
    Ok(())
}
