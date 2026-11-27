//! The contract of [`EventStore`], as checks that every backend runs.

use std::sync::Arc;

use jiff::ZonedDateTime;
use pace_core::{
    Appended, Catalog, Change, Clock, Command, DeviceId, EntryState, EventKind, Policy, ReadScope,
    Rules, State,
    command::{Adjust, Begin, End, Log},
    testing::SequentialIds,
    testing::TestClock,
};
use pace_error::{ErrorKind, PaceError, PaceResult};
use pace_time::TimeRange;

use crate::{EventStore, Tracker};

/// Run every check. `new_storage` gives fresh, empty storage for each
/// check, as a function that opens it for a device with a clock; stores
/// opened from one storage share it, like devices syncing `PACE_HOME`.
///
/// # Errors
///
/// The first broken promise, naming the check.
pub fn run_store_contract<O, S>(new_storage: impl Fn() -> O) -> PaceResult<()>
where
    O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
    S: EventStore,
{
    // The read checks come before the HLC checks: those run the tracker,
    // which reads, so a broken read would fail them under the wrong name.
    within(
        COME_BACK,
        appended_events_come_back_with_their_device_and_batch(&new_storage()),
    )?;
    within(
        OPEN_ENTRIES,
        open_entries_are_read_whatever_the_range(&new_storage()),
    )?;
    within(
        OVERLAPPING_ENTRIES,
        entries_overlapping_the_range_are_read_wherever_they_are_filed(&new_storage()),
    )?;
    within(
        RECORDED_SINCE,
        entries_recorded_since_are_read_wherever_they_are_filed(&new_storage()),
    )?;
    within(
        WHOLE_ENTRIES,
        read_returns_every_event_of_its_entries(&new_storage()),
    )?;
    within(
        UNDO_WITH_BATCH,
        an_undo_is_read_with_its_batch(&new_storage()),
    )?;
    within(
        HLC_NEVER_GOES_BACK,
        the_hlc_never_goes_back_when_the_clock_does(&new_storage()),
    )?;
    within(
        HLC_AFTER_OTHER_DEVICES,
        the_hlc_comes_after_what_other_devices_wrote(&new_storage()),
    )?;
    within(DEVICES, devices_are_the_ones_that_wrote(&new_storage()))?;
    within(
        MARKER_CHANGES,
        the_marker_changes_with_the_data(&new_storage()),
    )?;
    within(
        STALE_APPEND,
        append_after_stale_read_is_refused(&new_storage()),
    )?;
    Ok(())
}

// The checks, by the promise each one keeps.
const COME_BACK: &str = "appended events come back with their device and batch";
const OPEN_ENTRIES: &str = "open entries are read whatever the range";
const OVERLAPPING_ENTRIES: &str = "entries overlapping the range are read wherever they are filed";
const RECORDED_SINCE: &str =
    "entries with an event recorded since a moment are read wherever they are filed";
const WHOLE_ENTRIES: &str = "a read returns every event of each entry it returns";
const UNDO_WITH_BATCH: &str = "an undo is read with its batch";
const HLC_NEVER_GOES_BACK: &str = "the HLC never goes back when the clock does";
const HLC_AFTER_OTHER_DEVICES: &str = "the HLC comes after what other devices wrote";
const DEVICES: &str = "devices are the ones that wrote";
const MARKER_CHANGES: &str = "the marker changes with the data";
const STALE_APPEND: &str = "an append after a stale read is refused";

/// A check's result; a failed command inside it breaks the check too,
/// so the error names the check.
fn within(check: &str, result: PaceResult<()>) -> PaceResult<()> {
    result.map_err(|error| {
        if error.context_value("check").is_some() {
            error
        } else {
            broken(check, error)
        }
    })
}

fn broken(check: &str, detail: impl std::fmt::Display) -> Box<PaceError> {
    PaceError::new(
        ErrorKind::Internal,
        "the store breaks its contract: {check}: {detail}",
    )
    .attach_context("check", check)
    .attach_context("detail", detail.to_string())
}

fn ensure(condition: bool, check: &str, detail: impl std::fmt::Display) -> PaceResult<()> {
    if condition {
        Ok(())
    } else {
        Err(broken(check, detail))
    }
}

fn at(rfc9557: &str) -> PaceResult<ZonedDateTime> {
    rfc9557.parse().map_err(|error| {
        PaceError::with_source(
            ErrorKind::Internal,
            "the test time {time} is no RFC 9557 time",
            error,
        )
        .attach_context("time", rfc9557)
    })
}

fn device(number: u16) -> PaceResult<DeviceId> {
    format!("0193a8d0-000d-7000-8000-{number:012x}").parse()
}

