//! The state is the result of applying the events in merged order: entries
//! with holds, pauses and catches, and what is running, held or paused.

use std::collections::BTreeSet;

use jiff::{SignedDuration, ZonedDateTime};
use pace_core::{
    BatchId, Change, DeviceId, EntryChanges, EntryId, EntryOrigin, EntryState, Event, EventId,
    EventKind, Hlc, HoldSource, PauseKind, ProjectRef, RecordedEvent, State,
};

use crate::TestResult;

/// The log of one device, with a new batch per event unless told otherwise.
struct Log {
    device: DeviceId,
    number: u32,
    counter: u32,
    events: Vec<RecordedEvent>,
}

fn id(prefix: u16, number: u32) -> String {
    format!("0193a8d0-{prefix:04x}-7000-8000-{number:012x}")
}

fn at(rfc9557: &str) -> Result<ZonedDateTime, jiff::Error> {
    rfc9557.parse()
}

impl Log {
    fn new(device: u32) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            device: id(0xd, device).parse()?,
            number: device,
            counter: 0,
            events: Vec::new(),
        })
    }

    fn push(&mut self, when: &str, kind: EventKind) -> Result<BatchId, Box<dyn std::error::Error>> {
        let batch: BatchId = id(0xb, self.number * 1000 + self.counter).parse()?;
        self.push_in(batch, when, kind)?;
        Ok(batch)
    }

    fn push_in(
        &mut self,
        batch: BatchId,
        when: &str,
        kind: EventKind,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.counter += 1;
        let moment = at(when)?;
        let event_id = id(0xe, self.number * 1000 + self.counter);
        self.events.push(RecordedEvent {
            device: self.device,
            batch,
            hlc: Hlc::new(moment.timestamp(), 0),
            event: Event {
                id: event_id.parse::<EventId>()?,
                at: moment.clone(),
                about: moment,
                kind,
            },
        });
        Ok(())
    }
}

fn entry(number: u32) -> Result<EntryId, Box<dyn std::error::Error>> {
    Ok(id(0xa, number).parse()?)
}

fn begun(entry: EntryId) -> EventKind {
    EventKind::EntryBegun {
        entry,
        project: None,
        note: Some("Write offer".to_owned()),
        tags: BTreeSet::new(),
    }
}

const NOW: &str = "2026-11-26T12:00[Europe/Berlin]";

#[test]
fn begun_entry_runs_and_counts_up_to_now() -> TestResult {
    let mut log = Log::new(1)?;
    let e = entry(1)?;
    _ = log.push("2026-11-26T09:00[Europe/Berlin]", begun(e))?;

    let state = State::from_events(&log.events);
    let found = state.entry(e).ok_or("entry")?;

    assert_eq!(found.state(), EntryState::Running);
    assert_eq!(found.gross(&at(NOW)?), SignedDuration::from_hours(3));
    assert_eq!(found.net(&at(NOW)?), SignedDuration::from_hours(3));
    assert_eq!(found.note.as_deref(), Some("Write offer"));
    assert_eq!(found.origin, EntryOrigin::Live);
    assert_eq!(found.device, log.device);
    Ok(())
}

#[test]
fn hold_interrupts_until_resumed_and_is_not_net_time() -> TestResult {
    let mut log = Log::new(1)?;
    let e = entry(1)?;
    _ = log.push("2026-11-26T09:00[Europe/Berlin]", begun(e))?;
    _ = log.push(
        "2026-11-26T10:00[Europe/Berlin]",
        EventKind::EntryHeld {
            entry: e,
            source: HoldSource::Outer,
            reason: Some("phone call".to_owned()),
        },
    )?;

    let held = State::from_events(&log.events);
    let found = held.entry(e).ok_or("entry")?;
    assert_eq!(found.state(), EntryState::Held);
    assert_eq!(found.net(&at(NOW)?), SignedDuration::from_hours(1));

    _ = log.push(
        "2026-11-26T10:20[Europe/Berlin]",
        EventKind::EntryResumed { entry: e },
    )?;
    let resumed = State::from_events(&log.events);
    let found = resumed.entry(e).ok_or("entry")?;

    assert_eq!(found.state(), EntryState::Running);
    assert_eq!(found.hold_time(&at(NOW)?), SignedDuration::from_mins(20));
    assert_eq!(found.net(&at(NOW)?), SignedDuration::from_mins(160));
    assert_eq!(found.interruptions(), 1);
    assert_eq!(found.holds[0].reason.as_deref(), Some("phone call"));
    Ok(())
}

