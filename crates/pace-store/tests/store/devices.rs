//! Two devices, each with its own `PACE_HOME`, synced by copying their
//! device directories, as Syncthing or git would.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use jiff::ZonedDateTime;
use pace_core::{
    Change, Clock, Command, DeviceId, EntryId, Policy, SequentialIds, State, Tracker,
    command::{Adjust, Begin, End, Forget, Hold, Log},
    testing::TestClock,
};
use pace_store::{FilesStore, Split, testing::scratch_dir};
use pace_time::TimeRange;
use tempfile::TempDir;

use crate::TestResult;

fn at(rfc9557: &str) -> Result<ZonedDateTime, jiff::Error> {
    rfc9557.parse()
}

fn berlin(time: &str) -> Result<ZonedDateTime, jiff::Error> {
    at(&format!("2026-11-26T{time}[Europe/Berlin]"))
}

struct Device {
    home: PathBuf,
    id: DeviceId,
    clock: Arc<TestClock>,
    tracker: Tracker<FilesStore>,
}

impl Device {
    fn new(
        root: &Path,
        name: &str,
        number: u16,
        split: Split,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let home = root.join(name).join("home");
        let id: DeviceId = format!("0193a8d0-000d-7000-8000-{number:012x}").parse()?;
        let clock = Arc::new(TestClock::new(berlin("08:00")?));
        let tracker = Tracker::new(
            FilesStore::new(
                home.clone(),
                root.join(name).join("machine"),
                id,
                clock.clone(),
                split,
            ),
            clock.clone(),
            Arc::new(SequentialIds::starting_at(&clock.now(), number)),
            Policy::default(),
            id,
        );
        Ok(Self {
            home,
            id,
            clock,
            tracker,
        })
    }

    fn run(&self, time: &str, command: &Command) -> TestResult {
        self.clock.set(berlin(time)?);
        _ = pollster::block_on(self.tracker.run(command))?;
        Ok(())
    }

    fn state(&self) -> Result<State, Box<dyn std::error::Error>> {
        Ok(pollster::block_on(
            self.tracker.state(&TimeRange::new(None, None)?),
        )?)
    }

    fn own_dir(&self) -> PathBuf {
        self.home.join("events").join(self.id.to_string())
    }

    /// The text of all files of this device's own log.
    fn own_text(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut text = String::new();
        read_all(&self.own_dir(), &mut text)?;
        Ok(text)
    }
}

fn read_all(dir: &Path, text: &mut String) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            read_all(&path, text)?;
        } else {
            text.push_str(&std::fs::read_to_string(&path)?);
        }
    }
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    if to.exists() {
        std::fs::remove_dir_all(to)?;
    }
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let path = entry?.path();
        let target = to.join(path.file_name().unwrap_or_default());
        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            _ = std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

/// Copy each device's own directory into the other's `PACE_HOME`.
fn sync(a: &Device, b: &Device) -> TestResult {
    for (from, to) in [(a, b), (b, a)] {
        if from.own_dir().exists() {
            copy_dir(
                &from.own_dir(),
                &to.home.join("events").join(from.id.to_string()),
            )?;
        }
    }
    Ok(())
}

struct Pair {
    _root: TempDir,
    laptop: Device,
    desktop: Device,
}

fn pair(desktop_split: Split) -> Result<Pair, Box<dyn std::error::Error>> {
    let root = scratch_dir()?;
    Ok(Pair {
        laptop: Device::new(root.path(), "laptop", 1, Split::Month)?,
        desktop: Device::new(root.path(), "desktop", 2, desktop_split)?,
        _root: root,
    })
}

fn only_entry(state: &State) -> Result<EntryId, Box<dyn std::error::Error>> {
    match state.entries().collect::<Vec<_>>().as_slice() {
        [entry] => Ok(entry.id),
        other => Err(format!("expected one entry, found {}", other.len()).into()),
    }
}

#[test]
fn after_sync_both_devices_see_the_same_state() -> TestResult {
    let Pair {
        laptop, desktop, ..
    } = pair(Split::Week)?;
    laptop.run("09:00", &Command::Begin(Begin::default()))?;
    sync(&laptop, &desktop)?;
    desktop.run("10:00", &Command::End(End::default()))?;
    sync(&laptop, &desktop)?;

    assert_eq!(laptop.state()?, desktop.state()?);
    assert_eq!(laptop.state()?.open().count(), 0);
    Ok(())
}

