//! The report of `reflect`: time per role, project and day, split at
//! midnight in the report's zone, targets, budgets, holds, pauses, catches,
//! who decided, overlaps and the filters.

use std::collections::BTreeSet;

use jiff::{SignedDuration, ZonedDateTime, civil::Date, tz::TimeZone};
use pace_core::{
    Catalog, Client, DeterminedBy, DeviceId, EntryId, Event, EventKind, Hlc, HoldSource, Ids,
    PauseKind, ProjectId, ProjectRecord, ProjectRef, Rates, RecordedEvent, Report, ReportFilter,
    ReportInput, ReportWarning, Role, RoleId, RoleKind, Rounding, State, testing::SequentialIds,
};
use pace_time::TimeRange;

use crate::TestResult;

type Fallible<T> = Result<T, Box<dyn std::error::Error>>;

fn berlin(time: &str) -> Fallible<ZonedDateTime> {
    Ok(format!("{time}[Europe/Berlin]").parse()?)
}

fn zone() -> Fallible<TimeZone> {
    Ok(TimeZone::get("Europe/Berlin")?)
}

fn date(text: &str) -> Fallible<Date> {
    Ok(text.parse()?)
}

/// The days from `first` up to, not including, `after`, in Berlin.
fn days(first: &str, after: &str) -> Fallible<TimeRange> {
    Ok(TimeRange::new(
        Some(berlin(&format!("{first}T00:00"))?),
        Some(berlin(&format!("{after}T00:00"))?),
    )?)
}

const WORK: &str = "0193a7c4-0000-7000-8000-00000000000a";
const HEALTH: &str = "0193a7c4-0000-7000-8000-00000000000b";
const WEBSHOP: &str = "0193a7c4-0000-7000-8000-000000000001";
const GARDEN: &str = "0193a7c4-0000-7000-8000-000000000002";

fn role_id(id: &str) -> Fallible<RoleId> {
    Ok(id.parse()?)
}

fn project(id: &str) -> Fallible<ProjectRef> {
    Ok(ProjectRef::Id(id.parse::<ProjectId>()?))
}

fn role(id: &str, key: &str, weekly_hours: Option<i64>) -> Fallible<Role> {
    Ok(Role {
        id: id.parse()?,
        key: key.parse()?,
        kind: RoleKind::Area,
        name: key.to_owned(),
        purpose: None,
        weekly_target: weekly_hours.map(SignedDuration::from_hours),
        archived: None,
    })
}

fn record(id: &str, key: &str, role: &str, client: Option<&str>) -> Fallible<ProjectRecord> {
    Ok(ProjectRecord {
        id: id.parse()?,
        key: key.parse()?,
        name: key.to_owned(),
        client: client.map(str::parse).transpose()?,
        role: Some(role.parse()?),
        billable: false,
        budget: None,
        starts_on: None,
        ends_on: None,
        done_when: None,
        allow_overlap: false,
        archived: None,
    })
}

/// Work (35 h a week) with the webshop of acme, health (7 h a week) with
/// the garden.
fn catalog() -> Fallible<Catalog> {
    catalog_with(Some(35), Some(7), None)
}

fn catalog_with(
    work_hours: Option<i64>,
    health_hours: Option<i64>,
    webshop_budget: Option<i64>,
) -> Fallible<Catalog> {
    let acme = Client {
        key: "acme".parse()?,
        name: "ACME GmbH".to_owned(),
        currency: "EUR".parse()?,
        rounding: Rounding::NONE,
        rates: Rates::new(Vec::new())?,
        project_rates: std::collections::BTreeMap::new(),
    };
    let mut webshop = record(WEBSHOP, "acme-webshop", "work", Some("acme"))?;
    webshop.budget = webshop_budget.map(SignedDuration::from_hours);
    Ok(Catalog::new(
        vec![
            role(WORK, "work", work_hours)?,
            role(HEALTH, "health", health_hours)?,
        ],
        vec![acme],
        vec![webshop, record(GARDEN, "garden", "health", None)?],
    )?)
}

