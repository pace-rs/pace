//! The data checks of `pace doctor`: the entries, the structure and the
//! billing, each a function on the state, the catalog and the events.

use std::collections::BTreeMap;

use pace_core::{
    Bill, BillInput, Billing, Catalog, Change, Client, DataArea, DataFinding, DoctorInput,
    EntryChanges, EventKind, FindingSeverity, ProjectRef, Rate, Rates, RecordedEvent, Rounding,
    RoundingMode, RoundingPer, State, billing_findings, data_findings, entry_findings,
    structure_findings,
};

use crate::{
    TestResult,
    bill::eur,
    report::{Fallible, Fields, LATER, Log, berlin, days, hours, mins, on, record, role, zone},
};

const WORK: &str = "0193a7c4-0000-7000-8000-00000000000a";
const WEBSHOP: &str = "0193a7c4-0000-7000-8000-000000000001";
const GARDEN: &str = "0193a7c4-0000-7000-8000-000000000002";

/// The client `acme` with `hourly` EUR an hour from 2026-11-01, if given.
fn acme(hourly: Option<&str>) -> Fallible<Client> {
    let rates = hourly
        .map(|hourly| {
            Ok::<_, Box<dyn std::error::Error>>(Rate {
                hourly: eur(hourly)?,
                from: "2026-11-01".parse()?,
            })
        })
        .transpose()?;
    Ok(Client {
        key: "acme".parse()?,
        name: "ACME GmbH".to_owned(),
        currency: "EUR".parse()?,
        rounding: Rounding::NONE,
        rates: Rates::new(rates.into_iter().collect())?,
        project_rates: BTreeMap::new(),
    })
}

/// Work (35 h a week); the billable webshop of acme at `hourly` EUR an
/// hour; the garden without a client, not billable.
fn catalog_at(hourly: Option<&str>) -> Fallible<Catalog> {
    let mut webshop = record(WEBSHOP, "acme-webshop", "work", Some("acme"))?;
    webshop.billable = true;
    Ok(Catalog::new(
        vec![role(WORK, "work", Some(35))?],
        vec![acme(hourly)?],
        vec![webshop, record(GARDEN, "garden", "work", None)?],
    )?)
}

/// The findings of `check` on `events` and `catalog`, seen at [`LATER`].
fn check(
    events: &[RecordedEvent],
    catalog: &Catalog,
    check: fn(&DoctorInput<'_>) -> Vec<DataFinding>,
) -> Fallible<Vec<DataFinding>> {
    let state = State::from_events(events).resolve_projects(catalog);
    Ok(check(&DoctorInput {
        state: &state,
        catalog,
        events,
        zone: &zone()?,
        now: &berlin(LATER)?,
    }))
}

/// The messages of `findings`.
fn messages(findings: &[DataFinding]) -> Vec<&str> {
    findings
        .iter()
        .map(|finding| finding.message.as_str())
        .collect()
}

/// A note, for an entry's fields.
fn noted(note: &str) -> Fields {
    Fields {
        note: Some(note.to_owned()),
        ..Fields::default()
    }
}

/// The events of `logs`, in one list, as a sync gathers them.
fn synced(logs: &[&Log]) -> Vec<RecordedEvent> {
    logs.iter()
        .flat_map(|log| log.events().iter().cloned())
        .collect()
}

#[test]
fn ended_entries_apart_on_known_projects_have_no_findings() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(GARDEN)?)?;
    _ = log.log("2026-11-20T10:00", "2026-11-20T11:00", on(WEBSHOP)?)?;

    let findings = check(log.events(), &catalog_at(Some("100"))?, data_findings)?;

    assert_eq!(findings, []);
    Ok(())
}

