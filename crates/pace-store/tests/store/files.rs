//! The files backend: TOML event files, one directory per device, one file
//! per period.

use std::{path::Path, sync::Arc};

use jiff::ZonedDateTime;
use pace_core::testing::contract::run_store_contract;
use pace_core::{
    Clock, Command, DeviceId, EventStore, Policy, SequentialIds, Tracker,
    command::{Adjust, Begin, Catch, End, Hold, Log, Pause},
    testing::TestClock,
};
use pace_store::{FilesStore, Split};
use pace_time::TimeRange;
use rstest::rstest;
use tempfile::TempDir;

use crate::TestResult;

fn at(rfc9557: &str) -> Result<ZonedDateTime, jiff::Error> {
    rfc9557.parse()
}

fn device(number: u16) -> Result<DeviceId, Box<dyn std::error::Error>> {
    Ok(format!("0193a8d0-000d-7000-8000-{number:012x}").parse()?)
}

struct Setup {
    dir: TempDir,
    clock: Arc<TestClock>,
    device: DeviceId,
}

impl Setup {
    fn new(now: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            dir: tempfile::tempdir()?,
            clock: Arc::new(TestClock::new(at(now)?)),
            device: device(1)?,
        })
    }

    fn home(&self) -> std::path::PathBuf {
        self.dir.path().join("home")
    }

    fn store(&self, split: Split) -> FilesStore {
        FilesStore::new(
            self.home(),
            self.dir.path().join("machine"),
            self.device,
            self.clock.clone(),
            split,
        )
    }

    fn tracker(&self, split: Split) -> Tracker<FilesStore> {
        Tracker::new(
            self.store(split),
            self.clock.clone(),
            Arc::new(SequentialIds::starting_at(&self.clock.now(), 1)),
            Policy::default(),
            self.device,
        )
    }

    fn device_dir(&self) -> std::path::PathBuf {
        self.home().join("events").join(self.device.to_string())
    }

    /// The event files of this device, relative to its directory.
    fn files(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut files = Vec::new();
        collect(&self.device_dir(), &self.device_dir(), &mut files)?;
        files.sort();
        Ok(files)
    }
}

fn collect(root: &Path, dir: &Path, files: &mut Vec<String>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(root, &path, files)?;
        } else if let Ok(relative) = path.strip_prefix(root) {
            files.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

fn everything() -> Result<TimeRange, Box<dyn std::error::Error>> {
    Ok(TimeRange::new(None, None)?)
}

#[test]
fn the_files_backend_keeps_the_store_contract() -> TestResult {
    pollster::block_on(run_store_contract(|| {
        let dir = tempfile::tempdir().ok();
        move |device: DeviceId, clock: Arc<TestClock>| {
            let root = dir
                .as_ref()
                .map_or_else(std::env::temp_dir, |dir| dir.path().to_path_buf());
            FilesStore::new(
                root.join("home"),
                root.join("machine").join(device.to_string()),
                device,
                clock,
                Split::Month,
            )
        }
    }))?;
    Ok(())
}

#[rstest]
#[case::year(Split::Year, "2026-11-26T09:00[Europe/Berlin]", "2026.toml")]
#[case::month(Split::Month, "2026-11-26T09:00[Europe/Berlin]", "2026/2026-11.toml")]
#[case::week(Split::Week, "2026-11-26T09:00[Europe/Berlin]", "2026/2026-W48.toml")]
#[case::day(
    Split::Day,
    "2026-11-26T09:00[Europe/Berlin]",
    "2026/11/2026-11-26.toml"
)]
// 2027-01-01 is a Friday of ISO week 53 of 2026.
#[case::iso_week_year(Split::Week, "2027-01-01T09:00[Europe/Berlin]", "2026/2026-W53.toml")]
fn events_go_into_the_file_of_their_period(
    #[case] split: Split,
    #[case] now: &str,
    #[case] file: &str,
) -> TestResult {
    let setup = Setup::new(now)?;

    _ = pollster::block_on(setup.tracker(split).run(&Command::Begin(Begin::default())))?;

    let mut expected = vec!["device.toml".to_owned(), file.to_owned()];
    expected.sort();
    assert_eq!(setup.files()?, expected);
    Ok(())
}