/// What an entry is begun or logged with.
#[derive(Default)]
struct Fields {
    project: Option<ProjectRef>,
    tags: BTreeSet<pace_core::Tag>,
    role: Option<RoleId>,
    determined_by: Option<DeterminedBy>,
}

fn on(project_id: &str) -> Fallible<Fields> {
    Ok(Fields {
        project: Some(project(project_id)?),
        ..Fields::default()
    })
}

/// The log of one device, one batch per event, IDs from `SequentialIds`.
struct Log {
    ids: SequentialIds,
    device: DeviceId,
    counter: i32,
    events: Vec<RecordedEvent>,
}

impl Log {
    fn new(seed: u16) -> Fallible<Self> {
        let ids = SequentialIds::starting_at(&berlin("2026-10-01T00:00")?, seed);
        let device = DeviceId::try_from(ids.next_id())?;
        Ok(Self {
            ids,
            device,
            counter: 0,
            events: Vec::new(),
        })
    }

    fn record(&mut self, at: &str, kind: EventKind) -> Fallible<()> {
        self.counter += 1;
        let moment = berlin(at)?;
        self.events.push(RecordedEvent {
            device: self.device,
            batch: self.ids.next_id().try_into()?,
            hlc: Hlc::new(moment.timestamp(), u32::try_from(self.counter)?),
            event: Event {
                id: self.ids.next_id().try_into()?,
                at: moment.clone(),
                about: moment,
                kind,
            },
        });
        Ok(())
    }

    fn begin(&mut self, start: &str, fields: Fields) -> Fallible<EntryId> {
        let entry = EntryId::try_from(self.ids.next_id())?;
        self.record(
            start,
            EventKind::EntryBegun {
                entry,
                project: fields.project,
                note: None,
                tags: fields.tags,
                role: fields.role,
                determined_by: fields.determined_by,
                billable: None,
                allow_overlap: false,
            },
        )?;
        Ok(entry)
    }

    fn log(&mut self, start: &str, end: &str, fields: Fields) -> Fallible<EntryId> {
        let entry = EntryId::try_from(self.ids.next_id())?;
        self.record(
            start,
            EventKind::EntryLogged {
                entry,
                end: berlin(end)?,
                project: fields.project,
                note: None,
                tags: fields.tags,
                role: fields.role,
                determined_by: fields.determined_by,
                billable: None,
                allow_overlap: false,
            },
        )?;
        Ok(entry)
    }

    fn hold(&mut self, entry: EntryId, from: &str, to: &str, source: HoldSource) -> Fallible<()> {
        self.hold_for(entry, from, to, source, None)
    }

    fn hold_for(
        &mut self,
        entry: EntryId,
        from: &str,
        to: &str,
        source: HoldSource,
        reason: Option<&str>,
    ) -> Fallible<()> {
        let reason = reason.map(ToOwned::to_owned);
        self.record(
            from,
            EventKind::EntryHeld {
                entry,
                source,
                reason,
            },
        )?;
        self.record(to, EventKind::EntryResumed { entry })
    }

    fn pause(&mut self, entry: EntryId, from: &str, to: &str, kind: PauseKind) -> Fallible<()> {
        self.record(
            from,
            EventKind::EntryPaused {
                entry,
                kind,
                reason: None,
            },
        )?;
        self.record(to, EventKind::EntryResumed { entry })
    }

    fn catch(&mut self, entry: EntryId, at: &str, text: &str) -> Fallible<()> {
        self.record(
            at,
            EventKind::CatchNoted {
                entry,
                text: text.to_owned(),
            },
        )
    }

    fn end(&mut self, entry: EntryId, at: &str) -> Fallible<()> {
        self.record(at, EventKind::EntryEnded { entry })
    }

    fn state(&self) -> State {
        State::from_events(&self.events)
    }
}