#[test]
fn two_open_entries_are_one_error_that_names_both() -> TestResult {
    let mut laptop = Log::new(1)?;
    let mut desktop = Log::new(2)?;
    _ = laptop.begin("2026-11-20T09:00", noted("Write offer"))?;
    _ = desktop.begin("2026-11-20T09:30", noted("Call Anna"))?;
    let events = synced(&[&laptop, &desktop]);

    let findings = check(&events, &Catalog::default(), entry_findings)?;

    assert_eq!(findings.len(), 1, "{findings:?}");
    let finding = &findings[0];
    assert_eq!(finding.severity, FindingSeverity::Error);
    assert_eq!(finding.area, DataArea::Entries);
    assert!(
        finding.message.starts_with("2 entries are open at once: ")
            && finding
                .message
                .contains("\"Write offer\" (since 2026-11-20 09:00)")
            && finding
                .message
                .contains("\"Call Anna\" (since 2026-11-20 09:30)"),
        "{}",
        finding.message
    );
    assert_eq!(
        finding.fix.as_deref(),
        Some("end all but one with `pace end <id>`")
    );
    Ok(())
}

#[test]
fn overlap_that_nothing_allows_is_warning_with_shared_time() -> TestResult {
    let mut log = Log::new(1)?;
    let first = log.log("2026-11-20T09:00", "2026-11-20T10:00", noted("Offer"))?;
    _ = log.log("2026-11-20T09:30", "2026-11-20T11:00", noted("Call"))?;
    let state = log.state();

    let findings = check(log.events(), &Catalog::default(), entry_findings)?;

    assert_eq!(findings.len(), 1, "{findings:?}");
    let finding = &findings[0];
    assert_eq!(finding.severity, FindingSeverity::Warning);
    let short = state.short_id(first);
    assert_eq!(finding.entry.as_deref(), Some(short.as_str()));
    assert!(
        finding
            .message
            .ends_with(" overlap: they share 09:30–10:00 on 2026-11-20"),
        "{}",
        finding.message
    );
    assert_eq!(
        finding.fix.as_deref(),
        Some(
            "move one with `pace adjust <id> --start` or `--end`, or allow the overlap with \
             `pace adjust <id> --allow-overlap`"
        )
    );
    Ok(())
}

#[test]
fn overlap_allowed_by_entry_or_project_is_no_finding() -> TestResult {
    let mut catalog_log = Log::new(1)?;
    _ = catalog_log.log("2026-11-20T09:00", "2026-11-20T10:00", on(GARDEN)?)?;
    _ = catalog_log.log("2026-11-20T09:30", "2026-11-20T11:00", noted("Call"))?;
    let mut garden = record(GARDEN, "garden", "work", None)?;
    garden.allow_overlap = true;
    let catalog = Catalog::new(vec![role(WORK, "work", None)?], Vec::new(), vec![garden])?;

    let mut flag_log = Log::new(2)?;
    let entry = flag_log.log("2026-11-20T09:00", "2026-11-20T10:00", noted("Offer"))?;
    _ = flag_log.log("2026-11-20T09:30", "2026-11-20T11:00", noted("Call"))?;
    flag_log.record(
        "2026-11-20T12:00",
        EventKind::EntryAdjusted {
            entry,
            changes: EntryChanges {
                allow_overlap: true,
                ..EntryChanges::default()
            },
        },
    )?;

    assert_eq!(check(catalog_log.events(), &catalog, entry_findings)?, []);
    assert_eq!(
        check(flag_log.events(), &Catalog::default(), entry_findings)?,
        []
    );
    Ok(())
}

/// An `entry.adjusted` of `entry` that sets its note to `note`.
fn renote(entry: pace_core::EntryId, note: &str) -> EventKind {
    EventKind::EntryAdjusted {
        entry,
        changes: EntryChanges {
            note: Change::Set(note.to_owned()),
            ..EntryChanges::default()
        },
    }
}

