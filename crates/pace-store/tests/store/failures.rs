//! A write that fails half way leaves a state that pace goes on from: the
//! next command works, and nothing calls the machine a clone.

#![cfg(unix)]

use std::{os::unix::fs::PermissionsExt as _, path::Path, sync::Arc};

use jiff::ZonedDateTime;
use pace_core::{
    Clock, Command, DeviceId, Policy, SequentialIds,
    command::{Begin, End, Forget, Hold, Log},
    testing::TestClock,
};
use pace_store::{FilesStore, Split, Tracker, testing::scratch_dir};
use pace_time::TimeRange;

use crate::TestResult;

fn at(rfc9557: &str) -> Result<ZonedDateTime, jiff::Error> {
    rfc9557.parse()
}

fn device(number: u16) -> Result<DeviceId, Box<dyn std::error::Error>> {
    Ok(format!("0193a8d0-000d-7000-8000-{number:012x}").parse()?)
}

fn tracker(
    home: &Path,
    machine: &Path,
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
            Split::Day,
        ),
        clock.clone(),
        Arc::new(SequentialIds::starting_at(&clock.now(), seed)),
        Policy::default(),
        device,
    )
}

fn read_only(path: &Path) -> std::io::Result<()> {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o555))
}

fn writable(path: &Path) -> std::io::Result<()> {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}

/// The text of every event file below `dir`.
fn text_below(dir: &Path) -> std::io::Result<String> {
    let mut text = String::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            text.push_str(&text_below(&path)?);
        } else {
            text.push_str(&std::fs::read_to_string(&path)?);
        }
    }
    Ok(text)
}

#[test]
fn failed_cursor_save_records_nothing_and_next_command_works() -> TestResult {
    let dir = scratch_dir()?;
    let (home, machine) = (dir.path().join("home"), dir.path().join("machine"));
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let laptop = tracker(&home, &machine, device(1)?, &clock, 1);
    _ = laptop.run(&Command::Begin(Begin::default()))?;
    // A read brings the index up to date, as the command's output does.
    _ = laptop.state(&TimeRange::new(None, None)?)?;
    read_only(&machine)?;
    clock.set(at("2026-11-26T10:00[Europe/Berlin]")?);

    let held = laptop.run(&Command::Hold(Hold::default()));
    writable(&machine)?;

    let error = held.err().ok_or("the cursors can't be saved")?;
    assert_eq!(
        error.to_string(),
        format!("cannot create a temporary file in {}", machine.display())
    );
    assert_eq!(
        error.hint(),
        Some("check that the directory exists and that pace may write there")
    );
    assert!(
        !text_below(&home.join("events"))?.contains("entry.held"),
        "a failed append records nothing"
    );
    clock.set(at("2026-11-26T11:00[Europe/Berlin]")?);
    _ = laptop.run(&Command::End(End::default()))?;
    let state = laptop.state(&TimeRange::new(None, None)?)?;
    assert_eq!(state.open().count(), 0);
    Ok(())
}

#[test]
fn failed_cleaning_keeps_the_append_and_is_retried() -> TestResult {
    let dir = scratch_dir()?;
    let home = dir.path().join("home");
    let clock = Arc::new(TestClock::new(at("2026-10-30T12:00[Europe/Berlin]")?));
    let laptop = tracker(&home, &dir.path().join("laptop"), device(1)?, &clock, 1);
    let desktop = tracker(&home, &dir.path().join("desktop"), device(2)?, &clock, 2);
    let mut secret = Log::new(
        at("2026-10-30T09:00[Europe/Berlin]")?,
        at("2026-10-30T10:00[Europe/Berlin]")?,
    );
    secret.note = Some("secret".parse()?);
    _ = laptop.run(&Command::Log(secret))?;
    let entry = laptop
        .state(&TimeRange::new(None, None)?)?
        .entries()
        .next()
        .map(|entry| entry.id)
        .ok_or("the logged entry")?;
    clock.set(at("2026-11-26T09:00[Europe/Berlin]")?);
    _ = desktop.run(&Command::Forget(Forget { entry }))?;
    let october = home
        .join("events")
        .join(device(1)?.to_string())
        .join("2026/10");
    read_only(&october)?;

    let begun = laptop.run(&Command::Begin(Begin::default()));
    writable(&october)?;

    let begun = begun?;
    assert_eq!(begun.recorded.len(), 1, "the begin is recorded");
    let [warning] = begun.warnings.as_slice() else {
        return Err(format!("one warning, not {:?}", begun.warnings).into());
    };
    assert_eq!(
        warning.to_string(),
        format!(
            "pace recorded the command but kept the events of the forgotten entry {entry} in this device's files"
        )
    );
    assert!(text_below(&october)?.contains("secret"));
    clock.set(at("2026-11-26T10:00[Europe/Berlin]")?);
    let ended = laptop.run(&Command::End(End::default()))?;
    assert!(
        !text_below(&home.join("events").join(device(1)?.to_string()))?.contains("secret"),
        "the next append cleans the file"
    );
    assert!(ended.warnings.is_empty(), "{:?}", ended.warnings);
    Ok(())
}

#[test]
fn read_works_when_index_cannot_be_written() -> TestResult {
    let dir = scratch_dir()?;
    let (home, machine) = (dir.path().join("home"), dir.path().join("machine"));
    let clock = Arc::new(TestClock::new(at("2026-11-26T09:00[Europe/Berlin]")?));
    let laptop = tracker(&home, &machine, device(1)?, &clock, 1);
    // The append changes the event file after the index was refreshed, so
    // the next read refreshes it again.
    _ = laptop.run(&Command::Begin(Begin::default()))?;
    read_only(&machine)?;

    let state = laptop.state(&TimeRange::new(None, None)?);
    writable(&machine)?;

    assert_eq!(state?.open().count(), 1);
    Ok(())
}