/// The report of `state` over `period`, seen at `now`, in Berlin.
fn report(
    state: &State,
    catalog: &Catalog,
    period: &TimeRange,
    filter: &ReportFilter,
    now: &str,
) -> Fallible<Report> {
    Ok(Report::build(&ReportInput {
        state,
        catalog,
        period,
        filter,
        now: &berlin(now)?,
        zone: &zone()?,
        all_time: None,
    }))
}

/// The report of `log` over `period` at `now`, unfiltered, with `catalog()`.
fn plain(log: &Log, period: &TimeRange, now: &str) -> Fallible<Report> {
    report(
        &log.state(),
        &catalog()?,
        period,
        &ReportFilter::default(),
        now,
    )
}

fn day_times(report: &Report) -> Vec<(Date, SignedDuration)> {
    report.days.iter().map(|day| (day.date, day.net)).collect()
}

const fn hours(hours: i64) -> SignedDuration {
    SignedDuration::from_hours(hours)
}

const fn mins(minutes: i64) -> SignedDuration {
    SignedDuration::from_mins(minutes)
}

const LATER: &str = "2026-12-31T00:00";

#[test]
fn entry_over_midnight_counts_one_hour_on_each_day() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-23T23:00", "2026-11-24T01:00", on(WEBSHOP)?)?;

    let report = plain(&log, &days("2026-11-23", "2026-11-25")?, LATER)?;

    assert_eq!(
        day_times(&report),
        vec![
            (date("2026-11-23")?, hours(1)),
            (date("2026-11-24")?, hours(1))
        ]
    );
    assert_eq!(report.tracked, hours(2));
    Ok(())
}

#[test]
fn day_of_autumn_time_change_holds_twenty_five_hours() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-10-24T23:00", "2026-10-26T01:00", on(WEBSHOP)?)?;

    let report = plain(&log, &days("2026-10-24", "2026-10-27")?, LATER)?;

    assert_eq!(
        day_times(&report),
        vec![
            (date("2026-10-24")?, hours(1)),
            (date("2026-10-25")?, hours(25)),
            (date("2026-10-26")?, hours(1))
        ]
    );
    assert_eq!(report.tracked, hours(27));
    Ok(())
}

#[test]
fn entry_counts_only_its_part_inside_period() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-23T22:00", "2026-11-24T03:00", on(WEBSHOP)?)?;

    let report = plain(&log, &days("2026-11-24", "2026-11-25")?, LATER)?;

    assert_eq!(day_times(&report), vec![(date("2026-11-24")?, hours(3))]);
    assert_eq!(report.tracked, hours(3));
    Ok(())
}

#[test]
fn day_without_time_shows_with_zero() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-24T09:00", "2026-11-24T10:00", on(WEBSHOP)?)?;

    let report = plain(&log, &days("2026-11-23", "2026-11-26")?, LATER)?;

    assert_eq!(
        day_times(&report),
        vec![
            (date("2026-11-23")?, SignedDuration::ZERO),
            (date("2026-11-24")?, hours(1)),
            (date("2026-11-25")?, SignedDuration::ZERO)
        ]
    );
    Ok(())
}

#[test]
fn running_entry_counts_up_to_now_and_is_marked() -> TestResult {
    let mut log = Log::new(1)?;
    let running = log.begin("2026-11-26T09:00", on(WEBSHOP)?)?;

    let report = plain(&log, &days("2026-11-26", "2026-11-27")?, "2026-11-26T11:30")?;

    assert_eq!(report.tracked, mins(150));
    assert_eq!(report.running, vec![running]);
    let webshop = report.projects.first().ok_or("the webshop's time")?;
    assert!(webshop.running);
    assert_eq!(webshop.net, mins(150));
    Ok(())
}

#[test]
fn holds_and_pauses_are_not_net_time_of_their_day() -> TestResult {
    let mut log = Log::new(1)?;
    let entry = log.begin("2026-11-24T09:00", on(WEBSHOP)?)?;
    log.hold(
        entry,
        "2026-11-24T10:00",
        "2026-11-24T10:30",
        HoldSource::Outer,
    )?;
    log.pause(
        entry,
        "2026-11-24T12:00",
        "2026-11-24T13:00",
        PauseKind::Lunch,
    )?;
    log.end(entry, "2026-11-24T17:00")?;

    let report = plain(&log, &days("2026-11-24", "2026-11-25")?, LATER)?;

    assert_eq!(day_times(&report), vec![(date("2026-11-24")?, mins(390))]);
    Ok(())
}

