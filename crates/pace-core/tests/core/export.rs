//! The export: one row per entry that starts in the period and passes the
//! filters, unsplit, with its times, its project, client and role, and the
//! frozen or the current price.

use std::collections::BTreeSet;

use jiff::SignedDuration;
use pace_core::{
    DeterminedBy, ExportEntry, ExportInput, HoldSource, PauseKind, ReportFilter, RoundingPer,
    export_entries,
};
use pace_time::TimeRange;

use crate::{
    TestResult,
    bill::{BETA_SITE, LATER, WEBSHOP, catalog, eur, mark_billed, one_hour_on, quarter_hours},
    report::{Fallible, Fields, Log, berlin, days, mins, on, zone},
};

const GARDEN: &str = "0193a7c4-0000-7000-8000-000000000002";

/// The export of `log` over `period` with `filter`, seen at `now`, with
/// acme rounding up to a quarter hour per entry.
fn export(
    log: &Log,
    period: &TimeRange,
    filter: &ReportFilter,
    now: &str,
) -> Fallible<Vec<ExportEntry>> {
    let catalog = catalog(quarter_hours(RoundingPer::Entry)?)?;
    Ok(export_entries(&ExportInput {
        state: &log.state(),
        catalog: &catalog,
        period,
        filter,
        zone: &zone()?,
        now: &berlin(now)?,
    })?)
}

/// The export of all of `log`, unfiltered, at `LATER`.
fn export_all(log: &Log) -> Fallible<Vec<ExportEntry>> {
    export(
        log,
        &TimeRange::everything(),
        &ReportFilter::default(),
        LATER,
    )
}

fn only(entries: Vec<ExportEntry>) -> Fallible<ExportEntry> {
    let [entry]: [ExportEntry; 1] = entries
        .try_into()
        .map_err(|entries| format!("one entry, not {entries:?}"))?;
    Ok(entry)
}

#[test]
fn unbilled_billable_entry_takes_rate_of_start_day_without_rounding() -> TestResult {
    let mut log = Log::new(1)?;
    let id = log.log("2026-11-20T09:00", "2026-11-20T09:50", on(WEBSHOP)?)?;

    let entry = only(export_all(&log)?)?;

    assert_eq!(entry.id, id);
    assert_eq!(entry.net, mins(50));
    assert!(entry.billable);
    assert_eq!(entry.invoice, None);
    let price = entry.price.ok_or("a price")?;
    assert_eq!(price.rate, eur("120")?);
    assert_eq!(price.amount, eur("100")?);
    Ok(())
}

#[test]
fn billed_entry_carries_frozen_rate_and_amount() -> TestResult {
    let mut log = Log::new(1)?;
    let id = log.log("2026-11-20T09:00", "2026-11-20T09:50", on(WEBSHOP)?)?;
    mark_billed(&mut log, id, "2026-12-01T09:00", one_hour_on("2026-0042")?)?;

    let entry = only(export_all(&log)?)?;

    assert_eq!(entry.invoice, Some("2026-0042".parse()?));
    let price = entry.price.ok_or("a price")?;
    assert_eq!(price.rate, eur("100")?);
    assert_eq!(price.amount, eur("100")?);
    Ok(())
}

#[test]
fn entry_names_project_client_role_and_its_own_fields() -> TestResult {
    let mut log = Log::new(1)?;
    let fields = Fields {
        note: Some("Fix the checkout".to_owned()),
        tags: BTreeSet::from(["deep".parse()?]),
        determined_by: Some(DeterminedBy::Others),
        ..on(WEBSHOP)?
    };
    let id = log.begin("2026-11-20T09:00", fields)?;
    log.hold(
        id,
        "2026-11-20T10:00",
        "2026-11-20T10:20",
        HoldSource::Outer,
    )?;
    log.pause(id, "2026-11-20T11:00", "2026-11-20T11:10", PauseKind::Break)?;
    log.end(id, "2026-11-20T12:00")?;

    let entry = only(export_all(&log)?)?;

    assert_eq!(entry.short, log.state().short_id(id));
    assert_eq!(entry.start, berlin("2026-11-20T09:00")?);
    assert_eq!(entry.end, Some(berlin("2026-11-20T12:00")?));
    assert!(!entry.running);
    assert_eq!(entry.gross, SignedDuration::from_hours(3));
    assert_eq!(entry.held, mins(20));
    assert_eq!(entry.paused, mins(10));
    assert_eq!(entry.net, mins(150));
    assert_eq!(entry.project, Some("acme-webshop".parse()?));
    assert_eq!(entry.project_id, Some(WEBSHOP.parse()?));
    assert_eq!(entry.client, Some("acme".parse()?));
    assert_eq!(entry.role, Some("work".parse()?));
    assert_eq!(entry.tags, BTreeSet::from(["deep".parse()?]));
    assert_eq!(entry.note.as_deref(), Some("Fix the checkout"));
    assert_eq!(entry.determined_by, Some(DeterminedBy::Others));
    Ok(())
}

#[test]
fn running_entry_counts_up_to_now_without_end() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.begin("2026-11-20T09:00", on(WEBSHOP)?)?;

    let entry = only(export(
        &log,
        &TimeRange::everything(),
        &ReportFilter::default(),
        "2026-11-20T09:30",
    )?)?;

    assert!(entry.running);
    assert_eq!(entry.end, None);
    assert_eq!(entry.net, mins(30));
    assert_eq!(entry.price.ok_or("a price")?.amount, eur("60")?);
    Ok(())
}

#[test]
fn entry_without_billable_time_or_rate_has_no_price() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(GARDEN)?)?;
    _ = log.log("2025-12-01T09:00", "2025-12-01T10:00", on(WEBSHOP)?)?;

    let entries = export_all(&log)?;

    let shown: Vec<(bool, bool)> = entries
        .iter()
        .map(|entry| (entry.billable, entry.price.is_some()))
        .collect();
    assert_eq!(shown, [(true, false), (false, false)]);
    Ok(())
}

#[test]
fn export_takes_entries_that_start_in_period_whole() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-10-31T23:00", "2026-11-01T01:00", on(WEBSHOP)?)?;
    let inside = log.log("2026-11-30T23:00", "2026-12-01T01:00", on(WEBSHOP)?)?;

    let entries = export(
        &log,
        &days("2026-11-01", "2026-12-01")?,
        &ReportFilter::default(),
        LATER,
    )?;

    let entry = only(entries)?;
    assert_eq!(entry.id, inside);
    assert_eq!(entry.net, SignedDuration::from_hours(2));
    Ok(())
}

#[test]
fn export_keeps_entries_that_pass_filter() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    let beta = log.log("2026-11-20T10:00", "2026-11-20T11:00", on(BETA_SITE)?)?;
    let filter = ReportFilter {
        clients: BTreeSet::from(["beta".parse()?]),
        ..ReportFilter::default()
    };

    let entries = export(&log, &TimeRange::everything(), &filter, LATER)?;

    assert_eq!(only(entries)?.id, beta);
    Ok(())
}
