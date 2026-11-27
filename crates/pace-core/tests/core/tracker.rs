//! The tracker reads the state through the store port, decides, and appends.

use std::sync::Arc;

use jiff::ZonedDateTime;
use pace_core::testing::contract::run_store_contract;
use pace_core::{
    Clock, Command, DeviceId, EntryState, EventStore, Policy, SequentialIds, State, Tracker,
    command::{Adjust, Begin, End, Log},
    testing::{MemoryStore, Raced, TestClock},
};
use pace_time::TimeRange;

use crate::TestResult;

fn at(rfc9557: &str) -> Result<ZonedDateTime, jiff::Error> {
    rfc9557.parse()
}

fn device(number: u16) -> Result<DeviceId, Box<dyn std::error::Error>> {
    Ok(format!("0193a8d0-000d-7000-8000-{number:012x}").parse()?)
}

fn tracker(
    store: MemoryStore,
    clock: &Arc<TestClock>,
    number: u16,
) -> Result<Tracker<MemoryStore>, Box<dyn std::error::Error>> {
    Ok(Tracker::new(
        store,
        clock.clone(),
        Arc::new(SequentialIds::starting_at(&clock.now(), number)),
        Policy::default(),
        device(number)?,
    ))
}

fn everything() -> Result<TimeRange, Box<dyn std::error::Error>> {
    Ok(TimeRange::new(None, None)?)
}

#[test]
fn command_is_decided_appended_and_shown_in_the_new_state() -> TestResult {
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let store = MemoryStore::new(device(1)?, clock.clone());
    let tracker = tracker(store, &clock, 1)?;

    let outcome = pollster::block_on(tracker.run(&Command::Begin(Begin::default())))?;

    assert_eq!(outcome.recorded.len(), 1);
    assert_eq!(outcome.state.open().count(), 1);
    let stored = pollster::block_on(tracker.state(&everything()?))?;
    assert_eq!(stored.open().count(), 1);
    Ok(())
}

#[test]
fn rejected_command_appends_nothing() -> TestResult {
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let store = MemoryStore::new(device(1)?, clock.clone());
    let tracker = tracker(store, &clock, 1)?;

    let result = pollster::block_on(tracker.run(&Command::End(End::default())));

    assert!(result.is_err());
    assert_eq!(
        pollster::block_on(tracker.state(&everything()?))?
            .entries()
            .count(),
        0
    );
    Ok(())
}

#[test]
fn two_devices_on_one_storage_see_each_others_entries() -> TestResult {
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let laptop_store = MemoryStore::new(device(1)?, clock.clone());
    let desktop_store = laptop_store.for_device(device(2)?);
    let laptop = tracker(laptop_store, &clock, 1)?;
    let desktop = tracker(desktop_store, &clock, 2)?;

    _ = pollster::block_on(laptop.run(&Command::Begin(Begin::default())))?;
    clock.set(at("2026-11-26T10:00[Europe/Berlin]")?);
    _ = pollster::block_on(desktop.run(&Command::End(End::default())))?;

    let state = pollster::block_on(laptop.state(&everything()?))?;
    let entry = state.entries().next().ok_or("entry")?;
    assert_eq!(entry.state(), EntryState::Ended);
    assert_eq!(entry.device, device(1)?);
    Ok(())
}

#[test]
fn per_field_the_later_change_wins_between_devices() -> TestResult {
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let laptop_store = MemoryStore::new(device(1)?, clock.clone());
    let desktop_store = laptop_store.for_device(device(2)?);
    let laptop = tracker(laptop_store, &clock, 1)?;
    let desktop = tracker(desktop_store, &clock, 2)?;
    _ = pollster::block_on(laptop.run(&Command::Log(Log::new(
        at("2026-11-26T07:00[Europe/Berlin]")?,
        at("2026-11-26T08:00[Europe/Berlin]")?,
    ))))?;
    let entry = pollster::block_on(laptop.state(&everything()?))?
        .entries()
        .next()
        .map(|entry| entry.id)
        .ok_or("entry")?;
    let note = |text: &str| {
        Command::Adjust(Adjust {
            entry: Some(entry),
            note: pace_core::Change::Set(text.to_owned()),
            ..Adjust::default()
        })
    };

    clock.set(at("2026-11-26T09:10[Europe/Berlin]")?);
    _ = pollster::block_on(laptop.run(&note("from the laptop")))?;
    clock.set(at("2026-11-26T09:05[Europe/Berlin]")?);
    _ = pollster::block_on(desktop.run(&note("from the desktop")))?;

    let state: State = pollster::block_on(desktop.state(&everything()?))?;
    assert_eq!(
        state.entry(entry).and_then(|entry| entry.note.as_deref()),
        Some("from the desktop"),
        "the desktop wrote after reading the laptop's change, so its HLC is later"
    );
    Ok(())
}