#[test]
fn pause_is_no_interruption_but_no_net_time_either() -> TestResult {
    let mut log = Log::new(1)?;
    let e = entry(1)?;
    _ = log.push("2026-11-26T09:00[Europe/Berlin]", begun(e))?;
    _ = log.push(
        "2026-11-26T11:00[Europe/Berlin]",
        EventKind::EntryPaused {
            entry: e,
            kind: PauseKind::Lunch,
            reason: None,
        },
    )?;
    _ = log.push(
        "2026-11-26T11:30[Europe/Berlin]",
        EventKind::EntryResumed { entry: e },
    )?;

    let state = State::from_events(&log.events);
    let found = state.entry(e).ok_or("entry")?;

    assert_eq!(found.interruptions(), 0);
    assert_eq!(found.pause_time(&at(NOW)?), SignedDuration::from_mins(30));
    assert_eq!(found.net(&at(NOW)?), SignedDuration::from_mins(150));
    Ok(())
}

#[test]
fn pausing_while_held_turns_the_hold_into_pause() -> TestResult {
    let mut log = Log::new(1)?;
    let e = entry(1)?;
    _ = log.push("2026-11-26T09:00[Europe/Berlin]", begun(e))?;
    _ = log.push(
        "2026-11-26T10:00[Europe/Berlin]",
        EventKind::EntryHeld {
            entry: e,
            source: HoldSource::Unknown,
            reason: None,
        },
    )?;
    _ = log.push(
        "2026-11-26T10:10[Europe/Berlin]",
        EventKind::EntryPaused {
            entry: e,
            kind: PauseKind::Break,
            reason: None,
        },
    )?;

    let state = State::from_events(&log.events);
    let found = state.entry(e).ok_or("entry")?;

    assert_eq!(found.state(), EntryState::Paused);
    assert_eq!(found.hold_time(&at(NOW)?), SignedDuration::from_mins(10));
    assert_eq!(found.pause_time(&at(NOW)?), SignedDuration::from_mins(110));
    Ok(())
}

#[test]
fn second_hold_while_held_only_updates_the_reason() -> TestResult {
    let mut log = Log::new(1)?;
    let e = entry(1)?;
    _ = log.push("2026-11-26T09:00[Europe/Berlin]", begun(e))?;
    for (when, reason) in [("10:00", "Slack"), ("10:05", "phone call")] {
        _ = log.push(
            &format!("2026-11-26T{when}[Europe/Berlin]"),
            EventKind::EntryHeld {
                entry: e,
                source: HoldSource::Outer,
                reason: Some(reason.to_owned()),
            },
        )?;
    }

    let state = State::from_events(&log.events);
    let found = state.entry(e).ok_or("entry")?;

    assert_eq!(found.holds.len(), 1);
    assert_eq!(found.holds[0].reason.as_deref(), Some("phone call"));
    assert_eq!(found.holds[0].start, at("2026-11-26T10:00[Europe/Berlin]")?);
    Ok(())
}

