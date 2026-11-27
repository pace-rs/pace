//! A machine's device ID lives in the machine directory, never synced; pace
//! notices when two machines write with the same ID.

use std::sync::Arc;

use jiff::ZonedDateTime;
use pace_core::{
    Catalog, Clock, Command, DeviceId, Policy,
    command::{Begin, End},
    testing::SequentialIds,
    testing::TestClock,
};
use pace_store::{
    FilesStore, Split, Tracker, detect_cloned_device, device_id, event_file_paths,
    last_written_hlc, parse_events_collecting, testing::scratch_dir,
};
use pace_time::TimeRange;
use rstest::rstest;

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

    let path = machine.join("device.toml");
    assert_eq!(
        error.to_string(),
        format!(
            "the device file {} holds no valid device ID",
            path.display()
        )
    );
    assert_eq!(
        error.hint(),
        Some(
            format!(
                "remove {} and pace gives this machine a new device ID",
                path.display()
            )
            .as_str()
        )
    );
    Ok(())
}

#[test]
fn device_id_of_newer_pace_asks_for_update() -> TestResult {
    let dir = scratch_dir()?;
    let machine = dir.path().join("machine");
    std::fs::create_dir_all(&machine)?;
    let path = machine.join("device.toml");
    std::fs::write(
        &path,
        "version = 7\nid = \"0193a8d0-000d-7000-8000-000000000001\"\n",
    )?;
    let ids = SequentialIds::starting_at(&at("2026-11-26T09:00[Europe/Berlin]")?, 7);

    let error = device_id(&machine, &ids)
        .err()
        .ok_or("a newer version must stop pace")?;

    assert_eq!(
        error.to_string(),
        format!("this pace reads version 1 of {}, not 7", path.display())
    );
    assert_eq!(
        error.hint(),
        Some("a newer pace wrote this file; update pace")
    );
    Ok(())
}

#[test]
fn device_file_of_older_version_says_how_to_get_new_id() -> TestResult {
    let dir = scratch_dir()?;
    let machine = dir.path().join("machine");
    std::fs::create_dir_all(&machine)?;
    let path = machine.join("device.toml");
    std::fs::write(
        &path,
        "version = 0\nid = \"0193a8d0-000d-7000-8000-000000000001\"\n",
    )?;
    let ids = SequentialIds::starting_at(&at("2026-11-26T09:00[Europe/Berlin]")?, 7);

    let error = device_id(&machine, &ids)
        .err()
        .ok_or("an older version must stop pace")?;

    assert_eq!(
        error.to_string(),
        format!("this pace reads version 1 of {}, not 0", path.display())
    );
    assert_eq!(
        error.hint(),
        Some(
            format!(
                "remove {} and pace gives this machine a new device ID",
                path.display()
            )
            .as_str()
        )
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
            "laptop".to_owned(),
            clock.clone(),
            Split::Month,
        ),
        clock.clone(),
        Arc::new(SequentialIds::starting_at(&clock.now(), seed)),
        Policy::default(),
        Catalog::default(),
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

    _ = laptop.run(&Command::Begin(Begin::default()))?;
    clock.set(at("2026-11-26T10:00[Europe/Berlin]")?);
    _ = copy.run(&Command::End(End::default()))?;
    clock.set(at("2026-11-26T11:00[Europe/Berlin]")?);

    let error = laptop
        .run(&Command::Begin(Begin::default()))
        .err()
        .ok_or("the laptop must notice the copy")?;

    assert_eq!(
        error.to_string(),
        "another machine writes with this machine's device ID: the log of this ID reaches \
         2026-11-26T09:00:00.000Z/0, but this machine wrote up to 2026-11-26T08:00:00.000Z/0"
    );
    assert_eq!(
        error.hint(),
        Some(
            format!(
                "if this machine was restored from a backup or copied, remove {} so it gets a new device ID",
                dir.path().join("laptop").join("device.toml").display()
            )
            .as_str()
        )
    );
    let state = laptop.state(&TimeRange::new(None, None)?)?;
    assert_eq!(state.entries().count(), 1, "nothing more was written");
    Ok(())
}