#[test]
fn the_memory_store_keeps_the_store_contract() -> TestResult {
    pollster::block_on(run_store_contract(MemoryStore::opener))?;
    Ok(())
}

#[test]
fn the_marker_tells_whether_the_store_changed() -> TestResult {
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let store = MemoryStore::new(device(1)?, clock.clone());
    let before = pollster::block_on(store.marker())?;
    let tracker = tracker(store.clone(), &clock, 1)?;

    _ = pollster::block_on(tracker.run(&Command::Begin(Begin::default())))?;

    assert_ne!(pollster::block_on(store.marker())?, before);
    Ok(())
}

#[test]
fn an_outcome_shows_the_state_before_and_what_an_undo_took_back() -> TestResult {
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let store = MemoryStore::new(device(1)?, clock.clone());
    let tracker = tracker(store, &clock, 1)?;
    let begun = pollster::block_on(tracker.run(&Command::Begin(Begin::default())))?;

    let undo = pollster::block_on(tracker.run(&Command::Undo))?;

    assert_eq!(undo.before.open().count(), 1);
    assert_eq!(undo.state.open().count(), 0);
    assert_eq!(undo.undone, begun.recorded);
    Ok(())
}

#[test]
fn log_that_ends_before_it_starts_is_refused_by_the_rules() -> TestResult {
    let clock = Arc::new(TestClock::new(at("2026-11-26T18:00[Europe/Berlin]")?));
    let store = MemoryStore::new(device(1)?, clock.clone());
    let tracker = tracker(store, &clock, 1)?;

    let result = pollster::block_on(tracker.run(&Command::Log(Log::new(
        at("2026-11-26T17:00[Europe/Berlin]")?,
        at("2026-11-26T16:50[Europe/Berlin]")?,
    ))));

    let error = result.err().ok_or("the log must fail")?;
    assert_eq!(
        error.to_string(),
        "the entry would end at 16:50, before it starts at 17:00"
    );
    Ok(())
}

/// Two pace commands at once on one device: `rival` runs `races` times, by
/// `rival_clock`, between the read and the append of the returned tracker's
/// command.
fn raced(
    store: &MemoryStore,
    clock: &Arc<TestClock>,
    rival_clock: &Arc<TestClock>,
    rival: Command,
    races: usize,
) -> Result<Tracker<Raced<MemoryStore, MemoryStore>>, Box<dyn std::error::Error>> {
    let rival_tracker = Tracker::new(
        store.clone(),
        rival_clock.clone(),
        Arc::new(SequentialIds::starting_at(&rival_clock.now(), 2)),
        Policy::default(),
        device(1)?,
    );
    Ok(Tracker::new(
        Raced::new(store.clone(), rival_tracker, rival, races),
        clock.clone(),
        Arc::new(SequentialIds::starting_at(&clock.now(), 1)),
        Policy::default(),
        device(1)?,
    ))
}

#[test]
fn begin_raced_by_another_begin_decides_again_and_switches_from_it() -> TestResult {
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let rival_clock = Arc::new(TestClock::new(at("2026-11-26T08:59[Europe/Berlin]")?));
    let store = MemoryStore::new(device(1)?, clock.clone());
    let rival = Command::Begin(Begin {
        note: Some("B".to_owned()),
        ..Begin::default()
    });
    let tracker = raced(&store, &clock, &rival_clock, rival, 1)?;

    let outcome = pollster::block_on(tracker.run(&Command::Begin(Begin {
        note: Some("A".to_owned()),
        ..Begin::default()
    })))?;

    let state = pollster::block_on(tracker.state(&everything()?))?;
    let open: Vec<_> = state.open().map(|entry| entry.note.clone()).collect();
    assert_eq!(open, vec![Some("A".to_owned())]);
    assert_eq!(state.entries().count(), 2);
    assert_eq!(outcome.before.open().count(), 1, "the decision saw B");
    Ok(())
}

#[test]
fn command_raced_every_time_fails_naming_the_concurrent_writes() -> TestResult {
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let store = MemoryStore::new(device(1)?, clock.clone());
    let rival = Command::Log(Log {
        allow_overlap: true,
        ..Log::new(
            at("2026-11-26T07:00[Europe/Berlin]")?,
            at("2026-11-26T08:00[Europe/Berlin]")?,
        )
    });
    let tracker = raced(&store, &clock, &clock, rival, usize::MAX)?;

    let error = pollster::block_on(tracker.run(&Command::Begin(Begin::default())))
        .err()
        .ok_or("the command passed")?;

    assert!(error.is_code("pace::core::concurrent-write"), "{error}");
    assert_eq!(error.context_value("attempts"), Some("5"));
    let state = pollster::block_on(tracker.state(&everything()?))?;
    assert_eq!(state.open().count(), 0, "the begin was appended");
    Ok(())
}