#[test]
fn field_changed_on_two_devices_names_both_values_and_the_later_one() -> TestResult {
    let mut laptop = Log::new(1)?;
    let mut desktop = Log::new(2)?;
    let entry = laptop.log("2026-11-20T09:00", "2026-11-20T10:00", noted("Offer"))?;
    laptop.record("2026-11-20T12:00", renote(entry, "Offer for ACME"))?;
    desktop.record("2026-11-20T12:30", renote(entry, "Offer for Beta"))?;
    let events = synced(&[&laptop, &desktop]);
    let state = State::from_events(&events);
    let (laptop_id, desktop_id) = (laptop.events()[0].device, desktop.events()[0].device);

    let findings = check(&events, &Catalog::default(), entry_findings)?;

    assert_eq!(findings.len(), 1, "{findings:?}");
    let finding = &findings[0];
    let short = state.short_id(entry);
    assert_eq!(finding.severity, FindingSeverity::Warning);
    assert_eq!(finding.entry.as_deref(), Some(short.as_str()));
    assert_eq!(
        finding.message,
        format!(
            "the note of {short} changed on 2 devices: \"Offer for ACME\" on device \
             {laptop_id}, \"Offer for Beta\" on device {desktop_id}; \"Offer for Beta\" counts, \
             the later change"
        )
    );
    assert_eq!(
        finding.fix.as_deref(),
        Some(
            "pace doesn't record what a device had seen, so this can be no conflict; if another \
             value is right, set it with `pace edit <id>`"
                .replace("<id>", &short)
                .as_str()
        )
    );
    Ok(())
}

#[test]
fn ends_on_two_devices_are_changes_of_end() -> TestResult {
    let mut laptop = Log::new(1)?;
    let mut desktop = Log::new(2)?;
    let entry = laptop.begin("2026-11-20T09:00", noted("Offer"))?;
    laptop.end(entry, "2026-11-20T10:00")?;
    desktop.end(entry, "2026-11-20T10:45")?;
    let events = synced(&[&laptop, &desktop]);

    let findings = check(&events, &Catalog::default(), entry_findings)?;

    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(
        findings[0].message.contains("2026-11-20 10:00 on device")
            && findings[0]
                .message
                .ends_with("; 2026-11-20 10:45 counts, the later change"),
        "{}",
        findings[0].message
    );
    Ok(())
}

#[test]
fn changes_of_one_device_or_undone_ones_are_no_conflict() -> TestResult {
    let mut laptop = Log::new(1)?;
    let mut desktop = Log::new(2)?;
    let entry = laptop.log("2026-11-20T09:00", "2026-11-20T10:00", noted("Offer"))?;
    laptop.record("2026-11-20T12:00", renote(entry, "Offer for ACME"))?;
    laptop.record("2026-11-20T12:10", renote(entry, "Offer for ACME GmbH"))?;
    desktop.record("2026-11-20T12:30", renote(entry, "Offer for Beta"))?;
    let batch = desktop.events().last().ok_or("the desktop's change")?.batch;
    desktop.record("2026-11-20T12:31", EventKind::BatchUndone { batch })?;
    let events = synced(&[&laptop, &desktop]);

    assert_eq!(check(&events, &Catalog::default(), entry_findings)?, []);
    Ok(())
}

#[test]
fn entries_on_project_key_without_record_are_one_finding_per_key() -> TestResult {
    let mut log = Log::new(1)?;
    let key = || -> Fallible<Fields> {
        Ok(Fields {
            project: Some(ProjectRef::Key("webshop".parse()?)),
            ..Fields::default()
        })
    };
    let first = log.log("2026-11-20T09:00", "2026-11-20T10:00", key()?)?;
    let second = log.log("2026-11-21T09:00", "2026-11-21T10:00", key()?)?;
    _ = log.log("2026-11-22T09:00", "2026-11-22T10:00", on(GARDEN)?)?;
    let state = log.state();

    let findings = check(log.events(), &catalog_at(Some("100"))?, entry_findings)?;

    assert_eq!(
        messages(&findings),
        [format!(
            "2 entries name the project `webshop`, which has no record: {}, {}",
            state.short_id(first),
            state.short_id(second)
        )]
    );
    assert_eq!(findings[0].severity, FindingSeverity::Warning);
    assert_eq!(
        findings[0].fix.as_deref(),
        Some(
            "give it a record with `pace project init` in the project's directory, or with \
             `pace project add webshop`"
        )
    );
    Ok(())
}