/// The latest HLC of `device`'s own log in `home`, read as `pace doctor`
/// reads it: every event file through the collecting reader.
fn own_last(
    home: &std::path::Path,
    device: DeviceId,
) -> Result<Option<pace_core::Hlc>, Box<dyn std::error::Error>> {
    let mut last = None;
    for (of, path) in event_file_paths(home)? {
        let text = std::fs::read_to_string(&path)?;
        let read = parse_events_collecting(&path, &text, of);
        last = last.max(
            read.events
                .iter()
                .filter(|event| event.device == device)
                .map(|event| event.hlc)
                .max(),
        );
    }
    Ok(last)
}

#[test]
fn clone_check_without_append_names_machine_left_behind() -> TestResult {
    let dir = scratch_dir()?;
    let home = dir.path().join("home");
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let device: DeviceId = "0193a8d0-000d-7000-8000-000000000001".parse()?;
    let laptop = machine(&home, &dir.path().join("laptop"), device, &clock, 1);
    let copy = machine(&home, &dir.path().join("copy"), device, &clock, 2);
    _ = laptop.run(&Command::Begin(Begin::default()))?;
    clock.set(at("2026-11-26T10:00[Europe/Berlin]")?);
    _ = copy.run(&Command::End(End::default()))?;

    let last = own_last(&home, device)?;
    let laptop_dir = dir.path().join("laptop");
    let copy_dir = dir.path().join("copy");
    let found = detect_cloned_device(&laptop_dir, last, last_written_hlc(&laptop_dir)?)
        .ok_or("the laptop's log is ahead of what it wrote")?;

    assert!(found.is_code("pace::store::device-cloned"));
    assert!(
        detect_cloned_device(&copy_dir, last, last_written_hlc(&copy_dir)?).is_none(),
        "the copy wrote the last event itself"
    );
    Ok(())
}

#[test]
fn index_deleted_is_rebuilt_to_same_state() -> TestResult {
    let dir = scratch_dir()?;
    let home = dir.path().join("home");
    let machine_dir = dir.path().join("machine");
    let clock = Arc::new(TestClock::new(at("2026-10-15T09:00[Europe/Berlin]")?));
    let device: DeviceId = "0193a8d0-000d-7000-8000-000000000001".parse()?;
    let tracker = machine(&home, &machine_dir, device, &clock, 1);
    _ = tracker.run(&Command::Begin(Begin::default()))?;
    clock.set(at("2026-11-26T09:00[Europe/Berlin]")?);
    let today = TimeRange::new(
        Some(at("2026-11-26T00:00[Europe/Berlin]")?),
        Some(at("2026-11-27T00:00[Europe/Berlin]")?),
    )?;
    let before = tracker.state(&today)?;

    std::fs::remove_file(machine_dir.join("index.json"))?;
    let after = tracker.state(&today)?;

    assert_eq!(before, after);
    assert_eq!(after.open().count(), 1);
    Ok(())
}

/// A `Tracker` over the files of the device of these tests, standing for
/// the next run of pace: nothing it reads is parsed in this process yet.
fn next_run(
    home: &std::path::Path,
    machine_dir: &std::path::Path,
    clock: &Arc<TestClock>,
) -> Result<Tracker<FilesStore>, Box<dyn std::error::Error>> {
    let device: DeviceId = "0193a8d0-000d-7000-8000-000000000001".parse()?;
    Ok(machine(home, machine_dir, device, clock, 2))
}

/// The index of `machine_dir` after a `begin` at 09:00 and an `end` at
/// 10:00 on 2026-11-26, with the time range of that day.
fn index_after_append(
    home: &std::path::Path,
    machine_dir: &std::path::Path,
    clock: &Arc<TestClock>,
) -> Result<(String, TimeRange), Box<dyn std::error::Error>> {
    let device: DeviceId = "0193a8d0-000d-7000-8000-000000000001".parse()?;
    let tracker = machine(home, machine_dir, device, clock, 1);
    _ = tracker.run(&Command::Begin(Begin::default()))?;
    clock.set(at("2026-11-26T10:00[Europe/Berlin]")?);
    _ = tracker.run(&Command::End(End::default()))?;
    let today = TimeRange::new(
        Some(at("2026-11-26T00:00[Europe/Berlin]")?),
        Some(at("2026-11-27T00:00[Europe/Berlin]")?),
    )?;
    Ok((
        std::fs::read_to_string(machine_dir.join("index.json"))?,
        today,
    ))
}