/// A device: its tracker and its clock.
struct Device<S> {
    tracker: Tracker<S>,
    clock: Arc<TestClock>,
    id: DeviceId,
}

fn open<O, S>(open: &O, number: u16, now: &str) -> PaceResult<Device<S>>
where
    O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
    S: EventStore,
{
    let clock = Arc::new(TestClock::new(at(now)?));
    let id = device(number)?;
    let tracker = Tracker::new(
        open(id, clock.clone()),
        clock.clone(),
        Arc::new(SequentialIds::starting_at(&clock.now(), number)),
        Policy::default(),
        Catalog::default(),
        id,
    );
    Ok(Device { tracker, clock, id })
}

fn range(start: &str, end: &str) -> PaceResult<TimeRange> {
    TimeRange::new(Some(at(start)?), Some(at(end)?))
}

fn appended_events_come_back_with_their_device_and_batch<O, S>(open_store: &O) -> PaceResult<()>
where
    O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
    S: EventStore,
{
    let check = COME_BACK;
    let laptop = open(open_store, 1, "2026-11-26T09:00[Europe/Berlin]")?;
    let outcome = laptop.tracker.run(&Command::Begin(Begin::default()))?;
    let appended = outcome
        .recorded
        .first()
        .ok_or_else(|| broken(check, "append returned nothing"))?;

    let read = laptop.tracker.state(&TimeRange::new(None, None)?)?;
    let found = State::from_events(std::slice::from_ref(appended));
    ensure(appended.device == laptop.id, check, "wrong device")?;
    ensure(
        outcome.warnings.is_empty(),
        check,
        format!("an append on sound storage warned: {:?}", outcome.warnings),
    )?;
    ensure(
        read.entries().count() == 1,
        check,
        "the entry was not read back",
    )?;
    ensure(
        found.entries().next() == read.entries().next(),
        check,
        "the entry changed on the way",
    )
}

fn open_entries_are_read_whatever_the_range<O, S>(open_store: &O) -> PaceResult<()>
where
    O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
    S: EventStore,
{
    let check = OPEN_ENTRIES;
    let laptop = open(open_store, 1, "2026-10-15T09:00[Europe/Berlin]")?;
    _ = laptop.tracker.run(&Command::Begin(Begin::default()))?;
    laptop.clock.set(at("2026-11-26T09:00[Europe/Berlin]")?);

    let state = laptop.tracker.state(&range(
        "2026-11-26T00:00[Europe/Berlin]",
        "2026-11-27T00:00[Europe/Berlin]",
    )?)?;
    ensure(
        state.open().count() == 1,
        check,
        "an entry begun in October and still open is missing",
    )
}

fn entries_overlapping_the_range_are_read_wherever_they_are_filed<O, S>(
    open_store: &O,
) -> PaceResult<()>
where
    O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
    S: EventStore,
{
    let check = OVERLAPPING_ENTRIES;
    let laptop = open(open_store, 1, "2026-11-02T09:00[Europe/Berlin]")?;
    _ = laptop.tracker.run(&Command::Log(Log::new(
        at("2026-10-31T22:00[Europe/Berlin]")?,
        at("2026-11-01T02:00[Europe/Berlin]")?,
    )))?;

    let state = laptop.tracker.state(&range(
        "2026-11-01T00:00[Europe/Berlin]",
        "2026-11-01T01:00[Europe/Berlin]",
    )?)?;
    ensure(
        state.entries().count() == 1,
        check,
        "an entry from 31 October 22:00 to 1 November 02:00 is missing in 1 November",
    )
}