#[test]
fn ending_held_entry_closes_the_hold_first() -> TestResult {
    let mut log = Log::new(1)?;
    let e = entry(1)?;
    _ = log.push("2026-11-26T09:00[Europe/Berlin]", begun(e))?;
    _ = log.push(
        "2026-11-26T10:00[Europe/Berlin]",
        EventKind::EntryHeld {
            entry: e,
            source: HoldSource::Inner,
            reason: None,
        },
    )?;
    _ = log.push(
        "2026-11-26T10:30[Europe/Berlin]",
        EventKind::EntryEnded { entry: e },
    )?;

    let state = State::from_events(&log.events);
    let found = state.entry(e).ok_or("entry")?;

    assert_eq!(found.state(), EntryState::Ended);
    assert_eq!(
        found.holds[0].end,
        Some(at("2026-11-26T10:30[Europe/Berlin]")?)
    );
    assert_eq!(found.gross(&at(NOW)?), SignedDuration::from_mins(90));
    assert_eq!(found.net(&at(NOW)?), SignedDuration::from_mins(60));
    Ok(())
}

#[test]
fn break_outside_its_entry_counts_only_inside_it() -> TestResult {
    let mut log = Log::new(1)?;
    let e = entry(1)?;
    _ = log.push("2026-11-26T09:00[Europe/Berlin]", begun(e))?;
    _ = log.push(
        "2026-11-26T10:00[Europe/Berlin]",
        EventKind::EntryHeld {
            entry: e,
            source: HoldSource::Inner,
            reason: None,
        },
    )?;
    _ = log.push(
        "2026-11-26T10:30[Europe/Berlin]",
        EventKind::EntryResumed { entry: e },
    )?;
    _ = log.push(
        "2026-11-26T11:00[Europe/Berlin]",
        EventKind::EntryEnded { entry: e },
    )?;
    // An adjustment merged from another device, which never saw the hold,
    // moved the start and the end into it.
    _ = log.push(
        "2026-11-26T11:10[Europe/Berlin]",
        EventKind::EntryAdjusted {
            entry: e,
            changes: EntryChanges {
                start: Some(at("2026-11-26T10:10[Europe/Berlin]")?),
                end: Some(at("2026-11-26T10:20[Europe/Berlin]")?),
                ..EntryChanges::default()
            },
        },
    )?;

    let state = State::from_events(&log.events);
    let found = state.entry(e).ok_or("entry")?;

    assert_eq!(found.gross(&at(NOW)?), SignedDuration::from_mins(10));
    assert_eq!(found.hold_time(&at(NOW)?), SignedDuration::from_mins(10));
    assert_eq!(found.net(&at(NOW)?), SignedDuration::ZERO);
    Ok(())
}

#[test]
fn an_entry_keeps_the_zones_it_was_recorded_in() -> TestResult {
    let mut log = Log::new(1)?;
    let e = entry(1)?;
    _ = log.push("2026-11-26T09:00[Europe/Berlin]", begun(e))?;
    _ = log.push(
        "2026-11-26T05:00[America/New_York]",
        EventKind::EntryEnded { entry: e },
    )?;

    let state = State::from_events(&log.events);
    let found = state.entry(e).ok_or("entry")?;

    assert_eq!(
        found.start.to_string(),
        "2026-11-26T09:00:00+01:00[Europe/Berlin]"
    );
    assert_eq!(
        found.end.as_ref().map(ToString::to_string).as_deref(),
        Some("2026-11-26T05:00:00-05:00[America/New_York]")
    );
    assert_eq!(found.gross(&at(NOW)?), SignedDuration::from_hours(2));
    Ok(())
}

#[test]
fn catch_is_noted_without_changing_the_state() -> TestResult {
    let mut log = Log::new(1)?;
    let e = entry(1)?;
    _ = log.push("2026-11-26T09:00[Europe/Berlin]", begun(e))?;
    _ = log.push(
        "2026-11-26T09:30[Europe/Berlin]",
        EventKind::CatchNoted {
            entry: e,
            text: "call the tax office".to_owned(),
        },
    )?;

    let state = State::from_events(&log.events);
    let found = state.entry(e).ok_or("entry")?;

    assert_eq!(found.state(), EntryState::Running);
    assert_eq!(found.catches[0].text, "call the tax office");
    assert_eq!(found.interruptions(), 0);
    Ok(())
}