#[test]
fn append_leaves_index_that_next_run_keeps() -> TestResult {
    let dir = scratch_dir()?;
    let home = dir.path().join("home");
    let machine_dir = dir.path().join("machine");
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let (written, today) = index_after_append(&home, &machine_dir, &clock)?;
    let modified = std::fs::metadata(machine_dir.join("index.json"))?.modified()?;

    _ = next_run(&home, &machine_dir, &clock)?.state(&today)?;

    assert_eq!(
        std::fs::read_to_string(machine_dir.join("index.json"))?,
        written
    );
    assert_eq!(
        std::fs::metadata(machine_dir.join("index.json"))?.modified()?,
        modified
    );
    Ok(())
}

#[test]
fn index_saved_by_append_equals_rebuilt_index() -> TestResult {
    let dir = scratch_dir()?;
    let home = dir.path().join("home");
    let machine_dir = dir.path().join("machine");
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let (written, today) = index_after_append(&home, &machine_dir, &clock)?;

    std::fs::remove_file(machine_dir.join("index.json"))?;
    _ = next_run(&home, &machine_dir, &clock)?.state(&today)?;

    assert_eq!(
        std::fs::read_to_string(machine_dir.join("index.json"))?,
        written
    );
    Ok(())
}

#[test]
fn clock_reading_out_of_range_in_cursors_stops_writing() -> TestResult {
    let dir = scratch_dir()?;
    let home = dir.path().join("home");
    let machine_dir = dir.path().join("machine");
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let device: DeviceId = "0193a8d0-000d-7000-8000-000000000001".parse()?;
    let tracker = machine(&home, &machine_dir, device, &clock, 1);
    _ = tracker.run(&Command::Begin(Begin::default()))?;
    std::fs::write(
        machine_dir.join("cursors.toml"),
        "version = 1\n\n[own]\nlast-hlc = [9223372036854775807, 0]\n",
    )?;
    clock.set(at("2026-11-26T10:00[Europe/Berlin]")?);

    let error = tracker
        .run(&Command::End(End::default()))
        .err()
        .ok_or("an unknown clock reading must not turn the clone check off")?;

    assert_eq!(
        error.to_string(),
        format!(
            "{} holds the clock reading 9223372036854775807 ms, outside the range pace knows",
            machine_dir.join("cursors.toml").display()
        )
    );
    Ok(())
}

#[test]
fn clock_reading_out_of_range_in_index_stops_writing() -> TestResult {
    let dir = scratch_dir()?;
    let home = dir.path().join("home");
    let machine_dir = dir.path().join("machine");
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let device: DeviceId = "0193a8d0-000d-7000-8000-000000000001".parse()?;
    let tracker = machine(&home, &machine_dir, device, &clock, 1);
    _ = tracker.run(&Command::Begin(Begin::default()))?;
    _ = tracker.state(&TimeRange::everything())?;
    let index = machine_dir.join("index.json");
    let mut json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&index)?)?;
    let files = json
        .get_mut("files")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("the index lists its files")?;
    for summary in files.values_mut() {
        summary["max_hlc"] = serde_json::json!([i64::MAX, 0]);
    }
    std::fs::write(&index, json.to_string())?;
    clock.set(at("2026-11-26T10:00[Europe/Berlin]")?);

    let error = tracker
        .run(&Command::End(End::default()))
        .err()
        .ok_or("an unknown clock reading must not turn the clone check off")?;

    assert_eq!(
        error.to_string(),
        format!(
            "{} holds the clock reading 9223372036854775807 ms, outside the range pace knows",
            index.display()
        )
    );
    Ok(())
}

