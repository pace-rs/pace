//! `bill --mark-billed`: the rules decide the bill on the current state
//! and freeze it, one `billing.marked` per entry of the bill; they refuse
//! as the bill does, and refuse an invoice reference the client used.

use jiff::{SignedDuration, ZonedDateTime};
use pace_core::{
    BILL_OVERLAP, Batch, Billing, Catalog, Command, EntryId, EventKind, INVOICE_USED, Policy,
    Rounding, RoundingPer, Rules, command::MarkBilled, testing::SequentialIds,
};
use pace_error::{ExitCode, PaceError};

use crate::{
    TestResult,
    bill::{BETA_SITE, LATER, WEBSHOP, catalog, eur, mark_billed, one_hour_on, quarter_hours},
    report::{Fallible, Log, berlin, mins, on, zone},
};

/// The marking of acme's November 2026 on `invoice`.
fn november_on(invoice: &str) -> Fallible<Command> {
    Ok(Command::MarkBilled(MarkBilled {
        client: "acme".parse()?,
        start: Some(berlin("2026-11-01T00:00")?),
        end: Some(berlin("2026-12-01T00:00")?),
        invoice: invoice.parse()?,
    }))
}

/// Decide `command` on the state of `log` at `LATER`, with reports in
/// Berlin.
fn decide(log: &Log, catalog: &Catalog, command: &Command) -> Result<Batch, Box<PaceError>> {
    let now: ZonedDateTime = berlin(LATER).map_err(|_| internal("LATER"))?;
    let ids = SequentialIds::starting_at(&now, 9);
    let policy = Policy {
        reports_time_zone: zone().map_err(|_| internal("Berlin"))?,
        ..Policy::default()
    };
    let device = "0193a8d0-000d-7000-8000-000000000009"
        .parse()
        .map_err(|_| internal("the device"))?;
    Rules {
        state: &log.state(),
        now: &now,
        device,
        ids: &ids,
        policy: &policy,
        catalog,
    }
    .decide(command)
}

fn internal(what: &str) -> Box<PaceError> {
    PaceError::new(
        pace_error::ErrorKind::Internal,
        "the test's {what} is wrong",
    )
    .attach_context("what", what)
}

fn refused(log: &Log, catalog: &Catalog, command: &Command) -> Fallible<Box<PaceError>> {
    match decide(log, catalog, command) {
        Ok(batch) => Err(format!("the marking must be refused, but is {batch:?}").into()),
        Err(error) => Ok(error),
    }
}

/// The entries and billings of the batch's `billing.marked` events.
fn billings(batch: &Batch) -> Fallible<Vec<(EntryId, Billing)>> {
    batch
        .events
        .iter()
        .map(|event| match &event.kind {
            EventKind::BillingMarked { entry, billing } => Ok((*entry, billing.clone())),
            other => Err(format!("only billing.marked, but {other:?}").into()),
        })
        .collect()
}

fn billing(
    invoice: &str,
    net: SignedDuration,
    billed: SignedDuration,
    rate: &str,
    amount: &str,
) -> Fallible<Billing> {
    Ok(Billing {
        invoice: invoice.parse()?,
        net,
        billed,
        rate: eur(rate)?,
        amount: eur(amount)?,
    })
}

#[test]
fn marking_freezes_each_line_of_bill_as_billing_of_its_entry() -> TestResult {
    let mut log = Log::new(1)?;
    let first = log.log("2026-11-10T09:00", "2026-11-10T09:50", on(WEBSHOP)?)?;
    let second = log.log("2026-11-20T09:00", "2026-11-20T09:20", on(WEBSHOP)?)?;
    let catalog = catalog(quarter_hours(RoundingPer::Entry)?)?;

    let batch = decide(&log, &catalog, &november_on("2026-0042")?)?;

    assert_eq!(
        billings(&batch)?,
        vec![
            (
                first,
                billing("2026-0042", mins(50), mins(60), "100", "100.00")?
            ),
            (
                second,
                billing("2026-0042", mins(20), mins(30), "120", "60.00")?
            ),
        ]
    );
    Ok(())
}

#[test]
fn marking_is_filed_under_its_entry_and_happens_now() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-10T09:00", "2026-11-10T09:50", on(WEBSHOP)?)?;
    let catalog = catalog(Rounding::NONE)?;

    let batch = decide(&log, &catalog, &november_on("2026-0042")?)?;

    let event = batch.events.first().ok_or("one event")?;
    assert_eq!(event.about, berlin("2026-11-10T09:00")?);
    assert_eq!(event.at, berlin(LATER)?);
    Ok(())
}