#[test]
fn logged_entry_is_ended_from_the_start() -> TestResult {
    let mut log = Log::new(1)?;
    let e = entry(1)?;
    _ = log.push(
        "2026-11-25T14:00[Europe/Berlin]",
        EventKind::EntryLogged {
            entry: e,
            end: at("2026-11-25T15:30[Europe/Berlin]")?,
            project: Some(ProjectRef::Key("acme-webshop".parse()?)),
            note: Some("Call with ACME".to_owned()),
            tags: BTreeSet::new(),
        },
    )?;

    let state = State::from_events(&log.events);
    let found = state.entry(e).ok_or("entry")?;

    assert_eq!(found.state(), EntryState::Ended);
    assert_eq!(found.origin, EntryOrigin::Logged);
    assert_eq!(found.net(&at(NOW)?), SignedDuration::from_mins(90));
    assert_eq!(
        found.project,
        Some(ProjectRef::Key("acme-webshop".parse()?))
    );
    Ok(())
}

#[test]
fn adjustments_change_only_the_fields_they_name() -> TestResult {
    let mut log = Log::new(1)?;
    let e = entry(1)?;
    _ = log.push("2026-11-26T09:00[Europe/Berlin]", begun(e))?;
    _ = log.push(
        "2026-11-26T09:10[Europe/Berlin]",
        EventKind::EntryAdjusted {
            entry: e,
            changes: EntryChanges {
                start: Some(at("2026-11-26T08:45[Europe/Berlin]")?),
                tags: Some(BTreeSet::from(["dev".parse()?])),
                ..EntryChanges::default()
            },
        },
    )?;
    _ = log.push(
        "2026-11-26T09:20[Europe/Berlin]",
        EventKind::EntryAdjusted {
            entry: e,
            changes: EntryChanges {
                note: Change::Clear,
                ..EntryChanges::default()
            },
        },
    )?;

    let state = State::from_events(&log.events);
    let found = state.entry(e).ok_or("entry")?;

    assert_eq!(found.start, at("2026-11-26T08:45[Europe/Berlin]")?);
    assert_eq!(found.note, None);
    assert_eq!(found.tags, BTreeSet::from(["dev".parse()?]));
    Ok(())
}

#[test]
fn per_field_the_later_change_wins_across_devices() -> TestResult {
    let mut laptop = Log::new(1)?;
    let mut desktop = Log::new(2)?;
    let e = entry(1)?;
    _ = laptop.push("2026-11-26T09:00[Europe/Berlin]", begun(e))?;
    let change =
        |note: &str, tags: Option<&str>| -> Result<EventKind, Box<dyn std::error::Error>> {
            Ok(EventKind::EntryAdjusted {
                entry: e,
                changes: EntryChanges {
                    note: Change::Set(note.to_owned()),
                    tags: tags
                        .map(|tag| tag.parse().map(|tag| BTreeSet::from([tag])))
                        .transpose()?,
                    ..EntryChanges::default()
                },
            })
        };
    // Recorded apart: the desktop's change comes later by HLC.
    _ = laptop.push(
        "2026-11-26T10:00[Europe/Berlin]",
        change("laptop", Some("dev"))?,
    )?;
    _ = desktop.push("2026-11-26T10:05[Europe/Berlin]", change("desktop", None)?)?;

    let mut merged = desktop.events.clone();
    merged.extend(laptop.events.clone());
    let state = State::from_events(&merged);
    let found = state.entry(e).ok_or("entry")?;

    assert_eq!(found.note.as_deref(), Some("desktop"));
    assert_eq!(found.tags, BTreeSet::from(["dev".parse()?]));
    Ok(())
}