fn entries_recorded_since_are_read_wherever_they_are_filed<O, S>(open_store: &O) -> PaceResult<()>
where
    O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
    S: EventStore,
{
    let check = RECORDED_SINCE;
    let laptop = open(open_store, 1, "2026-08-01T09:00[Europe/Berlin]")?;
    let logged = laptop.tracker.run(&Command::Log(Log::new(
        at("2026-07-31T08:00[Europe/Berlin]")?,
        at("2026-07-31T09:00[Europe/Berlin]")?,
    )))?;
    let entry = logged
        .recorded
        .iter()
        .find_map(|recorded| match recorded.event.kind {
            EventKind::EntryLogged { entry, .. } => Some(entry),
            _ => None,
        })
        .ok_or_else(|| broken(check, "the log recorded no entry"))?;
    let desktop = open(open_store, 2, "2026-11-26T09:00[Europe/Berlin]")?;
    _ = desktop.tracker.run(&Command::Adjust(Adjust {
        entry: Some(entry),
        note: Change::Set("Recorded in November".parse()?),
        ..Adjust::default()
    }))?;
    _ = desktop.tracker.run(&Command::Log(Log::new(
        at("2026-06-30T08:00[Europe/Berlin]")?,
        at("2026-06-30T09:00[Europe/Berlin]")?,
    )))?;

    let now = desktop.clock.now();
    let scope = ReadScope::of(TimeRange::new(Some(now.clone()), Some(now))?)
        .with_recorded_since(at("2026-11-25T09:00[Europe/Berlin]")?.timestamp());
    let state = State::from_events(&laptop.tracker.store().read(&scope)?);
    let logged_in_november = at("2026-06-30T08:00[Europe/Berlin]")?;
    ensure(
        state
            .entries()
            .any(|logged| logged.start == logged_in_november),
        check,
        "an entry of June, logged in November, is missing in a read of what was recorded since",
    )?;
    let note = state
        .entry(entry)
        .ok_or_else(|| {
            broken(
                check,
                "an entry of July, adjusted in November, is missing in a read of what was recorded since",
            )
        })?
        .note
        .clone();
    ensure(
        note.as_deref() == Some("Recorded in November"),
        check,
        format!("the entry came without its adjustment: its note is {note:?}"),
    )
}

/// Device 2 begins an entry that device 1 ends: with a daily split on
/// device 1 and a monthly one on device 2, the end lies in a file whose
/// moments are far from what device 1 begins later. Device 2's log on
/// 21 November carries the check: it stretches the abouts of device 2's
/// month to meet device 1's file of 20 November. So a read picks device 2's
/// month with the entry's begin, and one step of widening misses device 1's
/// file of 3 November with its end.
fn read_returns_every_event_of_its_entries<O, S>(open_store: &O) -> PaceResult<()>
where
    O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
    S: EventStore,
{
    let check = WHOLE_ENTRIES;
    let laptop = open(open_store, 1, "2026-11-03T10:00[Europe/Berlin]")?;
    let desktop = open(open_store, 2, "2026-11-03T09:00[Europe/Berlin]")?;
    _ = desktop.tracker.run(&Command::Begin(Begin::default()))?;
    _ = laptop.tracker.run(&Command::End(End::default()))?;
    laptop.clock.set(at("2026-11-20T09:00[Europe/Berlin]")?);
    _ = laptop.tracker.run(&Command::Begin(Begin::default()))?;
    desktop.clock.set(at("2026-11-21T10:00[Europe/Berlin]")?);
    _ = desktop.tracker.run(&Command::Log(Log {
        allow_overlap: true,
        ..Log::new(
            at("2026-11-21T08:00[Europe/Berlin]")?,
            at("2026-11-21T09:00[Europe/Berlin]")?,
        )
    }))?;
    laptop.clock.set(at("2026-11-27T09:00[Europe/Berlin]")?);

    let all = laptop.tracker.state(&TimeRange::new(None, None)?)?;
    let state = laptop.tracker.state(&range(
        "2026-11-27T00:00[Europe/Berlin]",
        "2026-11-28T00:00[Europe/Berlin]",
    )?)?;
    for entry in state.entries() {
        ensure(
            all.entry(entry.id) == Some(entry),
            check,
            format!(
                "the entry begun at {} came without some of its events",
                entry.start
            ),
        )?;
    }
    ensure(
        state.open().count() == 1,
        check,
        format!(
            "a read of 27 November shows {} open entries, not one",
            state.open().count()
        ),
    )
}

fn an_undo_is_read_with_its_batch<O, S>(open_store: &O) -> PaceResult<()>
where
    O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
    S: EventStore,
{
    let check = UNDO_WITH_BATCH;
    let laptop = open(open_store, 1, "2026-10-15T09:00[Europe/Berlin]")?;
    _ = laptop.tracker.run(&Command::Begin(Begin::default()))?;
    laptop.clock.set(at("2026-10-15T10:00[Europe/Berlin]")?);
    _ = laptop.tracker.run(&Command::End(End::default()))?;
    laptop.clock.set(at("2026-11-26T09:00[Europe/Berlin]")?);
    _ = laptop.tracker.run(&Command::Undo)?;

    let state = laptop.tracker.state(&range(
        "2026-10-15T00:00[Europe/Berlin]",
        "2026-10-16T00:00[Europe/Berlin]",
    )?)?;
    let entry = state
        .entries()
        .next()
        .ok_or_else(|| broken(check, "the entry is missing"))?;
    ensure(
        entry.state() == EntryState::Running,
        check,
        "the end was undone in November, but reading October still shows it",
    )
}