#[test]
fn billable_project_without_client_is_warning() -> TestResult {
    let mut garden = record(GARDEN, "garden", "work", None)?;
    garden.billable = true;
    let catalog = Catalog::new(vec![role(WORK, "work", None)?], Vec::new(), vec![garden])?;

    let findings = structure_findings(&catalog);

    assert_eq!(
        messages(&findings),
        ["the project `garden` is billable, but has no client"]
    );
    assert_eq!(findings[0].area, DataArea::Structure);
    assert_eq!(findings[0].severity, FindingSeverity::Warning);
    assert_eq!(
        findings[0].fix.as_deref(),
        Some("give it a client, or make it not billable, with `pace project edit garden`")
    );
    Ok(())
}

#[test]
fn targets_above_week_are_warning_with_their_sum() -> TestResult {
    let catalog = Catalog::new(
        vec![
            role(WORK, "work", Some(120))?,
            role("0193a7c4-0000-7000-8000-00000000000b", "health", Some(60))?,
        ],
        Vec::new(),
        Vec::new(),
    )?;

    let findings = structure_findings(&catalog);

    assert_eq!(
        messages(&findings),
        ["the weekly targets of the roles in use add up to 180h, more than the 168h of a week"]
    );
    assert_eq!(
        findings[0].fix.as_deref(),
        Some("lower a target with `pace role edit`")
    );
    Ok(())
}

/// One hour of the webshop on 2026-11-20, billed at 100 EUR on
/// `2026-0042`.
fn billed_hour() -> Fallible<(Log, pace_core::EntryId)> {
    let mut log = Log::new(1)?;
    let entry = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    log.record(
        "2026-12-01T09:00",
        EventKind::BillingMarked {
            entry,
            billing: Billing {
                invoice: "2026-0042".parse()?,
                net: hours(1),
                billed: hours(1),
                rate: eur("100")?,
                amount: eur("100")?,
            },
        },
    )?;
    Ok((log, entry))
}

#[test]
fn billed_entry_whose_amount_differs_today_shows_both_amounts() -> TestResult {
    let (log, entry) = billed_hour()?;
    let state = log.state();

    let findings = check(log.events(), &catalog_at(Some("120"))?, billing_findings)?;

    let short = state.short_id(entry);
    assert_eq!(
        messages(&findings),
        [format!(
            "{short} went on the invoice 2026-0042 with 100.00 EUR; today's rates make it \
             120.00 EUR"
        )]
    );
    assert_eq!(findings[0].area, DataArea::Billing);
    assert_eq!(findings[0].entry.as_deref(), Some(short.as_str()));
    assert_eq!(
        findings[0].fix.as_deref(),
        Some(
            "a written invoice keeps its amounts; if the rate was wrong, mend it with `pace \
             client edit acme`"
        )
    );
    Ok(())
}

#[test]
fn billed_entry_at_todays_rate_is_no_finding() -> TestResult {
    let (log, _) = billed_hour()?;

    assert_eq!(
        check(log.events(), &catalog_at(Some("100"))?, billing_findings)?,
        []
    );
    Ok(())
}