#[test]
fn an_entrys_events_stay_in_the_file_of_its_first_start() -> TestResult {
    let setup = Setup::new("2026-10-31T23:00[Europe/Berlin]")?;
    let tracker = setup.tracker(Split::Month);
    _ = pollster::block_on(tracker.run(&Command::Begin(Begin::default())))?;

    setup.clock.set(at("2026-11-01T00:30[Europe/Berlin]")?);
    _ = pollster::block_on(tracker.run(&Command::End(End::default())))?;

    assert_eq!(setup.files()?, ["2026/2026-10.toml", "device.toml"]);
    Ok(())
}

#[test]
fn the_event_file_reads_like_the_plan() -> TestResult {
    let setup = Setup::new("2026-11-26T09:02:11[Europe/Berlin]")?;
    let tracker = setup.tracker(Split::Month);
    let run = |now: &str, command: Command| -> TestResult {
        setup.clock.set(at(now)?);
        _ = pollster::block_on(tracker.run(&command))?;
        Ok(())
    };
    run(
        "2026-11-26T09:02:11[Europe/Berlin]",
        Command::Begin(Begin {
            note: Some("Checkout flow".to_owned()),
            project: Some(pace_core::ProjectRef::Key("acme-webshop".parse()?)),
            tags: std::collections::BTreeSet::from(["dev".parse()?]),
            ..Begin::default()
        }),
    )?;
    run(
        "2026-11-26T10:15:40[Europe/Berlin]",
        Command::Hold(Hold {
            source: Some(pace_core::HoldSource::Outer),
            reason: Some("phone call".to_owned()),
            ..Hold::default()
        }),
    )?;
    run(
        "2026-11-26T10:30:00[Europe/Berlin]",
        Command::Pause(Pause {
            kind: pace_core::PauseKind::Lunch,
            ..Pause::default()
        }),
    )?;
    run(
        "2026-11-26T11:00:00[Europe/Berlin]",
        Command::Catch(Catch {
            text: "call the tax office".to_owned(),
        }),
    )?;
    run(
        "2026-11-26T11:05:00[Europe/Berlin]",
        Command::Adjust(Adjust {
            note: pace_core::Change::Clear,
            tag: std::collections::BTreeSet::from(["review".parse()?]),
            ..Adjust::default()
        }),
    )?;
    run(
        "2026-11-26T12:00:00[Europe/Berlin]",
        Command::End(End::default()),
    )?;
    run(
        "2026-11-26T18:00:00[Europe/Berlin]",
        Command::Log(Log::new(
            at("2026-11-26T14:00[Europe/Berlin]")?,
            at("2026-11-26T11:30[America/New_York]")?,
        )),
    )?;
    run("2026-11-26T18:01:00[Europe/Berlin]", Command::Undo)?;

    let text = std::fs::read_to_string(setup.device_dir().join("2026/2026-11.toml"))?;
    insta::assert_snapshot!(text);
    Ok(())
}

#[test]
fn what_was_written_reads_back_the_same() -> TestResult {
    let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;
    let tracker = setup.tracker(Split::Month);
    let written = pollster::block_on(tracker.run(&Command::Begin(Begin {
        note: Some("a \"quoted\" note\nwith two lines".to_owned()),
        ..Begin::default()
    })))?;

    let read = pollster::block_on(setup.store(Split::Month).read(&everything()?))?;

    assert_eq!(read, written.recorded);
    Ok(())
}

#[test]
fn the_device_file_names_the_device_and_its_split() -> TestResult {
    let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;
    _ = pollster::block_on(
        setup
            .tracker(Split::Week)
            .run(&Command::Begin(Begin::default())),
    )?;

    let devices = pollster::block_on(setup.store(Split::Week).devices())?;
    let text = std::fs::read_to_string(setup.device_dir().join("device.toml"))?;

    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].id, setup.device);
    assert_eq!(devices[0].created, at("2026-11-26T09:00[Europe/Berlin]")?);
    assert_ne!(devices[0].name, "");
    assert!(text.contains("split = \"week\""), "{text}");
    Ok(())
}