#[test]
fn line_of_several_entries_splits_its_billed_time_and_amount_by_net_time() -> TestResult {
    let mut log = Log::new(1)?;
    let four_minutes = log.log("2026-11-10T09:00", "2026-11-10T09:04", on(WEBSHOP)?)?;
    let eight_minutes = log.log("2026-11-10T14:00", "2026-11-10T14:08", on(WEBSHOP)?)?;
    let catalog = catalog(quarter_hours(RoundingPer::Day)?)?;

    let batch = decide(&log, &catalog, &november_on("2026-0042")?)?;

    assert_eq!(
        billings(&batch)?,
        vec![
            (
                four_minutes,
                billing("2026-0042", mins(4), mins(5), "100", "8.33")?
            ),
            (
                eight_minutes,
                billing("2026-0042", mins(8), mins(10), "100", "16.67")?
            ),
        ]
    );
    Ok(())
}

#[test]
fn split_of_line_adds_up_to_line_to_minor_unit() -> TestResult {
    let mut log = Log::new(1)?;
    for start in ["09:00", "11:00", "14:00"] {
        let end = start.replace(":00", ":05");
        _ = log.log(
            &format!("2026-11-10T{start}"),
            &format!("2026-11-10T{end}"),
            on(WEBSHOP)?,
        )?;
    }
    let catalog = catalog(quarter_hours(RoundingPer::Day)?)?;

    let batch = decide(&log, &catalog, &november_on("2026-0042")?)?;

    let split = billings(&batch)?;
    let amounts: Vec<String> = split.iter().map(|(_, b)| b.amount.to_string()).collect();
    assert_eq!(amounts, vec!["8.34", "8.33", "8.33"]);
    let billed: SignedDuration = split.iter().map(|(_, b)| b.billed).sum();
    assert_eq!(billed, mins(15));
    Ok(())
}

#[test]
fn marking_refuses_overlap_as_bill_does() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-10T09:00", "2026-11-10T10:00", on(WEBSHOP)?)?;
    _ = log.log("2026-11-10T09:30", "2026-11-10T10:30", on(WEBSHOP)?)?;
    let catalog = catalog(Rounding::NONE)?;

    let error = refused(&log, &catalog, &november_on("2026-0042")?)?;

    assert!(error.is_code(BILL_OVERLAP));
    Ok(())
}

#[test]
fn marking_without_anything_to_bill_has_nothing_to_do() -> TestResult {
    let mut log = Log::new(1)?;
    let entry = log.log("2026-11-10T09:00", "2026-11-10T10:00", on(WEBSHOP)?)?;
    mark_billed(
        &mut log,
        entry,
        "2026-12-01T09:00",
        one_hour_on("2026-0041")?,
    )?;
    let catalog = catalog(Rounding::NONE)?;

    let error = refused(&log, &catalog, &november_on("2026-0042")?)?;

    assert_eq!(error.exit_code(), ExitCode::NothingToDo);
    assert_eq!(error.to_string(), "nothing to bill for acme in the period");
    Ok(())
}

#[test]
fn invoice_reference_used_by_client_is_refused() -> TestResult {
    let mut log = Log::new(1)?;
    let earlier = log.log("2026-10-10T09:00", "2026-10-10T10:00", on(WEBSHOP)?)?;
    mark_billed(
        &mut log,
        earlier,
        "2026-11-01T09:00",
        one_hour_on("2026-0042")?,
    )?;
    _ = log.log("2026-11-10T09:00", "2026-11-10T10:00", on(WEBSHOP)?)?;
    let catalog = catalog(Rounding::NONE)?;

    let error = refused(&log, &catalog, &november_on("2026-0042")?)?;

    assert!(error.is_code(INVOICE_USED));
    assert_eq!(
        error.to_string(),
        "acme's invoice 2026-0042 is marked already, on 1 entry"
    );
    assert_eq!(
        error.hint(),
        Some("give the new invoice a reference of its own")
    );
    Ok(())
}

#[test]
fn invoice_reference_of_other_client_is_no_hindrance() -> TestResult {
    let mut log = Log::new(1)?;
    let beta = log.log("2026-10-10T09:00", "2026-10-10T10:00", on(BETA_SITE)?)?;
    mark_billed(
        &mut log,
        beta,
        "2026-11-01T09:00",
        one_hour_on("2026-0042")?,
    )?;
    _ = log.log("2026-11-10T09:00", "2026-11-10T10:00", on(WEBSHOP)?)?;
    let catalog = catalog(Rounding::NONE)?;

    let batch = decide(&log, &catalog, &november_on("2026-0042")?)?;

    assert_eq!(batch.events.len(), 1);
    Ok(())
}

#[test]
fn marking_for_unknown_client_is_refused() -> TestResult {
    let log = Log::new(1)?;
    let catalog = catalog(Rounding::NONE)?;
    let command = Command::MarkBilled(MarkBilled {
        client: "gamma".parse()?,
        start: None,
        end: None,
        invoice: "2026-0042".parse()?,
    });

    let error = refused(&log, &catalog, &command)?;

    assert_eq!(error.to_string(), "pace knows no client gamma");
    Ok(())
}