#[test]
fn time_goes_to_effective_role_then_project() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-24T09:00", "2026-11-24T12:00", on(WEBSHOP)?)?;
    _ = log.log(
        "2026-11-24T13:00",
        "2026-11-24T14:00",
        Fields {
            role: Some(role_id(HEALTH)?),
            ..on(WEBSHOP)?
        },
    )?;
    _ = log.log("2026-11-24T18:00", "2026-11-24T20:00", on(GARDEN)?)?;

    let report = plain(&log, &days("2026-11-24", "2026-11-25")?, LATER)?;

    let roles: Vec<(Option<RoleId>, SignedDuration)> = report
        .roles
        .iter()
        .map(|time| (time.role, time.net))
        .collect();
    assert_eq!(
        roles,
        vec![
            (Some(role_id(HEALTH)?), hours(3)),
            (Some(role_id(WORK)?), hours(3))
        ]
    );
    let projects: Vec<(ProjectRef, SignedDuration)> = report
        .projects
        .iter()
        .map(|time| (time.project.clone(), time.net))
        .collect();
    assert_eq!(
        projects,
        vec![(project(WEBSHOP)?, hours(4)), (project(GARDEN)?, hours(2))]
    );
    Ok(())
}

#[test]
fn entries_without_project_form_back_flow() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-24T09:00", "2026-11-24T10:00", on(WEBSHOP)?)?;
    _ = log.log("2026-11-24T11:00", "2026-11-24T11:45", Fields::default())?;

    let report = plain(&log, &days("2026-11-24", "2026-11-25")?, LATER)?;

    assert_eq!(report.back_flow.net, mins(45));
    assert!(!report.back_flow.running);
    assert_eq!(report.projects.len(), 1);
    let no_role = report.roles.iter().find(|time| time.role.is_none());
    assert_eq!(no_role.map(|time| time.net), Some(mins(45)));
    Ok(())
}

#[test]
fn weekly_target_scales_to_one_day() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-24T09:00", "2026-11-24T13:00", on(WEBSHOP)?)?;

    let report = plain(&log, &days("2026-11-24", "2026-11-25")?, LATER)?;

    let work_id = role_id(WORK)?;
    let work = report
        .targets
        .iter()
        .find(|target| target.role == work_id)
        .ok_or("a target for work")?;
    assert_eq!((work.target, work.actual), (hours(5), hours(4)));
    assert_eq!(report.period.days, 1);
    Ok(())
}

#[test]
fn weekly_target_of_week_is_whole() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-24T09:00", "2026-11-24T13:00", on(GARDEN)?)?;

    let report = plain(&log, &days("2026-11-23", "2026-11-30")?, LATER)?;

    let targets: Vec<(RoleId, SignedDuration, SignedDuration)> = report
        .targets
        .iter()
        .map(|target| (target.role, target.target, target.actual))
        .collect();
    assert_eq!(
        targets,
        vec![
            (role_id(HEALTH)?, hours(7), hours(4)),
            (role_id(WORK)?, hours(35), SignedDuration::ZERO)
        ]
    );
    assert_eq!(report.warnings, Vec::new());
    Ok(())
}

#[test]
fn targets_above_168_hours_warn_with_their_sum() -> TestResult {
    let log = Log::new(1)?;
    let catalog = catalog_with(Some(120), Some(50), None)?;

    let report = report(
        &log.state(),
        &catalog,
        &days("2026-11-23", "2026-11-30")?,
        &ReportFilter::default(),
        LATER,
    )?;

    assert_eq!(
        report.warnings,
        vec![ReportWarning::TargetsAboveWeek { total: hours(170) }]
    );
    Ok(())
}