#[test]
fn appends_from_two_processes_both_land() -> TestResult {
    let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;
    let tracker = setup.tracker(Split::Month);
    _ = pollster::block_on(tracker.run(&Command::Log(Log::new(
        at("2026-11-26T07:00[Europe/Berlin]")?,
        at("2026-11-26T08:00[Europe/Berlin]")?,
    ))))?;
    let entry = pollster::block_on(tracker.state(&everything()?))?
        .entries()
        .next()
        .map(|entry| entry.id)
        .ok_or("entry")?;

    std::thread::scope(|scope| {
        for writer in 0..4u16 {
            let store = setup.store(Split::Month);
            let clock = setup.clock.clone();
            let device = setup.device;
            _ = scope.spawn(move || {
                let tracker = Tracker::new(
                    store,
                    clock.clone(),
                    Arc::new(SequentialIds::starting_at(&clock.now(), 100 + writer)),
                    Policy::default(),
                    device,
                );
                for round in 0..10 {
                    _ = pollster::block_on(tracker.run(&Command::Adjust(Adjust {
                        entry: Some(entry),
                        note: pace_core::Change::Set(format!("{writer}/{round}")),
                        ..Adjust::default()
                    })));
                }
            });
        }
    });

    let read = pollster::block_on(setup.store(Split::Month).read(&everything()?))?;
    assert_eq!(read.len(), 1 + 4 * 10);
    let leftovers: Vec<_> = setup
        .files()?
        .into_iter()
        .filter(|file| {
            file.rsplit('/')
                .next()
                .is_some_and(|name| name.starts_with(".pace-"))
        })
        .collect();
    assert_eq!(
        leftovers,
        Vec::<String>::new(),
        "temporary files were left behind"
    );
    Ok(())
}

#[test]
fn an_unknown_key_in_an_event_file_names_the_file_and_line() -> TestResult {
    let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;
    _ = pollster::block_on(
        setup
            .tracker(Split::Month)
            .run(&Command::Begin(Begin::default())),
    )?;
    let file = setup.device_dir().join("2026/2026-11.toml");
    let text = std::fs::read_to_string(&file)?;
    std::fs::write(
        &file,
        text.replace(
            "type = \"entry.begun\"",
            "type = \"entry.begun\"\nnotes = \"x\"",
        ),
    )?;

    let error = pollster::block_on(setup.store(Split::Month).read(&everything()?))
        .err()
        .ok_or("the broken file must fail")?;

    assert_eq!(
        error.to_string(),
        "`notes` is not a key pace knows in an event"
    );
    assert!(error.hint().is_some_and(|hint| hint.contains("pace edit")));
    let report = format!("{error:?}");
    assert!(report.contains("2026-11.toml"), "{report}");
    Ok(())
}

#[test]
fn an_event_missing_field_is_named() -> TestResult {
    let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;
    _ = pollster::block_on(
        setup
            .tracker(Split::Month)
            .run(&Command::Begin(Begin::default())),
    )?;
    let file = setup.device_dir().join("2026/2026-11.toml");
    let text = std::fs::read_to_string(&file)?;
    let without_entry: String = text
        .split_inclusive('\n')
        .filter(|line| !line.starts_with("entry = "))
        .collect();
    std::fs::write(&file, without_entry)?;

    let error = pollster::block_on(setup.store(Split::Month).read(&everything()?))
        .err()
        .ok_or("the broken file must fail")?;

    assert_eq!(error.to_string(), "an `entry.begun` event needs `entry`");
    Ok(())
}

#[test]
fn an_event_type_from_newer_pace_asks_for_an_update() -> TestResult {
    let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;
    _ = pollster::block_on(
        setup
            .tracker(Split::Month)
            .run(&Command::Begin(Begin::default())),
    )?;
    let file = setup.device_dir().join("2026/2026-11.toml");
    let text = std::fs::read_to_string(&file)?;
    std::fs::write(
        &file,
        text.replace("\"entry.begun\"", "\"entry.teleported\""),
    )?;

    let error = pollster::block_on(setup.store(Split::Month).read(&everything()?))
        .err()
        .ok_or("the unknown type must fail")?;

    assert_eq!(
        error.to_string(),
        "pace doesn't know the event type `entry.teleported`"
    );
    assert_eq!(
        error.hint(),
        Some("a newer pace may have written it on another device; update pace")
    );
    Ok(())
}

/// A device with entries in October and November, split by month.
fn two_months(setup: &Setup) -> TestResult {
    let tracker = setup.tracker(Split::Month);
    setup.clock.set(at("2026-10-30T09:00[Europe/Berlin]")?);
    _ = pollster::block_on(tracker.run(&Command::Begin(Begin::default())))?;
    setup.clock.set(at("2026-10-30T10:00[Europe/Berlin]")?);
    _ = pollster::block_on(tracker.run(&Command::End(End::default())))?;
    setup.clock.set(at("2026-11-26T09:00[Europe/Berlin]")?);
    _ = pollster::block_on(tracker.run(&Command::Begin(Begin::default())))?;
    Ok(())
}

