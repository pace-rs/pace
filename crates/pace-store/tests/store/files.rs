//! The files backend: TOML event files, one directory per device, one file
//! per period.

use std::{path::Path, sync::Arc};

use jiff::ZonedDateTime;
use pace_core::{
    Clock, Command, DeviceId, Policy, ReadScope, SequentialIds,
    command::{Adjust, Begin, Catch, End, Forget, Hold, Log, Pause},
    testing::TestClock,
};
use pace_store::{
    EventStore, FilesStore, Split, Tracker,
    testing::{Raced, contract::run_store_contract, scratch_dir},
};
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
            dir: scratch_dir()?,
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
            "laptop".to_owned(),
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

#[rstest]
#[case::month_on_both(Split::Month, Split::Month)]
#[case::day_beside_month(Split::Day, Split::Month)]
#[case::month_beside_day(Split::Month, Split::Day)]
fn the_files_backend_keeps_the_store_contract(
    #[case] first_split: Split,
    #[case] other_split: Split,
) -> TestResult {
    let first = device(1)?;
    run_store_contract(|| {
        let dir = scratch_dir().expect("a temporary directory for the store");
        move |device: DeviceId, clock: Arc<TestClock>| {
            let root = dir.path();
            FilesStore::new(
                root.join("home"),
                root.join("machine").join(device.to_string()),
                device,
                "laptop".to_owned(),
                clock,
                if device == first {
                    first_split
                } else {
                    other_split
                },
            )
        }
    })?;
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

    _ = setup
        .tracker(split)
        .run(&Command::Begin(Begin::default()))?;

    let mut expected = vec!["device.toml".to_owned(), file.to_owned()];
    expected.sort();
    assert_eq!(setup.files()?, expected);
    Ok(())
}

#[test]
fn an_entrys_events_stay_in_the_file_of_its_first_start() -> TestResult {
    let setup = Setup::new("2026-10-31T23:00[Europe/Berlin]")?;
    let tracker = setup.tracker(Split::Month);
    _ = tracker.run(&Command::Begin(Begin::default()))?;

    setup.clock.set(at("2026-11-01T00:30[Europe/Berlin]")?);
    _ = tracker.run(&Command::End(End::default()))?;

    assert_eq!(setup.files()?, ["2026/2026-10.toml", "device.toml"]);
    Ok(())
}