#[test]
fn an_undone_batch_counts_as_never_written() -> TestResult {
    let mut log = Log::new(1)?;
    let e = entry(1)?;
    _ = log.push("2026-11-26T09:00[Europe/Berlin]", begun(e))?;
    let ended = log.push(
        "2026-11-26T10:00[Europe/Berlin]",
        EventKind::EntryEnded { entry: e },
    )?;
    _ = log.push(
        "2026-11-26T10:01[Europe/Berlin]",
        EventKind::BatchUndone { batch: ended },
    )?;

    let state = State::from_events(&log.events);

    assert_eq!(state.entry(e).ok_or("entry")?.state(), EntryState::Running);
    Ok(())
}

#[test]
fn forgotten_entry_is_gone() -> TestResult {
    let mut log = Log::new(1)?;
    let e = entry(1)?;
    _ = log.push("2026-11-26T09:00[Europe/Berlin]", begun(e))?;
    _ = log.push(
        "2026-11-26T10:00[Europe/Berlin]",
        EventKind::EntryForgotten { entry: e },
    )?;

    assert!(State::from_events(&log.events).entry(e).is_none());
    Ok(())
}

#[test]
fn batch_about_forgotten_entry_is_not_left_to_undo() -> TestResult {
    let mut laptop = Log::new(1)?;
    let mut desktop = Log::new(2)?;
    let e = entry(1)?;
    _ = laptop.push("2026-11-26T09:00[Europe/Berlin]", begun(e))?;
    _ = desktop.push(
        "2026-11-26T10:00[Europe/Berlin]",
        EventKind::EntryForgotten { entry: e },
    )?;

    let mut merged = laptop.events;
    merged.extend(desktop.events);
    let state = State::from_events(&merged);

    assert_eq!(state.last_undoable(laptop.device), None);
    Ok(())
}

#[test]
fn batch_about_entry_whose_begin_was_undone_is_not_left_to_undo() -> TestResult {
    let mut laptop = Log::new(1)?;
    let mut desktop = Log::new(2)?;
    let e = entry(1)?;
    let began = laptop.push("2026-11-26T09:00[Europe/Berlin]", begun(e))?;
    _ = desktop.push(
        "2026-11-26T09:30[Europe/Berlin]",
        EventKind::EntryEnded { entry: e },
    )?;
    _ = laptop.push(
        "2026-11-26T10:00[Europe/Berlin]",
        EventKind::BatchUndone { batch: began },
    )?;

    let mut merged = laptop.events;
    merged.extend(desktop.events);
    let state = State::from_events(&merged);

    assert_eq!(state.last_undoable(desktop.device), None);
    Ok(())
}

#[test]
fn batch_about_forgotten_and_kept_entry_is_left_to_undo() -> TestResult {
    let mut laptop = Log::new(1)?;
    let mut desktop = Log::new(2)?;
    let (kept, gone) = (entry(1)?, entry(2)?);
    _ = laptop.push("2026-11-26T09:00[Europe/Berlin]", begun(kept))?;
    let switched = laptop.push(
        "2026-11-26T10:00[Europe/Berlin]",
        EventKind::EntryEnded { entry: kept },
    )?;
    laptop.push_in(switched, "2026-11-26T10:00[Europe/Berlin]", begun(gone))?;
    _ = desktop.push(
        "2026-11-26T11:00[Europe/Berlin]",
        EventKind::EntryForgotten { entry: gone },
    )?;

    let mut merged = laptop.events;
    merged.extend(desktop.events);
    let state = State::from_events(&merged);

    let target = state.last_undoable(laptop.device).ok_or("undoable")?;
    assert_eq!(target.batch, switched);
    Ok(())
}

#[test]
fn events_about_an_unknown_entry_change_nothing() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.push(
        "2026-11-26T10:00[Europe/Berlin]",
        EventKind::EntryEnded { entry: entry(9)? },
    )?;

    assert_eq!(State::from_events(&log.events).entries().count(), 0);
    Ok(())
}

