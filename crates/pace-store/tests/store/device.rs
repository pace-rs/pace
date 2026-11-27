//! A machine's device ID lives in the machine directory, never synced; pace
//! notices when two machines write with the same ID.

use std::sync::Arc;

use jiff::ZonedDateTime;
use pace_core::{
    Clock, Command, DeviceId, Policy, SequentialIds, Tracker,
    command::{Begin, End},
    testing::TestClock,
};
use pace_store::{FilesStore, Split, device_id, testing::scratch_dir};
use pace_time::TimeRange;

use crate::TestResult;

fn at(rfc9557: &str) -> Result<ZonedDateTime, jiff::Error> {
    rfc9557.parse()
}

#[test]
fn the_first_run_creates_the_device_id_and_later_runs_keep_it() -> TestResult {
    let dir = scratch_dir()?;
    let machine = dir.path().join("machine");
    let ids = SequentialIds::starting_at(&at("2026-11-26T09:00[Europe/Berlin]")?, 7);

    let first = device_id(&machine, &ids)?;
    let second = device_id(&machine, &ids)?;

    assert!(first.created);
    assert!(!second.created);
    assert_eq!(first.id, second.id);
    let text = std::fs::read_to_string(machine.join("device.toml"))?;
    assert_eq!(text, format!("version = 1\nid = \"{}\"\n", first.id));
    Ok(())
}

#[test]
fn broken_device_file_says_how_to_get_new_id() -> TestResult {
    let dir = scratch_dir()?;
    let machine = dir.path().join("machine");
    std::fs::create_dir_all(&machine)?;
    std::fs::write(
        machine.join("device.toml"),
        "version = 1\nid = \"laptop\"\n",
    )?;
    let ids = SequentialIds::starting_at(&at("2026-11-26T09:00[Europe/Berlin]")?, 7);

    let error = device_id(&machine, &ids).err().ok_or("must fail")?;

    assert!(
        error
            .hint()
            .is_some_and(|hint| hint.contains("device.toml")),
        "{error:?}"
    );
    Ok(())
}

/// One machine: its own machine directory, a shared `PACE_HOME`.
fn machine(
    home: &std::path::Path,
    machine: &std::path::Path,
    device: DeviceId,
    clock: &Arc<TestClock>,
    seed: u16,
) -> Tracker<FilesStore> {
    Tracker::new(
        FilesStore::new(
            home.to_path_buf(),
            machine.to_path_buf(),
            device,
            clock.clone(),
            Split::Month,
        ),
        clock.clone(),
        Arc::new(SequentialIds::starting_at(&clock.now(), seed)),
        Policy::default(),
        device,
    )
}

#[test]
fn device_id_used_by_two_machines_stops_writing() -> TestResult {
    let dir = scratch_dir()?;
    let home = dir.path().join("home");
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let device: DeviceId = "0193a8d0-000d-7000-8000-000000000001".parse()?;
    // A backup of the machine directory restored onto a second machine.
    let laptop = machine(&home, &dir.path().join("laptop"), device, &clock, 1);
    let copy = machine(&home, &dir.path().join("copy"), device, &clock, 2);

    _ = pollster::block_on(laptop.run(&Command::Begin(Begin::default())))?;
    clock.set(at("2026-11-26T10:00[Europe/Berlin]")?);
    _ = pollster::block_on(copy.run(&Command::End(End::default())))?;
    clock.set(at("2026-11-26T11:00[Europe/Berlin]")?);

    let error = pollster::block_on(laptop.run(&Command::Begin(Begin::default())))
        .err()
        .ok_or("the laptop must notice the copy")?;

    assert_eq!(
        error.to_string(),
        "another machine writes with this machine's device ID"
    );
    assert!(
        error
            .hint()
            .is_some_and(|hint| hint.contains("device.toml")),
        "{error:?}"
    );
    let state = pollster::block_on(laptop.state(&TimeRange::new(None, None)?))?;
    assert_eq!(state.entries().count(), 1, "nothing more was written");
    Ok(())
}

#[test]
fn deleting_the_index_changes_nothing() -> TestResult {
    let dir = scratch_dir()?;
    let home = dir.path().join("home");
    let machine_dir = dir.path().join("machine");
    let clock = Arc::new(TestClock::new(at("2026-10-15T09:00[Europe/Berlin]")?));
    let device: DeviceId = "0193a8d0-000d-7000-8000-000000000001".parse()?;
    let tracker = machine(&home, &machine_dir, device, &clock, 1);
    _ = pollster::block_on(tracker.run(&Command::Begin(Begin::default())))?;
    clock.set(at("2026-11-26T09:00[Europe/Berlin]")?);
    let today = TimeRange::new(
        Some(at("2026-11-26T00:00[Europe/Berlin]")?),
        Some(at("2026-11-27T00:00[Europe/Berlin]")?),
    )?;
    let before = pollster::block_on(tracker.state(&today))?;

    std::fs::remove_file(machine_dir.join("index.json"))?;
    let after = pollster::block_on(tracker.state(&today))?;

    assert_eq!(before, after);
    assert_eq!(after.open().count(), 1);
    Ok(())
}