#[test]
fn untracked_is_time_passed_in_period_minus_tracked() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-24T09:00", "2026-11-24T17:00", on(WEBSHOP)?)?;

    let past = plain(&log, &days("2026-11-24", "2026-11-25")?, LATER)?;
    let today = plain(&log, &days("2026-11-24", "2026-11-25")?, "2026-11-24T18:00")?;

    assert_eq!(past.untracked, hours(16));
    assert_eq!(today.untracked, hours(10));
    Ok(())
}

#[test]
fn budget_counts_project_time_over_all_time() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-10-05T09:00", "2026-10-05T15:00", on(WEBSHOP)?)?;
    _ = log.log("2026-11-24T09:00", "2026-11-24T11:00", on(WEBSHOP)?)?;
    let all_time = log.state();
    let period = days("2026-11-24", "2026-11-25")?;
    let catalog = catalog_with(Some(35), Some(7), Some(10))?;
    let november = berlin("2026-11-01T00:00")?;
    let in_period = State::from_events(
        &log.events
            .iter()
            .filter(|recorded| recorded.event.at >= november)
            .cloned()
            .collect::<Vec<_>>(),
    );

    let needs_all = Report::needs_all_events(&in_period, &catalog, &ReportFilter::default());
    let report = Report::build(&ReportInput {
        state: &in_period,
        catalog: &catalog,
        period: &period,
        filter: &ReportFilter::default(),
        now: &berlin(LATER)?,
        zone: &zone()?,
        all_time: Some(&all_time),
    });

    assert!(needs_all);
    let budget = report.budgets.first().ok_or("the webshop's budget")?;
    assert_eq!(budget.project, project(WEBSHOP)?);
    assert_eq!((budget.used, budget.budget), (hours(8), hours(10)));
    assert_eq!(budget.percent, Some(80));
    Ok(())
}

#[test]
fn period_without_budgeted_project_needs_no_other_events() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-24T09:00", "2026-11-24T11:00", on(GARDEN)?)?;

    let catalog = catalog_with(None, None, Some(10))?;
    let needs_all = Report::needs_all_events(&log.state(), &catalog, &ReportFilter::default());

    assert!(!needs_all);
    Ok(())
}

#[test]
fn holds_count_by_source_with_five_most_frequent_reasons() -> TestResult {
    let mut log = Log::new(1)?;
    let entry = log.begin("2026-11-24T08:00", on(WEBSHOP)?)?;
    let holds = [
        ("09:00", "09:05", HoldSource::Outer, Some("phone call")),
        ("09:10", "09:15", HoldSource::Outer, Some("phone call")),
        ("09:20", "09:25", HoldSource::Outer, Some("phone call")),
        ("09:30", "09:35", HoldSource::Inner, Some("mail")),
        ("09:40", "09:45", HoldSource::Inner, Some("mail")),
        ("09:50", "09:55", HoldSource::Outer, Some("colleague")),
        ("10:00", "10:05", HoldSource::Inner, Some("news")),
        ("10:10", "10:15", HoldSource::Inner, Some("coffee")),
        ("10:20", "10:25", HoldSource::Unknown, Some("alarm")),
        ("10:30", "10:35", HoldSource::Unknown, None),
    ];
    for (from, to, source, reason) in holds {
        let (from, to) = (format!("2026-11-24T{from}"), format!("2026-11-24T{to}"));
        log.hold_for(entry, &from, &to, source, reason)?;
    }
    log.end(entry, "2026-11-24T12:00")?;

    let report = plain(&log, &days("2026-11-24", "2026-11-25")?, LATER)?;

    assert_eq!((report.holds.count, report.holds.time), (10, mins(50)));
    let sources: Vec<(HoldSource, usize, SignedDuration)> = report
        .holds
        .by_source
        .iter()
        .map(|share| (share.source, share.count, share.time))
        .collect();
    assert_eq!(
        sources,
        vec![
            (HoldSource::Inner, 4, mins(20)),
            (HoldSource::Outer, 4, mins(20)),
            (HoldSource::Unknown, 2, mins(10))
        ]
    );
    let reasons: Vec<(&str, usize)> = report
        .holds
        .reasons
        .iter()
        .map(|reason| (reason.reason.as_str(), reason.count))
        .collect();
    assert_eq!(
        reasons,
        vec![
            ("phone call", 3),
            ("mail", 2),
            ("alarm", 1),
            ("coffee", 1),
            ("colleague", 1)
        ]
    );
    Ok(())
}