#[test]
fn entries_from_two_devices_can_both_be_open() -> TestResult {
    let mut laptop = Log::new(1)?;
    let mut desktop = Log::new(2)?;
    _ = laptop.push("2026-11-26T09:00[Europe/Berlin]", begun(entry(1)?))?;
    _ = desktop.push("2026-11-26T09:30[Europe/Berlin]", begun(entry(2)?))?;

    let mut merged = laptop.events;
    merged.extend(desktop.events);
    let state = State::from_events(&merged);

    let open: Vec<EntryId> = state.open().map(|entry| entry.id).collect();
    assert_eq!(open, [entry(1)?, entry(2)?]);
    Ok(())
}

fn note_set(entry: EntryId, note: &str) -> EventKind {
    EventKind::EntryAdjusted {
        entry,
        changes: EntryChanges {
            note: Change::Set(note.to_owned()),
            ..EntryChanges::default()
        },
    }
}

#[test]
fn events_with_one_hlc_and_one_device_merge_by_event_id_in_either_input_order() -> TestResult {
    // A copied machine directory writes two events with one HLC and one device.
    let mut log = Log::new(1)?;
    let e = entry(1)?;
    _ = log.push("2026-11-26T09:00[Europe/Berlin]", begun(e))?;
    _ = log.push("2026-11-26T10:00[Europe/Berlin]", note_set(e, "first"))?;
    _ = log.push("2026-11-26T10:00[Europe/Berlin]", note_set(e, "second"))?;
    let mut reversed = log.events.clone();
    reversed.reverse();

    let forward = State::from_events(&log.events);
    let backward = State::from_events(&reversed);

    assert_eq!(forward, backward);
    assert_eq!(
        forward.entry(e).ok_or("entry")?.note.as_deref(),
        Some("second")
    );
    Ok(())
}

#[test]
fn hlc_tie_across_two_devices_goes_to_the_higher_device_id() -> TestResult {
    let mut laptop = Log::new(1)?;
    let mut desktop = Log::new(2)?;
    let e = entry(1)?;
    _ = laptop.push("2026-11-26T09:00[Europe/Berlin]", begun(e))?;
    _ = desktop.push("2026-11-26T10:00[Europe/Berlin]", note_set(e, "desktop"))?;
    _ = laptop.push("2026-11-26T10:00[Europe/Berlin]", note_set(e, "laptop"))?;

    for merged in [
        [desktop.events.clone(), laptop.events.clone()].concat(),
        [laptop.events.clone(), desktop.events.clone()].concat(),
    ] {
        let state = State::from_events(&merged);
        assert_eq!(
            state.entry(e).ok_or("entry")?.note.as_deref(),
            Some("desktop")
        );
    }
    Ok(())
}

// short IDs

#[test]
fn short_id_is_last_four_characters_or_more_when_needed() -> TestResult {
    let ids: Vec<EntryId> = [
        "0193a8d0-6c1e-7f3a-9b2d-4e5f60719a2c",
        "0193a8d0-6c1e-7f3a-9b2d-4e5f60729a2c",
        "0193a8d0-6c1e-7f3a-9b2d-4e5f607100ff",
    ]
    .iter()
    .map(|id| id.parse())
    .collect::<Result<_, _>>()?;
    let mut log = Log::new(1)?;
    for id in &ids {
        _ = log.push("2026-11-26T09:00[Europe/Berlin]", begun(*id))?;
    }

    let state = State::from_events(&log.events);

    assert_eq!(state.short_id(ids[0]), "19a2c");
    assert_eq!(state.short_id(ids[1]), "29a2c");
    assert_eq!(state.short_id(ids[2]), "00ff");
    Ok(())
}

#[test]
fn entry_outside_state_shows_its_whole_id() -> TestResult {
    let mut log = Log::new(1)?;
    _ = log.push("2026-11-26T09:00[Europe/Berlin]", begun(entry(1)?))?;
    let unknown: EntryId = "deadbeef-0000-7000-8000-000000000000".parse()?;

    let state = State::from_events(&log.events);

    assert_eq!(state.short_id(unknown), unknown.to_string());
    Ok(())
}
