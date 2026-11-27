use std::collections::{BTreeMap, BTreeSet};

use jiff::ZonedDateTime;
use pace_core::{
    BatchId, DeviceId, EntryId, Event, EventId, EventKind, Hlc, ReadScope, RecordedEvent,
};
use pace_time::TimeRange;

use crate::index::{FileSummary, INDEX_VERSION, Index, select, summarize};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn at(time: &str) -> Result<ZonedDateTime, jiff::Error> {
    format!("2026-{time}[Europe/Berlin]").parse()
}

fn id<T: std::str::FromStr>(prefix: u16, number: u32) -> Result<T, T::Err> {
    format!("0193a8d0-{prefix:04x}-7000-8000-{number:012x}").parse()
}

struct Events {
    events: Vec<RecordedEvent>,
    counter: u32,
}

impl Events {
    const fn new() -> Self {
        Self {
            events: Vec::new(),
            counter: 0,
        }
    }

    /// Add an event at `when`, filed under `about`, in batch `batch`.
    fn add(
        &mut self,
        batch: u32,
        when: &str,
        about: &str,
        kind: EventKind,
    ) -> Result<&mut Self, Box<dyn std::error::Error>> {
        self.counter += 1;
        let moment = at(when)?;
        self.events.push(RecordedEvent {
            device: id::<DeviceId>(0xd, 1)?,
            batch: id::<BatchId>(0xb, batch)?,
            hlc: Hlc::new(moment.timestamp(), self.counter),
            event: Event {
                id: id::<EventId>(0xe, self.counter)?,
                at: moment,
                about: at(about)?,
                kind,
            },
        });
        Ok(self)
    }
}

fn entry(number: u32) -> Result<EntryId, Box<dyn std::error::Error>> {
    Ok(id(0xa, number)?)
}

fn begun(entry: EntryId) -> EventKind {
    EventKind::EntryBegun {
        entry,
        project: None,
        note: None,
        tags: BTreeSet::new(),
    }
}

#[test]
fn begun_entry_without_an_end_is_unended() -> TestResult {
    let mut events = Events::new();
    _ = events.add(1, "11-26T09:00", "11-26T09:00", begun(entry(1)?))?;

    let summary = summarize(&events.events);

    assert_eq!(summary.unended, [entry(1)?]);
    assert_eq!(summary.ends_elsewhere, Vec::<EntryId>::new());
    Ok(())
}

#[test]
fn an_ended_entry_is_not_unended_unless_the_end_was_undone() -> TestResult {
    let mut events = Events::new();
    _ = events
        .add(1, "11-26T09:00", "11-26T09:00", begun(entry(1)?))?
        .add(
            2,
            "11-26T10:00",
            "11-26T09:00",
            EventKind::EntryEnded { entry: entry(1)? },
        )?;
    assert_eq!(summarize(&events.events).unended, Vec::new());

    _ = events.add(
        3,
        "11-26T10:01",
        "11-26T09:00",
        EventKind::BatchUndone { batch: id(0xb, 2)? },
    )?;

    assert_eq!(summarize(&events.events).unended.len(), 1);
    Ok(())
}

#[test]
fn an_end_of_an_entry_begun_elsewhere_is_listed() -> TestResult {
    let mut events = Events::new();
    _ = events.add(
        1,
        "11-26T10:00",
        "11-26T09:00",
        EventKind::EntryEnded { entry: entry(1)? },
    )?;

    assert_eq!(summarize(&events.events).ends_elsewhere, [entry(1)?]);
    Ok(())
}

#[test]
fn tombstone_keeps_the_moment_its_entry_is_filed_under() -> TestResult {
    let mut events = Events::new();
    _ = events.add(
        1,
        "11-27T08:00",
        "11-26T09:00",
        EventKind::EntryForgotten { entry: entry(1)? },
    )?;

    let summary = summarize(&events.events);

    assert_eq!(
        summary.forgotten,
        [(entry(1)?, at("11-26T09:00")?.timestamp().as_millisecond())]
    );
    Ok(())
}

#[test]
fn an_index_from_before_tombstones_is_not_read() {
    // Version 1 of the index had no `forgotten`; reading it as empty would
    // miss the tombstones of its files.
    let old = r#"{"version":1,"files":{"d/2026/2026-11.toml":{"len":1,
        "modified":null,"max_hlc":null,"envelope":null,"abouts":null,
        "unended":[],"ends_elsewhere":[]}}}"#;

    assert!(serde_json::from_str::<Index>(old).is_err());
    assert_ne!(INDEX_VERSION, 1);
}

#[test]
fn the_envelope_covers_every_moment_in_the_file() -> TestResult {
    let mut events = Events::new();
    _ = events
        .add(1, "11-26T09:00", "11-26T09:00", begun(entry(1)?))?
        .add(
            2,
            "11-27T08:00",
            "11-26T09:00",
            EventKind::EntryAdjusted {
                entry: entry(1)?,
                changes: pace_core::EntryChanges {
                    start: Some(at("11-25T23:00")?),
                    ..pace_core::EntryChanges::default()
                },
            },
        )?;

    let summary = summarize(&events.events);

    assert_eq!(
        summary.envelope,
        Some((
            at("11-25T23:00")?.timestamp().as_millisecond(),
            at("11-27T08:00")?.timestamp().as_millisecond()
        ))
    );
    assert_eq!(
        summary.abouts,
        Some((
            at("11-26T09:00")?.timestamp().as_millisecond(),
            at("11-26T09:00")?.timestamp().as_millisecond()
        ))
    );
    Ok(())
}