#[test]
fn pauses_go_by_kind_and_catches_keep_their_texts() -> TestResult {
    let mut log = Log::new(1)?;
    let entry = log.begin("2026-11-24T09:00", on(WEBSHOP)?)?;
    log.pause(
        entry,
        "2026-11-24T10:00",
        "2026-11-24T10:10",
        PauseKind::Break,
    )?;
    log.pause(
        entry,
        "2026-11-24T12:00",
        "2026-11-24T12:45",
        PauseKind::Lunch,
    )?;
    log.pause(
        entry,
        "2026-11-24T15:00",
        "2026-11-24T15:05",
        PauseKind::Break,
    )?;
    log.catch(entry, "2026-11-24T11:00", "call the bank")?;
    log.end(entry, "2026-11-24T17:00")?;

    let report = plain(&log, &days("2026-11-24", "2026-11-25")?, LATER)?;

    let pauses: Vec<(PauseKind, usize, SignedDuration)> = report
        .pauses
        .iter()
        .map(|share| (share.kind, share.count, share.time))
        .collect();
    assert_eq!(
        pauses,
        vec![
            (PauseKind::Break, 2, mins(15)),
            (PauseKind::Lunch, 1, mins(45))
        ]
    );
    let catches: Vec<(EntryId, &str)> = report
        .catches
        .iter()
        .map(|caught| (caught.entry, caught.text.as_str()))
        .collect();
    assert_eq!(catches, vec![(entry, "call the bank")]);
    Ok(())
}

#[test]
fn who_decided_shares_time_by_me_by_others_and_unmarked() -> TestResult {
    let mut log = Log::new(1)?;
    let by = |determined_by| Fields {
        determined_by,
        ..Fields::default()
    };
    _ = log.log(
        "2026-11-24T09:00",
        "2026-11-24T12:00",
        by(Some(DeterminedBy::Me)),
    )?;
    _ = log.log(
        "2026-11-24T13:00",
        "2026-11-24T14:00",
        by(Some(DeterminedBy::Others)),
    )?;
    _ = log.log("2026-11-24T15:00", "2026-11-24T17:00", by(None))?;

    let report = plain(&log, &days("2026-11-24", "2026-11-25")?, LATER)?;

    let decided = &report.decided;
    assert_eq!(
        (decided.by_me, decided.by_others, decided.unmarked),
        (hours(3), hours(1), hours(2))
    );
    Ok(())
}

#[test]
fn overlap_warns_with_both_entries_and_shared_time() -> TestResult {
    let mut log = Log::new(1)?;
    let first = log.log("2026-11-24T09:00", "2026-11-24T11:00", on(WEBSHOP)?)?;
    let second = log.log("2026-11-24T10:00", "2026-11-24T12:00", on(GARDEN)?)?;

    let report = plain(&log, &days("2026-11-24", "2026-11-25")?, LATER)?;

    assert_eq!(
        report.warnings,
        vec![ReportWarning::Overlap {
            first,
            second,
            start: berlin("2026-11-24T10:00")?,
            end: berlin("2026-11-24T11:00")?,
        }]
    );
    Ok(())
}