#[rstest]
#[case::cursors("machine/cursors.toml")]
#[case::device_file("home/events/0193a8d0-000d-7000-8000-000000000001/device.toml")]
fn store_file_of_newer_pace_stops_writing(#[case] file: &str) -> TestResult {
    let dir = scratch_dir()?;
    let home = dir.path().join("home");
    let machine_dir = dir.path().join("machine");
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let device: DeviceId = "0193a8d0-000d-7000-8000-000000000001".parse()?;
    let tracker = machine(&home, &machine_dir, device, &clock, 1);
    _ = tracker.run(&Command::Begin(Begin::default()))?;
    let path = dir.path().join(file);
    let text = std::fs::read_to_string(&path)?;
    std::fs::write(&path, text.replace("version = 1", "version = 7"))?;
    clock.set(at("2026-11-26T10:00[Europe/Berlin]")?);

    let error = tracker
        .run(&Command::End(End::default()))
        .err()
        .ok_or("a newer version must stop pace")?;

    // With `/` as the separator, as `file` writes it, on every OS.
    assert_eq!(
        error.to_string().replace('\\', "/"),
        format!("this pace reads version 1 of {}, not 7", path.display()).replace('\\', "/")
    );
    assert_eq!(
        error.hint(),
        Some("a newer pace wrote this file; update pace")
    );
    Ok(())
}

#[rstest]
#[case::cursors(
    "machine/cursors.toml",
    "version = 1\nown = 3\n",
    "the cursors file {} is not valid"
)]
#[case::cursors_without_version(
    "machine/cursors.toml",
    "[own]\ncleaned = []\n",
    "the cursors file {} is not valid"
)]
#[case::device_file(
    "home/events/0193a8d0-000d-7000-8000-000000000001/device.toml",
    "version = 1\nname = 3\n",
    "the device file {} is not valid"
)]
#[case::stray_directory("home/events/notes/", "", "pace found no device ID in the name of {}")]
fn store_errors_start_with_fixed_phrase(
    #[case] file: &str,
    #[case] text: &str,
    #[case] message: &str,
) -> TestResult {
    let dir = scratch_dir()?;
    let home = dir.path().join("home");
    let machine_dir = dir.path().join("machine");
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let device: DeviceId = "0193a8d0-000d-7000-8000-000000000001".parse()?;
    let tracker = machine(&home, &machine_dir, device, &clock, 1);
    _ = tracker.run(&Command::Begin(Begin::default()))?;
    let path = dir.path().join(file);
    if let Some(stray) = file.strip_suffix('/') {
        std::fs::create_dir_all(dir.path().join(stray))?;
    } else {
        std::fs::write(&path, text)?;
    }
    clock.set(at("2026-11-26T10:00[Europe/Berlin]")?);

    let error = tracker
        .run(&Command::End(End::default()))
        .err()
        .ok_or("a broken store file must stop pace")?;

    // With `/` as the separator, as `file` writes it, on every OS.
    let shown = dir.path().join(file.trim_end_matches('/'));
    assert_eq!(
        error.to_string().replace('\\', "/"),
        message
            .replace("{}", &shown.display().to_string())
            .replace('\\', "/")
    );
    Ok(())
}

/// Tests that cost more than the others: four threads create the device ID
/// of a fresh machine directory at once, eight times, and each creation
/// syncs its file to disk. `.config/nextest.toml` runs them in the test group
/// `expensive`.
mod expensive {
    use super::*;

    #[test]
    fn first_runs_at_once_agree_on_one_device_id() -> TestResult {
        for round in 0..8u16 {
            let dir = scratch_dir()?;
            let machine = dir.path().join("machine");
            let start = at("2026-11-26T09:00[Europe/Berlin]")?;
            let barrier = std::sync::Barrier::new(4);
            let identities = std::thread::scope(|scope| {
                let runs: Vec<_> = (0..4u16)
                    .map(|run| {
                        let (machine, start, barrier) = (&machine, &start, &barrier);
                        scope.spawn(move || {
                            let ids = SequentialIds::starting_at(start, 10 * round + run + 1);
                            _ = barrier.wait();
                            device_id(machine, &ids)
                        })
                    })
                    .collect();
                runs.into_iter()
                    .map(|run| run.join().map_err(|_| "a first run panicked"))
                    .collect::<Result<Vec<_>, _>>()
            })?;
            let identities = identities.into_iter().collect::<Result<Vec<_>, _>>()?;

            let ids: std::collections::BTreeSet<DeviceId> =
                identities.iter().map(|identity| identity.id).collect();
            assert_eq!(ids.len(), 1, "round {round}: {identities:?}");
            let created = identities
                .iter()
                .filter(|identity| identity.created)
                .count();
            assert_eq!(created, 1, "round {round}: {identities:?}");
        }
        Ok(())
    }
}