fn range(start: &str, end: &str) -> Result<ReadScope, Box<dyn std::error::Error>> {
    Ok(ReadScope::of(TimeRange::new(
        Some(at(start)?),
        Some(at(end)?),
    )?))
}

fn files(list: Vec<(&str, FileSummary)>) -> BTreeMap<String, FileSummary> {
    list.into_iter()
        .map(|(name, summary)| (name.to_owned(), summary))
        .collect()
}

fn summary_of(events: &Events) -> FileSummary {
    summarize(&events.events)
}

#[test]
fn files_whose_events_meet_the_range_are_selected() -> TestResult {
    let mut october = Events::new();
    _ = october
        .add(1, "10-15T09:00", "10-15T09:00", begun(entry(1)?))?
        .add(
            2,
            "10-15T10:00",
            "10-15T09:00",
            EventKind::EntryEnded { entry: entry(1)? },
        )?;
    let mut november = Events::new();
    _ = november
        .add(3, "11-26T09:00", "11-26T09:00", begun(entry(2)?))?
        .add(
            4,
            "11-26T10:00",
            "11-26T09:00",
            EventKind::EntryEnded { entry: entry(2)? },
        )?;
    let all = files(vec![
        ("a/2026/2026-10.toml", summary_of(&october)),
        ("a/2026/2026-11.toml", summary_of(&november)),
    ]);

    let selected = select(&all, &range("11-26T00:00", "11-27T00:00")?);

    assert_eq!(selected, BTreeSet::from(["a/2026/2026-11.toml".to_owned()]));
    Ok(())
}

#[test]
fn the_file_of_an_open_entry_is_selected_whatever_the_range() -> TestResult {
    let mut october = Events::new();
    _ = october.add(1, "10-15T09:00", "10-15T09:00", begun(entry(1)?))?;
    let all = files(vec![("a/2026/2026-10.toml", summary_of(&october))]);

    let selected = select(&all, &range("11-26T00:00", "11-27T00:00")?);

    assert_eq!(selected, BTreeSet::from(["a/2026/2026-10.toml".to_owned()]));
    Ok(())
}

#[test]
fn an_entry_ended_by_another_device_is_not_open() -> TestResult {
    let mut laptop = Events::new();
    _ = laptop.add(1, "10-15T09:00", "10-15T09:00", begun(entry(1)?))?;
    let mut desktop = Events::new();
    _ = desktop.add(
        2,
        "10-15T10:00",
        "10-15T09:00",
        EventKind::EntryEnded { entry: entry(1)? },
    )?;
    let all = files(vec![
        ("a/2026/2026-10.toml", summary_of(&laptop)),
        ("b/2026/2026-W42.toml", summary_of(&desktop)),
    ]);

    let selected = select(&all, &range("11-26T00:00", "11-27T00:00")?);

    assert_eq!(selected, BTreeSet::new());
    Ok(())
}

#[test]
fn files_of_other_devices_about_the_same_entries_come_along() -> TestResult {
    let mut laptop = Events::new();
    _ = laptop.add(1, "10-15T09:00", "10-15T09:00", begun(entry(1)?))?;
    // The desktop held the laptop's open entry, in its weekly file.
    let mut desktop = Events::new();
    _ = desktop.add(
        2,
        "11-26T10:00",
        "10-15T09:00",
        EventKind::EntryHeld {
            entry: entry(1)?,
            source: pace_core::HoldSource::Outer,
            reason: None,
        },
    )?;
    let mut unrelated = Events::new();
    _ = unrelated
        .add(3, "09-01T09:00", "09-01T09:00", begun(entry(3)?))?
        .add(
            4,
            "09-01T10:00",
            "09-01T09:00",
            EventKind::EntryEnded { entry: entry(3)? },
        )?;
    let all = files(vec![
        ("a/2026/2026-10.toml", summary_of(&laptop)),
        ("b/2026/2026-W42.toml", summary_of(&desktop)),
        ("b/2026/2026-W36.toml", summary_of(&unrelated)),
    ]);

    let selected = select(&all, &range("10-15T09:30", "10-15T09:40")?);

    assert_eq!(
        selected,
        BTreeSet::from([
            "a/2026/2026-10.toml".to_owned(),
            "b/2026/2026-W42.toml".to_owned()
        ])
    );
    Ok(())
}

#[test]
fn files_with_events_recorded_since_are_selected_with_their_entries() -> TestResult {
    // Logged on 26 November about June: the moments lie in June, the HLC
    // in November.
    let mut june = Events::new();
    _ = june.add(
        1,
        "06-30T08:00",
        "06-30T08:00",
        EventKind::EntryLogged {
            entry: entry(1)?,
            end: at("06-30T09:00")?,
            project: None,
            note: None,
            tags: BTreeSet::new(),
        },
    )?;
    for recorded in &mut june.events {
        recorded.hlc = Hlc::new(at("11-26T10:00")?.timestamp(), 0);
    }
    let mut september = Events::new();
    _ = september
        .add(2, "09-01T09:00", "09-01T09:00", begun(entry(2)?))?
        .add(
            3,
            "09-01T10:00",
            "09-01T09:00",
            EventKind::EntryEnded { entry: entry(2)? },
        )?;
    let all = files(vec![
        ("a/2026/2026-06.toml", summary_of(&june)),
        ("a/2026/2026-09.toml", summary_of(&september)),
    ]);
    let scope = range("11-26T11:00", "11-26T11:00")?;

    let without = select(&all, &scope);
    let with = select(
        &all,
        &scope.with_recorded_since(at("11-25T00:00")?.timestamp()),
    );

    assert_eq!(without, BTreeSet::new());
    assert_eq!(with, BTreeSet::from(["a/2026/2026-06.toml".to_owned()]));
    Ok(())
}