#[test]
fn share_of_day_line_billed_at_unchanged_rate_is_no_finding() -> TestResult {
    let mut catalog_client = acme(Some("100"))?;
    catalog_client.rounding = Rounding::new(RoundingPer::Day, mins(1), RoundingMode::Up)?;
    let mut webshop = record(WEBSHOP, "acme-webshop", "work", Some("acme"))?;
    webshop.billable = true;
    let catalog = Catalog::new(
        vec![role(WORK, "work", Some(35))?],
        vec![catalog_client],
        vec![webshop],
    )?;
    let mut log = Log::new(1)?;
    for (start, end) in [
        ("2026-11-24T09:00", "2026-11-24T09:07"),
        ("2026-11-24T10:00", "2026-11-24T10:07"),
        ("2026-11-24T11:00", "2026-11-24T11:07"),
    ] {
        _ = log.log(start, end, on(WEBSHOP)?)?;
    }
    let client = catalog.client(&"acme".parse()?).ok_or("acme")?;
    let bill = Bill::build(&BillInput {
        state: &log.state(),
        catalog: &catalog,
        client,
        period: &days("2026-11-01", "2026-12-01")?,
        zone: &zone()?,
        now: &berlin(LATER)?,
    })?;
    let billings = bill.billings(&"2026-0042".parse()?);
    // The day's 35.00 EUR split in three: one share is a cent short of
    // 7 minutes at 100 EUR an hour.
    let amounts: Vec<String> = billings
        .iter()
        .map(|(_, billing)| billing.amount.to_string())
        .collect();
    assert_eq!(amounts, ["11.67", "11.67", "11.66"]);
    for (entry, billing) in billings {
        log.record(
            "2026-12-01T09:00",
            EventKind::BillingMarked { entry, billing },
        )?;
    }

    assert_eq!(check(log.events(), &catalog, billing_findings)?, []);
    Ok(())
}

#[test]
fn billed_entry_without_rate_today_names_its_day() -> TestResult {
    let (log, entry) = billed_hour()?;
    let state = log.state();

    let findings = check(log.events(), &catalog_at(None)?, billing_findings)?;

    assert_eq!(
        messages(&findings),
        [format!(
            "{} went on the invoice 2026-0042 with 100.00 EUR; today no rate holds on \
             2026-11-20",
            state.short_id(entry)
        )]
    );
    Ok(())
}

#[test]
fn billable_entry_before_first_rate_of_its_client_is_warning() -> TestResult {
    let mut log = Log::new(1)?;
    let early = log.log("2026-10-20T09:00", "2026-10-20T10:00", on(WEBSHOP)?)?;
    _ = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    _ = log.log("2026-10-21T09:00", "2026-10-21T10:00", on(GARDEN)?)?;
    let state = log.state();

    let findings = check(log.events(), &catalog_at(Some("100"))?, billing_findings)?;

    let short = state.short_id(early);
    assert_eq!(
        messages(&findings),
        [format!(
            "{short} is billable, but no rate of the client `acme` holds on its day 2026-10-20"
        )]
    );
    assert_eq!(findings[0].severity, FindingSeverity::Warning);
    assert_eq!(
        findings[0].fix.as_deref(),
        Some("add an earlier rate with `pace client edit acme`")
    );
    Ok(())
}

#[test]
fn data_findings_come_by_area() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-10-20T09:00", "2026-10-20T10:00", on(WEBSHOP)?)?;
    _ = log.log("2026-10-20T09:30", "2026-10-20T10:30", noted("Call"))?;
    let mut webshop = record(WEBSHOP, "acme-webshop", "work", Some("acme"))?;
    webshop.billable = true;
    let catalog = Catalog::new(
        vec![role(WORK, "work", Some(200))?],
        vec![acme(Some("100"))?],
        vec![webshop],
    )?;

    let findings = check(log.events(), &catalog, data_findings)?;

    let areas: Vec<DataArea> = findings.iter().map(|finding| finding.area).collect();
    assert_eq!(
        areas,
        [DataArea::Entries, DataArea::Structure, DataArea::Billing]
    );
    assert_eq!(
        findings
            .iter()
            .map(|finding| finding.area.name())
            .collect::<Vec<_>>(),
        ["entries", "structure", "billing"]
    );
    Ok(())
}