#[test]
fn resplitting_rewrites_this_devices_files_and_keeps_the_events() -> TestResult {
    let setup = Setup::new("2026-10-30T09:00[Europe/Berlin]")?;
    two_months(&setup)?;
    let before = pollster::block_on(setup.store(Split::Month).read(&everything()?))?;

    let resplit = setup.store(Split::Month).resplit(Split::Day)?;

    assert_eq!((resplit.from, resplit.to), (Split::Month, Split::Day));
    assert_eq!(
        (resplit.files_before, resplit.files_after, resplit.events),
        (2, 2, 3)
    );
    assert_eq!(
        setup.files()?,
        [
            "2026/10/2026-10-30.toml",
            "2026/11/2026-11-26.toml",
            "device.toml"
        ]
    );
    let after = pollster::block_on(setup.store(Split::Month).read(&everything()?))?;
    assert_eq!(after, before);
    let device_file = std::fs::read_to_string(setup.device_dir().join("device.toml"))?;
    assert!(device_file.contains("split = \"day\""), "{device_file}");
    Ok(())
}

#[test]
fn after_resplitting_new_events_go_into_the_new_files() -> TestResult {
    let setup = Setup::new("2026-10-30T09:00[Europe/Berlin]")?;
    two_months(&setup)?;
    _ = setup.store(Split::Month).resplit(Split::Year)?;

    setup.clock.set(at("2026-11-26T10:00[Europe/Berlin]")?);
    _ = pollster::block_on(
        setup
            .tracker(Split::Month)
            .run(&Command::End(End::default())),
    )?;

    assert_eq!(setup.files()?, ["2026.toml", "device.toml"]);
    Ok(())
}

#[test]
fn resplitting_to_the_same_split_changes_nothing() -> TestResult {
    let setup = Setup::new("2026-10-30T09:00[Europe/Berlin]")?;
    two_months(&setup)?;

    let resplit = setup.store(Split::Month).resplit(Split::Month)?;

    assert_eq!(
        (resplit.from, resplit.to, resplit.events),
        (Split::Month, Split::Month, 0)
    );
    assert_eq!(
        setup.files()?,
        ["2026/2026-10.toml", "2026/2026-11.toml", "device.toml"]
    );
    Ok(())
}

#[test]
fn resplitting_leaves_other_devices_alone() -> TestResult {
    let setup = Setup::new("2026-10-30T09:00[Europe/Berlin]")?;
    two_months(&setup)?;
    let other = device(2)?;
    let other_store = FilesStore::new(
        setup.home(),
        setup.dir.path().join("other-machine"),
        other,
        setup.clock.clone(),
        Split::Month,
    );
    let other_tracker = Tracker::new(
        other_store,
        setup.clock.clone(),
        Arc::new(SequentialIds::starting_at(&setup.clock.now(), 2)),
        Policy::default(),
        other,
    );
    setup.clock.set(at("2026-11-26T09:30[Europe/Berlin]")?);
    let other_dir = setup.home().join("events").join(other.to_string());
    _ = pollster::block_on(other_tracker.run(&Command::Log(Log::new(
        at("2026-11-25T09:00[Europe/Berlin]")?,
        at("2026-11-25T10:00[Europe/Berlin]")?,
    ))))?;

    _ = setup.store(Split::Month).resplit(Split::Week)?;

    let mut files = Vec::new();
    collect(&other_dir, &other_dir, &mut files)?;
    files.sort();
    assert_eq!(files, ["2026/2026-11.toml", "device.toml"]);
    Ok(())
}

#[test]
fn device_without_events_takes_the_new_split_for_its_first() -> TestResult {
    let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;

    let resplit = setup.store(Split::Month).resplit(Split::Week)?;

    assert_eq!(
        (resplit.from, resplit.to, resplit.events),
        (Split::Month, Split::Week, 0)
    );
    _ = pollster::block_on(
        setup
            .tracker(Split::Month)
            .run(&Command::Begin(Begin::default())),
    )?;
    assert_eq!(setup.files()?, ["2026/2026-W48.toml", "device.toml"]);
    Ok(())
}

#[test]
fn an_event_in_two_files_is_read_once() -> TestResult {
    let setup = Setup::new("2026-10-30T09:00[Europe/Berlin]")?;
    two_months(&setup)?;
    // What a crash in the middle of a resplit leaves: the same events under
    // the old and the new file name.
    _ = std::fs::copy(
        setup.device_dir().join("2026/2026-10.toml"),
        setup.device_dir().join("2026/2026-W44.toml"),
    )?;

    let events = pollster::block_on(setup.store(Split::Month).read(&everything()?))?;

    assert_eq!(events.len(), 3);
    Ok(())
}