fn the_hlc_never_goes_back_when_the_clock_does<O, S>(open_store: &O) -> PaceResult<()>
where
    O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
    S: EventStore,
{
    let check = HLC_NEVER_GOES_BACK;
    let laptop = open(open_store, 1, "2026-11-26T10:00[Europe/Berlin]")?;
    let first = laptop.tracker.run(&Command::Begin(Begin::default()))?;
    laptop.clock.set(at("2026-11-26T09:00[Europe/Berlin]")?);
    // The log lies before the jumped-back clock, and so before the begin's
    // HLC; the store must still order it after the begin.
    let second = laptop.tracker.run(&Command::Log(Log::new(
        at("2026-11-26T08:00[Europe/Berlin]")?,
        at("2026-11-26T08:30[Europe/Berlin]")?,
    )))?;
    let (Some(first), Some(second)) = (first.recorded.first(), second.recorded.first()) else {
        return Err(broken(check, "append returned nothing"));
    };
    ensure(
        second.hlc > first.hlc,
        check,
        format!("{:?} after {:?}", second.hlc, first.hlc),
    )
}

fn the_hlc_comes_after_what_other_devices_wrote<O, S>(open_store: &O) -> PaceResult<()>
where
    O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
    S: EventStore,
{
    let check = HLC_AFTER_OTHER_DEVICES;
    let laptop = open(open_store, 1, "2026-11-26T10:00[Europe/Berlin]")?;
    let desktop = open(open_store, 2, "2026-11-26T09:00[Europe/Berlin]")?;
    let began = laptop.tracker.run(&Command::Begin(Begin {
        at: Some(at("2026-11-26T08:00[Europe/Berlin]")?),
        ..Begin::default()
    }))?;
    let ended = desktop.tracker.run(&Command::End(End {
        at: Some(at("2026-11-26T08:30[Europe/Berlin]")?),
        ..End::default()
    }))?;
    let (Some(began), Some(ended)) = (began.recorded.first(), ended.recorded.first()) else {
        return Err(broken(check, "append returned nothing"));
    };
    ensure(
        ended.hlc > began.hlc,
        check,
        "the desktop's clock is behind, but its event must come later",
    )
}

fn devices_are_the_ones_that_wrote<O, S>(open_store: &O) -> PaceResult<()>
where
    O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
    S: EventStore,
{
    let check = DEVICES;
    let laptop = open(open_store, 1, "2026-11-26T09:00[Europe/Berlin]")?;
    let desktop = open(open_store, 2, "2026-11-26T10:00[Europe/Berlin]")?;
    _ = laptop.tracker.run(&Command::Begin(Begin::default()))?;
    _ = desktop.tracker.run(&Command::End(End::default()))?;

    let store = open_store(laptop.id, laptop.clock.clone());
    let mut ids: Vec<DeviceId> = store.devices()?.iter().map(|device| device.id).collect();
    ids.sort();
    let mut expected = vec![laptop.id, desktop.id];
    expected.sort();
    ensure(ids == expected, check, format!("{ids:?}"))
}

fn the_marker_changes_with_the_data<O, S>(open_store: &O) -> PaceResult<()>
where
    O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
    S: EventStore,
{
    let check = MARKER_CHANGES;
    let laptop = open(open_store, 1, "2026-11-26T09:00[Europe/Berlin]")?;
    let store = open_store(laptop.id, laptop.clock.clone());
    let before = store.marker()?;
    ensure(
        store.marker()? == before,
        check,
        "it changed without a change",
    )?;
    _ = laptop.tracker.run(&Command::Begin(Begin::default()))?;
    ensure(
        store.marker()? != before,
        check,
        "it stayed after an append",
    )
}

fn append_after_stale_read_is_refused<O, S>(open_store: &O) -> PaceResult<()>
where
    O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
    S: EventStore,
{
    let check = STALE_APPEND;
    let laptop = open(open_store, 1, "2026-11-26T09:00[Europe/Berlin]")?;
    let store = open_store(laptop.id, laptop.clock.clone());
    let read = store.marker()?;
    _ = laptop.tracker.run(&Command::Begin(Begin::default()))?;
    // The batch as decided on the stale read: nothing was open.
    let now = laptop.clock.now();
    let batch = Rules {
        state: &State::default(),
        now: &now,
        device: laptop.id,
        ids: &SequentialIds::starting_at(&now, 9),
        policy: &Policy::default(),
        catalog: &Catalog::default(),
    }
    .decide(&Command::Begin(Begin::default()))?;

    let appended = store.append(batch, read)?;
    ensure(
        matches!(appended, Appended::Stale),
        check,
        "it appended a batch decided before another append",
    )?;
    let state = laptop.tracker.state(&TimeRange::new(None, None)?)?;
    ensure(
        state.entries().count() == 1,
        check,
        "the refused batch is in the log",
    )
}