#[test]
fn the_event_file_reads_like_the_plan() -> TestResult {
    let setup = Setup::new("2026-11-26T09:02:11[Europe/Berlin]")?;
    let tracker = setup.tracker(Split::Month);
    let run = |now: &str, command: Command| -> TestResult {
        setup.clock.set(at(now)?);
        _ = tracker.run(&command)?;
        Ok(())
    };
    run(
        "2026-11-26T09:02:11[Europe/Berlin]",
        Command::Begin(Begin {
            note: Some("Checkout flow".parse()?),
            project: Some(pace_core::ProjectRef::Key("acme-webshop".parse()?)),
            tags: std::collections::BTreeSet::from(["dev".parse()?]),
            ..Begin::default()
        }),
    )?;
    run(
        "2026-11-26T10:15:40[Europe/Berlin]",
        Command::Hold(Hold {
            source: Some(pace_core::HoldSource::Outer),
            reason: Some("phone call".parse()?),
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
            text: "call the tax office".parse()?,
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
    let written = tracker.run(&Command::Begin(Begin {
        note: Some("a \"quoted\" note\nwith two lines".parse()?),
        ..Begin::default()
    }))?;

    let read = setup.store(Split::Month).read(&ReadScope::everything())?;

    assert_eq!(read, written.recorded);
    // `ZonedDateTime` equality compares the instant only; the debug text
    // holds the time zone of each timestamp too.
    assert_eq!(format!("{read:#?}"), format!("{:#?}", written.recorded));
    Ok(())
}

#[test]
fn the_device_file_names_the_device_and_its_split() -> TestResult {
    let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;
    _ = setup
        .tracker(Split::Week)
        .run(&Command::Begin(Begin::default()))?;

    let devices = setup.store(Split::Week).devices()?;
    let text = std::fs::read_to_string(setup.device_dir().join("device.toml"))?;

    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].id, setup.device);
    assert_eq!(devices[0].created, at("2026-11-26T09:00[Europe/Berlin]")?);
    assert_eq!(devices[0].name, "laptop");
    assert!(text.contains("split = \"week\""), "{text}");
    Ok(())
}

#[test]
fn an_unknown_key_in_an_event_file_names_the_file_and_line() -> TestResult {
    let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;
    _ = setup
        .tracker(Split::Month)
        .run(&Command::Begin(Begin::default()))?;
    let file = setup.device_dir().join("2026/2026-11.toml");
    let text = std::fs::read_to_string(&file)?;
    std::fs::write(
        &file,
        text.replace(
            "type = \"entry.begun\"",
            "type = \"entry.begun\"\nnotes = \"x\"",
        ),
    )?;

    let error = setup
        .store(Split::Month)
        .read(&ReadScope::everything())
        .err()
        .ok_or("the broken file must fail")?;

    assert_eq!(
        error.to_string(),
        "`notes` is not a key pace knows in an event"
    );
    assert_eq!(error.context_value("key"), Some("notes"));
    assert_eq!(error.kind(), pace_error::ErrorKind::InputOutput);
    assert!(error.hint().is_some_and(|hint| hint.contains("pace edit")));
    let report = format!("{error:?}");
    assert!(report.contains("2026-11.toml"), "{report}");
    Ok(())
}

#[test]
fn an_event_missing_field_is_named() -> TestResult {
    let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;
    _ = setup
        .tracker(Split::Month)
        .run(&Command::Begin(Begin::default()))?;
    let file = setup.device_dir().join("2026/2026-11.toml");
    let text = std::fs::read_to_string(&file)?;
    let without_entry: String = text
        .split_inclusive('\n')
        .filter(|line| !line.starts_with("entry = "))
        .collect();
    std::fs::write(&file, without_entry)?;

    let error = setup
        .store(Split::Month)
        .read(&ReadScope::everything())
        .err()
        .ok_or("the broken file must fail")?;

    assert_eq!(
        error.to_string(),
        "an event of type `entry.begun` needs `entry`"
    );
    Ok(())
}

/// The event file of November, cut to nothing or in the middle of a value.
fn cut(text: &str, empty: bool) -> Result<&str, Box<dyn std::error::Error>> {
    if empty {
        return Ok("");
    }
    let hlc = text.rfind("hlc = \"").ok_or("the file has an HLC")?;
    Ok(text.get(..hlc + 11).ok_or("the HLC is longer")?)
}

#[rstest]
#[case::empty(true, "missing field `version`")]
#[case::truncated(false, "invalid basic string")]
fn broken_event_file_keeps_its_cause_and_asks_to_repair_it(
    #[case] empty: bool,
    #[case] cause: &str,
) -> TestResult {
    let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;
    _ = setup
        .tracker(Split::Month)
        .run(&Command::Begin(Begin::default()))?;
    let file = setup.device_dir().join("2026/2026-11.toml");
    let text = std::fs::read_to_string(&file)?;
    std::fs::write(&file, cut(&text, empty)?)?;

    let error = setup
        .store(Split::Month)
        .read(&ReadScope::everything())
        .err()
        .ok_or("the broken file must fail")?;

    assert!(error.to_string().contains("2026-11.toml"), "{error}");
    let source = std::error::Error::source(&*error).ok_or("the TOML error is the cause")?;
    assert!(source.to_string().contains(cause), "{source}");
    let hint = error.hint().ok_or("the error says how to go on")?;
    assert!(!hint.contains("pace edit"), "{hint}");
    assert!(hint.contains("restore"), "{hint}");
    Ok(())
}

/// The line of the span an error's snippet points at, counted from 1.
fn blamed_line(error: &pace_error::PaceError, text: &str) -> Option<usize> {
    let label = miette::Diagnostic::labels(error)?.next()?;
    Some(text.get(..label.offset())?.matches('\n').count() + 1)
}

#[test]
fn an_invalid_event_is_blamed_on_its_own_line() -> TestResult {
    let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;
    let tracker = setup.tracker(Split::Month);
    _ = tracker.run(&Command::Begin(Begin {
        note: Some("agenda\n[[event]]\nminutes".parse()?),
        ..Begin::default()
    }))?;
    setup.clock.set(at("2026-11-26T10:00[Europe/Berlin]")?);
    _ = tracker.run(&Command::End(End::default()))?;
    let file = setup.device_dir().join("2026/2026-11.toml");
    let text = std::fs::read_to_string(&file)?;
    // The second event's header gets a comment, and the event loses its
    // entry; the note of the first holds a line `[[event]]`.
    let header = text.rfind("[[event]]\n").ok_or("the file has events")?;
    let (first, second) = text.split_at(header);
    let second: String = second
        .replacen("[[event]]", "[[event]] # the end", 1)
        .split_inclusive('\n')
        .filter(|line| !line.starts_with("entry = "))
        .collect();
    let broken = format!("{first}{second}");
    std::fs::write(&file, &broken)?;

    let error = setup
        .store(Split::Month)
        .read(&ReadScope::everything())
        .err()
        .ok_or("the broken event must fail")?;

    assert_eq!(
        error.to_string(),
        "an event of type `entry.ended` needs `entry`"
    );
    assert_eq!(
        blamed_line(&error, &broken),
        Some(first.matches('\n').count() + 1),
        "{broken}"
    );
    Ok(())
}

#[test]
fn an_event_type_from_newer_pace_asks_for_an_update() -> TestResult {
    let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;
    _ = setup
        .tracker(Split::Month)
        .run(&Command::Begin(Begin::default()))?;
    let file = setup.device_dir().join("2026/2026-11.toml");
    let text = std::fs::read_to_string(&file)?;
    std::fs::write(
        &file,
        text.replace("\"entry.begun\"", "\"entry.teleported\""),
    )?;

    let error = setup
        .store(Split::Month)
        .read(&ReadScope::everything())
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
    _ = tracker.run(&Command::Begin(Begin::default()))?;
    setup.clock.set(at("2026-10-30T10:00[Europe/Berlin]")?);
    _ = tracker.run(&Command::End(End::default()))?;
    setup.clock.set(at("2026-11-26T09:00[Europe/Berlin]")?);
    _ = tracker.run(&Command::Begin(Begin::default()))?;
    Ok(())
}

#[test]
fn resplitting_rewrites_this_devices_files_and_keeps_the_events() -> TestResult {
    let setup = Setup::new("2026-10-30T09:00[Europe/Berlin]")?;
    two_months(&setup)?;
    let before = setup.store(Split::Month).read(&ReadScope::everything())?;

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
    let after = setup.store(Split::Month).read(&ReadScope::everything())?;
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
    _ = setup
        .tracker(Split::Month)
        .run(&Command::End(End::default()))?;

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
        "laptop".to_owned(),
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
    _ = other_tracker.run(&Command::Log(Log::new(
        at("2026-11-25T09:00[Europe/Berlin]")?,
        at("2026-11-25T10:00[Europe/Berlin]")?,
    )))?;

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
    _ = setup
        .tracker(Split::Month)
        .run(&Command::Begin(Begin::default()))?;
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

    let events = setup.store(Split::Month).read(&ReadScope::everything())?;

    assert_eq!(events.len(), 3);
    Ok(())
}

#[test]
fn store_that_read_once_sees_what_another_process_appended() -> TestResult {
    let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;
    let reader = setup.store(Split::Month);
    _ = setup
        .tracker(Split::Month)
        .run(&Command::Begin(Begin::default()))?;
    let before = reader.read(&ReadScope::everything())?;
    setup.clock.set(at("2026-11-26T09:30[Europe/Berlin]")?);

    _ = setup
        .tracker(Split::Month)
        .run(&Command::End(End::default()))?;

    let after = reader.read(&ReadScope::everything())?;
    assert_eq!((before.len(), after.len()), (1, 2));
    Ok(())
}

#[test]
fn store_reads_back_what_it_wrote_like_fresh_store() -> TestResult {
    let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;
    let tracker = setup.tracker(Split::Month);
    _ = tracker.run(&Command::Begin(Begin {
        note: Some("Write offer".parse()?),
        ..Begin::default()
    }))?;
    setup.clock.set(at("2026-11-26T09:30[Europe/Berlin]")?);
    let state = tracker.state(&TimeRange::everything())?;
    let running = state.open().next().map(|entry| entry.id).ok_or("running")?;
    _ = tracker.run(&Command::Begin(Begin::default()))?;
    setup.clock.set(at("2026-11-26T09:45[Europe/Berlin]")?);

    _ = tracker.run(&Command::Forget(Forget { entry: running }))?;

    assert_eq!(
        tracker.store().read(&ReadScope::everything())?,
        setup.store(Split::Month).read(&ReadScope::everything())?
    );
    Ok(())
}

/// What a stop in the middle of a split to weeks leaves: the October events
/// under the week's file name too, and `device.toml` still on months.
fn interrupted_split_to_weeks(setup: &Setup) -> TestResult {
    two_months(setup)?;
    _ = std::fs::copy(
        setup.device_dir().join("2026/2026-10.toml"),
        setup.device_dir().join("2026/2026-W44.toml"),
    )?;
    Ok(())
}

/// The text of this device's event files, `device.toml` left out.
fn event_text(setup: &Setup) -> Result<String, Box<dyn std::error::Error>> {
    let mut text = String::new();
    for file in setup.files()? {
        if file != "device.toml" {
            text.push_str(&std::fs::read_to_string(setup.device_dir().join(file))?);
        }
    }
    Ok(text)
}

#[test]
fn resplitting_after_stop_keeps_each_event_once() -> TestResult {
    let setup = Setup::new("2026-10-30T09:00[Europe/Berlin]")?;
    interrupted_split_to_weeks(&setup)?;

    let resplit = setup.store(Split::Month).resplit(Split::Week)?;

    assert_eq!(resplit.events, 3);
    assert_eq!(
        setup.files()?,
        ["2026/2026-W44.toml", "2026/2026-W48.toml", "device.toml"]
    );
    assert_eq!(event_text(&setup)?.matches("[[event]]").count(), 3);
    Ok(())
}

#[test]
fn resplitting_to_same_split_removes_what_stop_left() -> TestResult {
    let setup = Setup::new("2026-10-30T09:00[Europe/Berlin]")?;
    interrupted_split_to_weeks(&setup)?;

    let resplit = setup.store(Split::Month).resplit(Split::Month)?;

    assert_eq!(
        (resplit.from, resplit.to, resplit.events),
        (Split::Month, Split::Month, 3)
    );
    assert_eq!(
        setup.files()?,
        ["2026/2026-10.toml", "2026/2026-11.toml", "device.toml"]
    );
    assert_eq!(event_text(&setup)?.matches("[[event]]").count(), 3);
    Ok(())
}

#[test]
fn forgotten_entry_leaves_every_own_file_that_holds_it() -> TestResult {
    let setup = Setup::new("2026-10-30T09:00[Europe/Berlin]")?;
    interrupted_split_to_weeks(&setup)?;
    let tracker = setup.tracker(Split::Month);
    let state = tracker.state(&TimeRange::everything())?;
    let october = state
        .entries()
        .find(|entry| state.open().all(|open| open.id != entry.id))
        .map(|entry| entry.id)
        .ok_or("the October entry")?;

    _ = tracker.run(&Command::Forget(Forget { entry: october }))?;

    assert_eq!(
        setup.files()?,
        ["2026/2026-10.toml", "2026/2026-11.toml", "device.toml"],
        "the left-over week file held only the forgotten entry"
    );
    let text = event_text(&setup)?;
    assert!(text.contains("entry.forgotten"), "the tombstone stays");
    assert_eq!(text.matches(&format!("entry = \"{october}\"")).count(), 1);
    Ok(())
}

#[test]
fn forgotten_entry_leaves_undo_of_other_command_in_file() -> TestResult {
    let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;
    let tracker = setup.tracker(Split::Month);
    let logged = tracker.run(&Command::Log(Log::new(
        at("2026-11-26T07:00[Europe/Berlin]")?,
        at("2026-11-26T08:00[Europe/Berlin]")?,
    )))?;
    setup.clock.set(at("2026-11-26T09:01[Europe/Berlin]")?);
    _ = tracker.run(&Command::Undo)?;
    setup.clock.set(at("2026-11-26T09:02[Europe/Berlin]")?);
    _ = tracker.run(&Command::Begin(Begin::default()))?;
    let state = tracker.state(&TimeRange::everything())?;
    let running = state.open().next().map(|entry| entry.id).ok_or("running")?;
    setup.clock.set(at("2026-11-26T09:03[Europe/Berlin]")?);

    _ = tracker.run(&Command::Forget(Forget { entry: running }))?;

    let text = event_text(&setup)?;
    let batch = logged.recorded.first().ok_or("the log's event")?.batch;
    assert!(
        text.contains("type = \"batch.undone\""),
        "the undo stays: {text}"
    );
    assert!(text.contains(&format!("undoes = \"{batch}\"")), "{text}");
    assert!(
        tracker
            .state(&TimeRange::everything())?
            .entries()
            .next()
            .is_none(),
        "the logged entry stays undone"
    );
    Ok(())
}

#[test]
fn begin_raced_by_another_process_decides_again_and_switches_from_it() -> TestResult {
    let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;
    _ = setup.tracker(Split::Month).run(&Command::Log(Log::new(
        at("2026-11-26T07:00[Europe/Berlin]")?,
        at("2026-11-26T08:00[Europe/Berlin]")?,
    )))?;
    // The rival read a minute earlier.
    let rival_clock = Arc::new(TestClock::new(at("2026-11-26T08:59[Europe/Berlin]")?));
    let rival = Tracker::new(
        setup.store(Split::Month),
        rival_clock.clone(),
        Arc::new(SequentialIds::starting_at(&rival_clock.now(), 2)),
        Policy::default(),
        setup.device,
    );
    let tracker = Tracker::new(
        Raced::new(
            setup.store(Split::Month),
            rival,
            Command::Begin(Begin {
                note: Some("B".parse()?),
                ..Begin::default()
            }),
            1,
        ),
        setup.clock.clone(),
        Arc::new(SequentialIds::starting_at(&setup.clock.now(), 1)),
        Policy::default(),
        setup.device,
    );

    _ = tracker.run(&Command::Begin(Begin {
        note: Some("A".parse()?),
        ..Begin::default()
    }))?;

    let state = setup
        .tracker(Split::Month)
        .state(&TimeRange::everything())?;
    let open: Vec<_> = state.open().map(|entry| entry.note.clone()).collect();
    assert_eq!(open, vec![Some("A".to_owned())]);
    assert_eq!(state.entries().count(), 3);
    Ok(())
}

/// Tests that cost more than the others: four threads append 40 events to one
/// event file at once, each append under the file lock, and the test reads
/// the file back. A thread runs a command again that the others outran. `.config/nextest.toml` runs them in the test group
/// `expensive`.
mod expensive {
    use super::*;

    #[test]
    fn appends_from_two_processes_both_land() -> TestResult {
        let setup = Setup::new("2026-11-26T09:00[Europe/Berlin]")?;
        let tracker = setup.tracker(Split::Month);
        _ = tracker.run(&Command::Log(Log::new(
            at("2026-11-26T07:00[Europe/Berlin]")?,
            at("2026-11-26T08:00[Europe/Berlin]")?,
        )))?;
        let entry = tracker
            .state(&TimeRange::everything())?
            .entries()
            .next()
            .map(|entry| entry.id)
            .ok_or("entry")?;

        let results = std::thread::scope(|scope| {
            // All four are spawned before the first is joined.
            let mut writers = Vec::new();
            for writer in 0..4u16 {
                let store = setup.store(Split::Month);
                let clock = setup.clock.clone();
                let device = setup.device;
                writers.push(scope.spawn(move || -> pace_error::PaceResult<()> {
                    let tracker = Tracker::new(
                        store,
                        clock.clone(),
                        Arc::new(SequentialIds::starting_at(&clock.now(), 100 + writer)),
                        Policy::default(),
                        device,
                    );
                    for round in 0..10 {
                        let adjust = Command::Adjust(Adjust {
                            entry: Some(entry),
                            note: pace_core::Change::Set(
                                format!("{writer}/{round}").parse().unwrap(),
                            ),
                            ..Adjust::default()
                        });
                        // A command that the others outran is run
                        // again, as its error asks; any other error
                        // ends the writer.
                        loop {
                            match tracker.run(&adjust) {
                                Ok(_) => break,
                                Err(error) if error.is_code("pace::core::concurrent-write") => {}
                                Err(error) => return Err(error),
                            }
                        }
                    }
                    Ok(())
                }));
            }
            writers
                .into_iter()
                .map(|writer| writer.join().map_err(|_| "a writer panicked"))
                .collect::<Result<Vec<_>, _>>()
        })?;

        for (writer, result) in results.into_iter().enumerate() {
            assert!(result.is_ok(), "writer {writer}: {result:?}");
        }
        let read = setup.store(Split::Month).read(&ReadScope::everything())?;
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
}