/// The log of a day: the webshop (tag `call`), the garden (tag `deep`) and
/// an entry without a project (tag `call`), each one hour; the garden from
/// a second device.
fn day_of_three_entries() -> Fallible<(Log, Log)> {
    let mut log = Log::new(1)?;
    let mut other = Log::new(2)?;
    let tag =
        |name: &str| -> Fallible<BTreeSet<pace_core::Tag>> { Ok(BTreeSet::from([name.parse()?])) };
    _ = log.log(
        "2026-11-24T09:00",
        "2026-11-24T10:00",
        Fields {
            tags: tag("call")?,
            ..on(WEBSHOP)?
        },
    )?;
    _ = other.log(
        "2026-11-24T11:00",
        "2026-11-24T12:00",
        Fields {
            tags: tag("deep")?,
            ..on(GARDEN)?
        },
    )?;
    _ = log.log(
        "2026-11-24T13:00",
        "2026-11-24T14:00",
        Fields {
            tags: tag("call")?,
            ..Fields::default()
        },
    )?;
    Ok((log, other))
}

fn filtered(filter: &ReportFilter) -> Fallible<Report> {
    let (log, other) = day_of_three_entries()?;
    let events: Vec<RecordedEvent> = log.events.iter().chain(&other.events).cloned().collect();
    report(
        &State::from_events(&events),
        &catalog()?,
        &days("2026-11-24", "2026-11-25")?,
        filter,
        LATER,
    )
}

#[test]
fn filters_of_one_kind_combine_with_or() -> TestResult {
    let filter = ReportFilter {
        projects: BTreeSet::from([project(WEBSHOP)?, project(GARDEN)?]),
        ..ReportFilter::default()
    };

    let report = filtered(&filter)?;

    assert_eq!(report.tracked, hours(2));
    assert_eq!(report.back_flow.net, SignedDuration::ZERO);
    Ok(())
}

#[test]
fn filters_of_different_kinds_combine_with_and() -> TestResult {
    let filter = ReportFilter {
        tags: BTreeSet::from(["call".parse()?]),
        roles: BTreeSet::from([role_id(WORK)?]),
        ..ReportFilter::default()
    };

    let report = filtered(&filter)?;

    assert_eq!(report.tracked, hours(1));
    assert_eq!(report.projects.len(), 1);
    Ok(())
}

#[test]
fn client_filter_keeps_projects_of_that_client() -> TestResult {
    let filter = ReportFilter {
        clients: BTreeSet::from(["acme".parse()?]),
        ..ReportFilter::default()
    };

    let report = filtered(&filter)?;

    let projects: Vec<&ProjectRef> = report.projects.iter().map(|time| &time.project).collect();
    assert_eq!(projects, vec![&project(WEBSHOP)?]);
    Ok(())
}

#[test]
fn device_filter_keeps_entries_of_that_device() -> TestResult {
    let (_, other) = day_of_three_entries()?;
    let filter = ReportFilter {
        devices: BTreeSet::from([other.device]),
        ..ReportFilter::default()
    };

    let report = filtered(&filter)?;

    let projects: Vec<&ProjectRef> = report.projects.iter().map(|time| &time.project).collect();
    assert_eq!(projects, vec![&project(GARDEN)?]);
    assert_eq!(report.tracked, hours(1));
    Ok(())
}

#[test]
fn period_open_at_start_begins_with_first_entry() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-20T09:00", "2026-11-20T10:00", on(WEBSHOP)?)?;
    _ = log.log("2026-11-24T09:00", "2026-11-24T10:00", on(WEBSHOP)?)?;

    let report = plain(&log, &TimeRange::everything(), "2026-11-26T12:00")?;

    assert_eq!(report.period.start, berlin("2026-11-20T09:00")?);
    assert_eq!(report.period.end, berlin("2026-11-26T12:00")?);
    assert_eq!(report.period.days, 7);
    assert_eq!(report.tracked, hours(2));
    Ok(())
}

#[test]
fn report_serializes_to_json() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.log("2026-11-24T09:00", "2026-11-24T10:00", on(WEBSHOP)?)?;

    let report = plain(&log, &days("2026-11-24", "2026-11-25")?, LATER)?;
    let json = serde_json::to_value(&report)?;

    assert_eq!(json["tracked"], "PT1H");
    assert_eq!(json["days"][0]["date"], "2026-11-24");
    Ok(())
}