#[test]
fn changes_made_apart_merge_by_the_later_change_per_field() -> TestResult {
    let Pair {
        laptop, desktop, ..
    } = pair(Split::Month)?;
    laptop.run(
        "09:00",
        &Command::Log(Log::new(berlin("07:00")?, berlin("08:00")?)),
    )?;
    sync(&laptop, &desktop)?;
    let entry = only_entry(&laptop.state()?)?;

    // Apart: the desktop changes note and tags at 10:00, the laptop the note
    // at 10:05.
    desktop.run(
        "10:00",
        &Command::Adjust(Adjust {
            entry: Some(entry),
            note: Change::Set("from the desktop".to_owned()),
            tag: std::collections::BTreeSet::from(["review".parse()?]),
            ..Adjust::default()
        }),
    )?;
    laptop.run(
        "10:05",
        &Command::Adjust(Adjust {
            entry: Some(entry),
            note: Change::Set("from the laptop".to_owned()),
            ..Adjust::default()
        }),
    )?;
    sync(&laptop, &desktop)?;

    for state in [laptop.state()?, desktop.state()?] {
        let merged = state.entry(entry).ok_or("entry")?;
        assert_eq!(merged.note.as_deref(), Some("from the laptop"));
        assert_eq!(
            merged.tags,
            std::collections::BTreeSet::from(["review".parse()?])
        );
    }
    Ok(())
}

#[test]
fn entries_begun_apart_are_both_running_until_one_ends() -> TestResult {
    let Pair {
        laptop, desktop, ..
    } = pair(Split::Month)?;
    laptop.run("09:00", &Command::Begin(Begin::default()))?;
    desktop.run("09:30", &Command::Begin(Begin::default()))?;
    sync(&laptop, &desktop)?;

    assert_eq!(laptop.state()?.open().count(), 2);
    laptop.clock.set(berlin("10:00")?);
    let refused = pollster::block_on(laptop.tracker.run(&Command::Begin(Begin::default())));
    assert!(
        refused.is_err(),
        "begin must ask to end one of the two first"
    );

    let desktop_entry = laptop
        .state()?
        .open()
        .find(|entry| entry.device == desktop.id)
        .map(|entry| entry.id)
        .ok_or("the desktop's entry")?;
    laptop.run(
        "10:00",
        &Command::End(End {
            entry: Some(desktop_entry),
            ..End::default()
        }),
    )?;
    assert_eq!(laptop.state()?.open().count(), 1);
    Ok(())
}

#[test]
fn entries_logged_apart_can_overlap_and_pace_finds_them() -> TestResult {
    let Pair {
        laptop, desktop, ..
    } = pair(Split::Month)?;
    laptop.run(
        "18:00",
        &Command::Log(Log::new(berlin("09:00")?, berlin("10:00")?)),
    )?;
    desktop.run(
        "18:00",
        &Command::Log(Log::new(berlin("09:30")?, berlin("10:30")?)),
    )?;
    sync(&laptop, &desktop)?;

    let state = laptop.state()?;
    let now = berlin("18:00")?;
    let overlapping = state
        .overlapping(&berlin("09:00")?, &berlin("10:00")?, &[], &now)
        .count();
    assert_eq!(overlapping, 2);
    Ok(())
}

#[test]
fn forgotten_entry_leaves_the_own_files_of_every_device() -> TestResult {
    let Pair {
        laptop, desktop, ..
    } = pair(Split::Week)?;
    laptop.run(
        "09:00",
        &Command::Begin(Begin {
            note: Some("secret".to_owned()),
            ..Begin::default()
        }),
    )?;
    sync(&laptop, &desktop)?;
    desktop.run("09:30", &Command::Hold(Hold::default()))?;
    sync(&laptop, &desktop)?;
    let entry = only_entry(&laptop.state()?)?;
    let desktop_log_before_forget = std::fs::read_dir(desktop.own_dir())?.count();

    laptop.run("10:00", &Command::Forget(Forget { entry }))?;

    assert!(
        !laptop.own_text()?.contains("secret"),
        "the laptop cleans its own files at once"
    );
    assert!(
        laptop.own_text()?.contains("entry.forgotten"),
        "the tombstone stays"
    );
    sync(&laptop, &desktop)?;
    assert!(
        desktop.own_text()?.contains(&entry.to_string()),
        "until it writes, the desktop keeps its events"
    );

    desktop.run("11:00", &Command::Begin(Begin::default()))?;

    assert!(
        !desktop
            .own_text()?
            .contains(&format!("entry = \"{entry}\""))
    );
    assert!(desktop.state()?.entry(entry).is_none());
    assert!(std::fs::read_dir(desktop.own_dir())?.count() >= desktop_log_before_forget);
    Ok(())
}
