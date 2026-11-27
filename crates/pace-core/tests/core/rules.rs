//! What each command turns into: the events of one batch, or an error that
//! says how to fix it. Scenarios marked "old pace" come from the 19
//! state-machine tests of the old in-memory backend.

use std::collections::BTreeSet;

use jiff::ZonedDateTime;
use pace_core::{
    Batch, BatchId, Catalog, Change, Command, DeterminedBy, DeviceId, EntryId, EntryState,
    EntryVerb, Event, EventId, EventKind, Hlc, HoldSource, Ids, OVERLAP, PauseKind, Policy,
    ProjectRecord, ProjectRef, RecordedEvent, Role, RoleId, RoleKind, Rules, State,
    command::{Adjust, Begin, Catch, End, Forget, Hold, Log, Pause, Resume},
    testing::SequentialIds,
};
use pace_error::{ExitCode, PaceError};
use rstest::rstest;

use crate::TestResult;

/// A device that decides commands against its state and appends the
/// batches to an in-memory log.
struct World {
    device: DeviceId,
    ids: SequentialIds,
    policy: Policy,
    catalog: Catalog,
    log: Vec<RecordedEvent>,
    counter: u32,
}

fn at(rfc9557: &str) -> Result<ZonedDateTime, jiff::Error> {
    rfc9557.parse()
}

fn berlin(time: &str) -> Result<ZonedDateTime, jiff::Error> {
    at(&format!("2026-11-26T{time}[Europe/Berlin]"))
}

impl World {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::on_device(1)
    }

    fn on_device(number: u16) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            device: format!("0193a8d0-000d-7000-8000-{number:012x}").parse()?,
            ids: SequentialIds::starting_at(&berlin("08:00")?, number),
            policy: Policy::default(),
            catalog: Catalog::default(),
            log: Vec::new(),
            counter: 0,
        })
    }

    fn state(&self) -> State {
        State::from_events(&self.log)
    }

    /// Decide `command` at `now` (Berlin time) and append its batch.
    /// `now` is `HH:MM` on 2026-11-26 in Berlin, or a full RFC 9557 time.
    fn run(&mut self, now: &str, command: &Command) -> Result<Batch, Box<PaceError>> {
        let now = if now.contains('[') {
            at(now)
        } else {
            berlin(now)
        }
        .map_err(|error| {
            PaceError::with_source(
                pace_error::ErrorKind::Internal,
                "the test time {time} is no time",
                error,
            )
            .attach_context("time", now)
        })?;
        let state = self.state();
        let batch = Rules {
            state: &state,
            now: &now,
            device: self.device,
            ids: &self.ids,
            policy: &self.policy,
            catalog: &self.catalog,
        }
        .decide(command)?;
        for event in &batch.events {
            self.counter += 1;
            self.log.push(RecordedEvent {
                device: self.device,
                batch: batch.id,
                hlc: Hlc::new(now.timestamp(), self.counter),
                event: event.clone(),
            });
        }
        Ok(batch)
    }

    /// Append `kind`, which happened at `time`, as a batch of its own
    /// recorded at `now` (both Berlin), without the rules: an entry as an
    /// older pace or a merge of two devices left it.
    fn record(&mut self, now: &str, time: &str, kind: EventKind) -> TestResult {
        let at = berlin(time)?;
        self.counter += 1;
        self.log.push(RecordedEvent {
            device: self.device,
            batch: BatchId::try_from(self.ids.next_id())?,
            hlc: Hlc::new(berlin(now)?.timestamp(), self.counter),
            event: Event {
                id: EventId::try_from(self.ids.next_id())?,
                at: at.clone(),
                about: at,
                kind,
            },
        });
        Ok(())
    }

    fn only_entry(&self, state: EntryState) -> Result<EntryId, Box<dyn std::error::Error>> {
        let found = self.state();
        let entries: Vec<_> = found.entries().filter(|e| e.state() == state).collect();
        match entries.as_slice() {
            [entry] => Ok(entry.id),
            other => Err(format!("expected one {state:?} entry, found {}", other.len()).into()),
        }
    }
}

fn begin(note: &str) -> Command {
    Command::Begin(Begin {
        note: note.parse().ok(),
        ..Begin::default()
    })
}

fn begin_at(note: &str, time: &str) -> Result<Command, Box<dyn std::error::Error>> {
    Ok(Command::Begin(Begin {
        at: Some(berlin(time)?),
        note: Some(note.parse()?),
        ..Begin::default()
    }))
}

fn kinds(batch: &Batch) -> Vec<&'static str> {
    batch
        .events
        .iter()
        .map(|event| match event.kind {
            EventKind::EntryBegun { .. } => "entry.begun",
            EventKind::EntryHeld { .. } => "entry.held",
            EventKind::EntryPaused { .. } => "entry.paused",
            EventKind::EntryResumed { .. } => "entry.resumed",
            EventKind::EntryEnded { .. } => "entry.ended",
            EventKind::CatchNoted { .. } => "catch.noted",
            EventKind::EntryLogged { .. } => "entry.logged",
            EventKind::EntryAdjusted { .. } => "entry.adjusted",
            EventKind::EntryForgotten { .. } => "entry.forgotten",
            EventKind::BatchUndone { .. } => "batch.undone",
        })
        .collect()
}

fn failure(
    result: Result<Batch, Box<PaceError>>,
) -> Result<Box<PaceError>, Box<dyn std::error::Error>> {
    result.err().ok_or_else(|| "the command must fail".into())
}

// begin

#[test]
fn begin_starts_running_entry_now() -> TestResult {
    let mut world = World::new()?;

    let batch = world.run("09:00", &begin("Write offer"))?;

    assert_eq!(kinds(&batch), ["entry.begun"]);
    let state = world.state();
    let entry = state
        .entry(world.only_entry(EntryState::Running)?)
        .ok_or("entry")?;
    assert_eq!(entry.start, berlin("09:00")?);
    assert_eq!(entry.note.as_deref(), Some("Write offer"));
    Ok(())
}

// old pace: begin_and_auto_end_for_multiple_activities; the old pace ended
// the running entry at "now", not at the new entry's start.
#[test]
fn begin_while_running_ends_the_running_entry_at_the_new_start() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    let first = world.only_entry(EntryState::Running)?;

    let batch = world.run("10:30", &begin_at("Call ACME", "10:15")?)?;

    assert_eq!(kinds(&batch), ["entry.ended", "entry.begun"]);
    let state = world.state();
    assert_eq!(
        state.entry(first).ok_or("first")?.end,
        Some(berlin("10:15")?)
    );
    assert_eq!(state.open().count(), 1);
    Ok(())
}

// old pace: begin_activity_with_held_activity
#[test]
fn begin_while_held_ends_the_held_entry_and_its_hold() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    let first = world.only_entry(EntryState::Running)?;
    _ = world.run("10:00", &Command::Hold(Hold::default()))?;

    _ = world.run("10:30", &begin("Call ACME"))?;

    let state = world.state();
    let ended = state.entry(first).ok_or("first")?;
    assert_eq!(ended.state(), EntryState::Ended);
    assert_eq!(ended.holds[0].end, Some(berlin("10:30")?));
    Ok(())
}

#[test]
fn begin_before_the_running_entry_began_is_refused() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("10:00", &begin("Write offer"))?;

    let error = failure(world.run("10:30", &begin_at("Call", "09:30")?))?;

    assert_eq!(
        error.to_string(),
        "the running entry began at 10:00, after the new entry's start at 09:30"
    );
    assert_eq!(
        error.hint(),
        Some("begin after 10:00, or move the running entry with `pace adjust --start`")
    );
    Ok(())
}

#[test]
fn begin_at_running_entrys_own_start_is_refused_as_same_moment() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("10:00", &begin("Write offer"))?;

    let error = failure(world.run("10:00", &begin("Call")))?;

    assert_eq!(
        error.to_string(),
        "the running entry began at 10:00, the same moment as the new entry"
    );
    assert_eq!(
        error.context_value("start"),
        None,
        "no detail repeats the time"
    );
    assert_eq!(
        error.hint(),
        Some("begin after 10:00, or move the running entry with `pace adjust --start`")
    );
    Ok(())
}

#[test]
fn begin_inside_closed_break_of_running_entry_is_refused() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Pause(Pause::default()))?;
    _ = world.run("10:30", &Command::Resume(Resume::default()))?;

    let error = failure(world.run("10:40", &begin_at("Call ACME", "10:15")?))?;

    assert_eq!(
        error.to_string(),
        "the running entry has a break until 10:30, after the new entry's start at 10:15"
    );
    assert_eq!(error.hint(), Some("begin at 10:30 or later with `--at`"));
    Ok(())
}

#[test]
fn begin_before_open_break_of_running_entry_is_refused() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Hold(Hold::default()))?;

    let error = failure(world.run("10:10", &begin_at("Call ACME", "09:30")?))?;

    assert_eq!(error.to_string(), "the break began at 10:00, after 09:30");
    Ok(())
}

#[test]
fn begin_into_an_ended_entry_is_an_overlap() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::End(End::default()))?;

    let error = failure(world.run("10:30", &begin_at("Call", "09:30")?))?;

    assert!(
        error.to_string().starts_with("the new entry would overlap"),
        "{error}"
    );
    assert_eq!(
        error.hint(),
        Some("begin at 10:00 or later, or allow it with `--allow-overlap`")
    );
    Ok(())
}

#[test]
fn an_overlap_can_be_allowed() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::End(End::default()))?;

    let batch = world.run(
        "10:30",
        &Command::Begin(Begin {
            at: Some(berlin("09:30")?),
            allow_overlap: true,
            ..Begin::default()
        }),
    )?;

    assert_eq!(kinds(&batch), ["entry.begun"]);
    Ok(())
}

#[test]
fn begin_with_two_open_entries_asks_to_end_one() -> TestResult {
    let mut laptop = World::new()?;
    let mut desktop = World::on_device(2)?;
    _ = laptop.run("09:00", &begin("on the laptop"))?;
    _ = desktop.run("09:30", &begin("on the desktop"))?;
    laptop.log.extend(desktop.log);

    let error = failure(laptop.run("10:00", &begin("Call")))?;

    assert!(
        error.to_string().starts_with("several entries are open: "),
        "{error}"
    );
    assert_eq!(error.hint(), Some("end one of them with `pace end <id>`"));
    Ok(())
}

#[test]
fn begin_records_the_project_and_tags() -> TestResult {
    let mut world = World::new()?;

    _ = world.run(
        "09:00",
        &Command::Begin(Begin {
            project: Some(ProjectRef::Key("acme-webshop".parse()?)),
            tags: BTreeSet::from(["dev".parse()?]),
            ..Begin::default()
        }),
    )?;

    let state = world.state();
    let entry = state
        .entry(world.only_entry(EntryState::Running)?)
        .ok_or("entry")?;
    assert_eq!(
        entry.project,
        Some(ProjectRef::Key("acme-webshop".parse()?))
    );
    assert_eq!(entry.tags, BTreeSet::from(["dev".parse()?]));
    Ok(())
}

// hold, pause, resume

// old pace: hold_most_recent_active_activity
#[test]
fn hold_interrupts_the_running_entry() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;

    let batch = world.run(
        "10:00",
        &Command::Hold(Hold {
            source: Some(HoldSource::Outer),
            reason: Some("phone call".parse()?),
            ..Hold::default()
        }),
    )?;

    assert_eq!(kinds(&batch), ["entry.held"]);
    _ = world.only_entry(EntryState::Held)?;
    Ok(())
}

#[test]
fn hold_without_source_takes_the_setting() -> TestResult {
    let mut world = World::new()?;
    world.policy.default_hold_source = HoldSource::Inner;
    _ = world.run("09:00", &begin("Write offer"))?;

    _ = world.run("10:00", &Command::Hold(Hold::default()))?;

    let state = world.state();
    let entry = state
        .entry(world.only_entry(EntryState::Held)?)
        .ok_or("entry")?;
    assert_eq!(entry.holds[0].source, HoldSource::Inner);
    Ok(())
}

#[test]
fn hold_while_nothing_runs_has_nothing_to_do() -> TestResult {
    let mut world = World::new()?;

    let error = failure(world.run("10:00", &Command::Hold(Hold::default())))?;

    assert_eq!(error.to_string(), "nothing is running");
    assert_eq!(error.hint(), Some("`pace begin` starts an entry"));
    assert_eq!(error.exit_code(), ExitCode::NothingToDo);
    Ok(())
}

// old pace: hold_last_unfinished_activity_with_existing_intermission_does_nothing
#[test]
fn second_hold_without_reason_has_nothing_to_do() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Hold(Hold::default()))?;

    let error = failure(world.run("10:05", &Command::Hold(Hold::default())))?;

    assert_eq!(error.to_string(), "the entry is held since 10:00");
    assert_eq!(error.exit_code(), ExitCode::NothingToDo);
    Ok(())
}

#[test]
fn second_hold_with_reason_updates_it() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Hold(Hold::default()))?;

    _ = world.run(
        "10:05",
        &Command::Hold(Hold {
            reason: Some("Slack".parse()?),
            ..Hold::default()
        }),
    )?;

    let state = world.state();
    let entry = state
        .entry(world.only_entry(EntryState::Held)?)
        .ok_or("entry")?;
    assert_eq!(entry.holds.len(), 1);
    assert_eq!(entry.holds[0].reason.as_deref(), Some("Slack"));
    Ok(())
}

#[test]
fn second_pause_without_reason_has_nothing_to_do() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("12:00", &Command::Pause(Pause::default()))?;

    let error = failure(world.run("12:05", &Command::Pause(Pause::default())))?;

    assert_eq!(error.to_string(), "the entry is paused since 12:00");
    assert_eq!(error.hint(), Some("`pace resume` goes on with it"));
    assert_eq!(error.exit_code(), ExitCode::NothingToDo);
    Ok(())
}

#[test]
fn second_pause_with_reason_updates_it() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("12:00", &Command::Pause(Pause::default()))?;

    _ = world.run(
        "12:05",
        &Command::Pause(Pause {
            reason: Some("lunch".parse()?),
            ..Pause::default()
        }),
    )?;

    let state = world.state();
    let entry = state
        .entry(world.only_entry(EntryState::Paused)?)
        .ok_or("entry")?;
    assert_eq!(entry.pauses.len(), 1);
    assert_eq!(entry.pauses[0].reason.as_deref(), Some("lunch"));
    Ok(())
}

#[test]
fn hold_before_the_entry_began_is_refused() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;

    let error = failure(world.run(
        "10:00",
        &Command::Hold(Hold {
            at: Some(berlin("08:30")?),
            ..Hold::default()
        }),
    ))?;

    assert_eq!(error.to_string(), "the entry began at 09:00, after 08:30");
    Ok(())
}

#[test]
fn hold_in_the_future_is_refused() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;

    let error = failure(world.run(
        "10:00",
        &Command::Hold(Hold {
            at: Some(berlin("23:00")?),
            ..Hold::default()
        }),
    ))?;

    assert_eq!(
        error.to_string(),
        "the entry would be held at 23:00, in the future"
    );
    assert_eq!(error.hint(), Some("leave out `--at` to hold it now"));
    Ok(())
}

#[test]
fn pause_in_the_future_is_refused() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;

    let error = failure(world.run(
        "10:00",
        &Command::Pause(Pause {
            at: Some(berlin("23:00")?),
            ..Pause::default()
        }),
    ))?;

    assert_eq!(
        error.to_string(),
        "the entry would be paused at 23:00, in the future"
    );
    assert_eq!(error.hint(), Some("leave out `--at` to pause it now"));
    Ok(())
}

#[test]
fn resume_in_the_future_is_refused() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Hold(Hold::default()))?;

    let error = failure(world.run(
        "10:20",
        &Command::Resume(Resume {
            at: Some(berlin("23:00")?),
            ..Resume::default()
        }),
    ))?;

    assert_eq!(
        error.to_string(),
        "the entry would resume at 23:00, in the future"
    );
    assert_eq!(error.hint(), Some("leave out `--at` to resume it now"));
    Ok(())
}

#[test]
fn hold_before_the_open_pause_began_is_refused() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Pause(Pause::default()))?;

    let error = failure(world.run(
        "10:06",
        &Command::Hold(Hold {
            at: Some(berlin("09:30")?),
            ..Hold::default()
        }),
    ))?;

    assert_eq!(error.to_string(), "the break began at 10:00, after 09:30");
    assert_eq!(error.hint(), Some("give a time after 10:00"));
    Ok(())
}

#[test]
fn pause_before_the_open_hold_began_is_refused() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Hold(Hold::default()))?;

    let error = failure(world.run(
        "10:06",
        &Command::Pause(Pause {
            at: Some(berlin("09:30")?),
            ..Pause::default()
        }),
    ))?;

    assert_eq!(error.to_string(), "the break began at 10:00, after 09:30");
    Ok(())
}

#[test]
fn hold_before_the_last_break_ended_is_refused() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Pause(Pause::default()))?;
    _ = world.run("10:30", &Command::Resume(Resume::default()))?;

    let error = failure(world.run(
        "10:40",
        &Command::Hold(Hold {
            at: Some(berlin("10:15")?),
            ..Hold::default()
        }),
    ))?;

    assert_eq!(
        error.to_string(),
        "the last break ended at 10:30, after 10:15"
    );
    Ok(())
}

#[test]
fn pause_while_held_turns_the_hold_into_pause() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Hold(Hold::default()))?;

    let batch = world.run(
        "10:10",
        &Command::Pause(Pause {
            kind: PauseKind::Lunch,
            ..Pause::default()
        }),
    )?;

    assert_eq!(kinds(&batch), ["entry.paused"]);
    _ = world.only_entry(EntryState::Paused)?;
    Ok(())
}

#[test]
fn resume_ends_the_hold() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Hold(Hold::default()))?;

    let batch = world.run("10:20", &Command::Resume(Resume::default()))?;

    assert_eq!(kinds(&batch), ["entry.resumed"]);
    _ = world.only_entry(EntryState::Running)?;
    Ok(())
}

#[test]
fn resume_while_running_has_nothing_to_do() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;

    let error = failure(world.run("10:00", &Command::Resume(Resume::default())))?;

    assert_eq!(
        error.to_string(),
        "the entry is running, not held or paused"
    );
    assert_eq!(error.exit_code(), ExitCode::NothingToDo);
    Ok(())
}

#[test]
fn resume_before_hold_began_is_refused() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Hold(Hold::default()))?;

    let error = failure(world.run(
        "10:20",
        &Command::Resume(Resume {
            at: Some(berlin("09:50")?),
            ..Resume::default()
        }),
    ))?;

    assert_eq!(error.to_string(), "the break began at 10:00, after 09:50");
    assert_eq!(error.hint(), Some("give a time after 10:00"));
    Ok(())
}

#[test]
fn resume_names_entry_when_two_are_open() -> TestResult {
    let mut world = two_open()?;

    let error = failure(world.run("10:00", &Command::Resume(Resume::default())))?;

    assert_eq!(error.hint(), Some("name one with `pace resume <id>`"));
    Ok(())
}

// catch

#[test]
fn catch_notes_thought_on_the_running_entry() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;

    let batch = world.run(
        "09:30",
        &Command::Catch(Catch {
            text: "call the tax office".parse()?,
        }),
    )?;

    assert_eq!(kinds(&batch), ["catch.noted"]);
    _ = world.only_entry(EntryState::Running)?;
    Ok(())
}

#[test]
fn catch_while_nothing_runs_is_refused() -> TestResult {
    let mut world = World::new()?;

    let error = failure(world.run(
        "09:30",
        &Command::Catch(Catch {
            text: "call the tax office".parse()?,
        }),
    ))?;

    assert_eq!(
        error.to_string(),
        "nothing is running to note the thought on"
    );
    assert_eq!(error.exit_code(), ExitCode::Error);
    Ok(())
}

// end

// old pace: end_single_activity, end_last_unfinished_activity
#[test]
fn end_ends_the_open_entry() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;

    let batch = world.run("10:00", &Command::End(End::default()))?;

    assert_eq!(kinds(&batch), ["entry.ended"]);
    _ = world.only_entry(EntryState::Ended)?;
    Ok(())
}

// old pace: end_all_active_intermissions
#[test]
fn end_closes_an_open_pause() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("12:00", &Command::Pause(Pause::default()))?;

    _ = world.run("12:30", &Command::End(End::default()))?;

    let state = world.state();
    let entry = state
        .entry(world.only_entry(EntryState::Ended)?)
        .ok_or("entry")?;
    assert_eq!(entry.pauses[0].end, Some(berlin("12:30")?));
    Ok(())
}

#[test]
fn end_while_nothing_runs_has_nothing_to_do() -> TestResult {
    let mut world = World::new()?;

    let error = failure(world.run("10:00", &Command::End(End::default())))?;

    assert_eq!(error.to_string(), "nothing is running");
    assert_eq!(error.exit_code(), ExitCode::NothingToDo);
    Ok(())
}

#[test]
fn adjust_while_nothing_runs_names_id_and_last() -> TestResult {
    let mut world = World::new()?;

    let error = failure(world.run(
        "10:00",
        &Command::Adjust(Adjust {
            note: Change::Set("Write offer".parse()?),
            ..Adjust::default()
        }),
    ))?;

    assert_eq!(error.to_string(), "nothing is running");
    assert_eq!(
        error.hint(),
        Some(
            "name the entry with `pace adjust <id>`, or take the last one with `pace adjust --last`"
        )
    );
    assert_eq!(error.exit_code(), ExitCode::NothingToDo);
    Ok(())
}

#[test]
fn end_before_the_entry_began_is_refused() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;

    let error = failure(world.run(
        "10:00",
        &Command::End(End {
            at: Some(berlin("08:00")?),
            ..End::default()
        }),
    ))?;

    assert_eq!(error.to_string(), "the entry began at 09:00, after 08:00");
    Ok(())
}

#[test]
fn end_inside_closed_hold_is_refused() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Hold(Hold::default()))?;
    _ = world.run("10:30", &Command::Resume(Resume::default()))?;

    let error = failure(world.run(
        "10:40",
        &Command::End(End {
            at: Some(berlin("10:15")?),
            ..End::default()
        }),
    ))?;

    assert_eq!(
        error.to_string(),
        "the entry has a break until 10:30, after the new end at 10:15"
    );
    Ok(())
}

#[test]
fn end_before_the_open_break_began_is_refused() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Hold(Hold::default()))?;

    let error = failure(world.run(
        "10:40",
        &Command::End(End {
            at: Some(berlin("09:45")?),
            ..End::default()
        }),
    ))?;

    assert_eq!(error.to_string(), "the break began at 10:00, after 09:45");
    Ok(())
}

#[test]
fn end_at_the_start_is_refused_as_log_and_adjust_refuse_it() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;

    let error = failure(world.run(
        "10:00",
        &Command::End(End {
            at: Some(berlin("09:00")?),
            ..End::default()
        }),
    ))?;

    assert_eq!(
        error.to_string(),
        "the entry would end at 09:00, the moment it starts"
    );
    assert_eq!(error.hint(), Some("give an end after the start"));
    // A value the message doesn't name would show up as a detail.
    assert_eq!(error.context_value("start"), None);
    Ok(())
}

#[test]
fn end_names_the_entry_when_two_are_open() -> TestResult {
    let mut laptop = World::new()?;
    let mut desktop = World::on_device(2)?;
    _ = laptop.run("09:00", &begin("on the laptop"))?;
    _ = desktop.run("09:30", &begin("on the desktop"))?;
    laptop.log.extend(desktop.log);
    let desktop_entry = laptop
        .state()
        .open()
        .find(|entry| entry.note.as_deref() == Some("on the desktop"))
        .map(|entry| entry.id)
        .ok_or("desktop entry")?;

    let error = failure(laptop.run("10:00", &Command::End(End::default())))?;
    assert!(
        error.to_string().starts_with("several entries are open: "),
        "{error}"
    );
    assert_eq!(error.hint(), Some("name one with `pace end <id>`"));
    _ = laptop.run(
        "10:00",
        &Command::End(End {
            entry: Some(desktop_entry),
            ..End::default()
        }),
    )?;

    assert_eq!(laptop.state().open().count(), 1);
    Ok(())
}

#[test]
fn end_of_ended_entry_has_nothing_to_do() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::End(End::default()))?;
    let entry = world.only_entry(EntryState::Ended)?;

    let error = failure(world.run(
        "11:00",
        &Command::End(End {
            entry: Some(entry),
            ..End::default()
        }),
    ))?;

    assert_eq!(error.to_string(), "the entry ended at 10:00");
    assert_eq!(error.hint(), Some("`pace adjust --end` moves its end"));
    assert_eq!(error.exit_code(), ExitCode::NothingToDo);
    Ok(())
}

// log

#[test]
fn log_enters_an_ended_entry_afterwards() -> TestResult {
    let mut world = World::new()?;

    let batch = world.run(
        "18:00",
        &Command::Log(Log {
            note: Some("Call with ACME".parse()?),
            ..Log::new(berlin("14:00")?, berlin("15:30")?)
        }),
    )?;

    assert_eq!(kinds(&batch), ["entry.logged"]);
    _ = world.only_entry(EntryState::Ended)?;
    Ok(())
}

#[test]
fn log_with_the_end_before_the_start_is_refused() -> TestResult {
    let mut world = World::new()?;

    let error = failure(world.run(
        "18:00",
        &Command::Log(Log::new(berlin("15:30")?, berlin("14:00")?)),
    ))?;

    assert_eq!(
        error.to_string(),
        "the entry would end at 14:00, before it starts at 15:30"
    );
    Ok(())
}

#[test]
fn log_without_length_is_refused() -> TestResult {
    let mut world = World::new()?;

    let error = failure(world.run(
        "18:00",
        &Command::Log(Log::new(berlin("15:30")?, berlin("15:30")?)),
    ))?;

    assert_eq!(
        error.to_string(),
        "the entry would end at 15:30, the moment it starts"
    );
    Ok(())
}

#[test]
fn log_into_another_entry_is_an_overlap() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;

    let error = failure(world.run(
        "18:00",
        &Command::Log(Log::new(berlin("14:00")?, berlin("15:00")?)),
    ))?;

    assert!(
        error.to_string().starts_with("the new entry would overlap"),
        "{error}"
    );
    Ok(())
}

#[test]
fn log_into_another_entry_can_be_allowed() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;

    let batch = world.run(
        "18:00",
        &Command::Log(Log {
            allow_overlap: true,
            ..Log::new(berlin("14:00")?, berlin("15:00")?)
        }),
    )?;

    assert_eq!(kinds(&batch), ["entry.logged"]);
    Ok(())
}

#[test]
fn log_into_the_future_is_refused() -> TestResult {
    let mut world = World::new()?;

    let error = failure(world.run(
        "18:00",
        &Command::Log(Log::new(berlin("17:30")?, berlin("18:30")?)),
    ))?;

    assert_eq!(
        error.to_string(),
        "the entry would end at 18:30, in the future"
    );
    assert_eq!(
        error.hint(),
        Some("pace records what happened; `pace begin` starts an entry now")
    );
    Ok(())
}

// adjust

#[test]
fn adjust_changes_the_open_entry() -> TestResult {
    let mut world = World::new()?;
    _ = world.run(
        "09:00",
        &Command::Begin(Begin {
            tags: BTreeSet::from(["dev".parse()?, "draft".parse()?]),
            ..Begin::default()
        }),
    )?;

    let batch = world.run(
        "09:30",
        &Command::Adjust(Adjust {
            start: Some(berlin("08:45")?),
            note: Change::Set("Write offer".parse()?),
            tag: BTreeSet::from(["offer".parse()?]),
            untag: BTreeSet::from(["draft".parse()?]),
            ..Adjust::default()
        }),
    )?;

    assert_eq!(kinds(&batch), ["entry.adjusted"]);
    let state = world.state();
    let entry = state
        .entry(world.only_entry(EntryState::Running)?)
        .ok_or("entry")?;
    assert_eq!(entry.start, berlin("08:45")?);
    assert_eq!(entry.note.as_deref(), Some("Write offer"));
    assert_eq!(
        entry.tags,
        BTreeSet::from(["dev".parse()?, "offer".parse()?])
    );
    Ok(())
}

#[test]
fn adjust_without_change_has_nothing_to_do() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;

    let error = failure(world.run("09:30", &Command::Adjust(Adjust::default())))?;

    assert_eq!(error.exit_code(), ExitCode::NothingToDo);
    Ok(())
}

#[test]
fn adjust_keeps_the_start_before_the_end() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::End(End::default()))?;
    let entry = world.only_entry(EntryState::Ended)?;

    let error = failure(world.run(
        "11:00",
        &Command::Adjust(Adjust {
            entry: Some(entry),
            start: Some(berlin("10:30")?),
            ..Adjust::default()
        }),
    ))?;

    assert_eq!(
        error.to_string(),
        "the entry would end at 10:00, before it starts at 10:30"
    );
    Ok(())
}

#[test]
fn adjust_of_tags_leaves_entry_without_length_alone() -> TestResult {
    let mut world = World::new()?;
    world.record(
        "09:00",
        "09:00",
        EventKind::EntryLogged {
            entry: "0193a8d0-0000-7000-8000-0000000000e1".parse()?,
            end: berlin("09:00")?,
            project: None,
            note: None,
            tags: BTreeSet::new(),
            role: None,
            determined_by: None,
            billable: None,
            allow_overlap: false,
        },
    )?;
    let entry = world.only_entry(EntryState::Ended)?;

    let batch = world.run(
        "10:00",
        &Command::Adjust(Adjust {
            entry: Some(entry),
            tag: BTreeSet::from(["dev".parse()?]),
            ..Adjust::default()
        }),
    )?;

    assert_eq!(kinds(&batch), ["entry.adjusted"]);
    Ok(())
}

#[test]
fn adjust_of_note_leaves_break_past_end_alone() -> TestResult {
    let mut world = World::new()?;
    let entry: EntryId = "0193a8d0-0000-7000-8000-0000000000e2".parse()?;
    world.record(
        "09:00",
        "09:00",
        EventKind::EntryBegun {
            entry,
            project: None,
            note: None,
            tags: BTreeSet::new(),
            role: None,
            determined_by: None,
            billable: None,
            allow_overlap: false,
        },
    )?;
    world.record(
        "10:00",
        "10:00",
        EventKind::EntryHeld {
            entry,
            source: HoldSource::Unknown,
            reason: None,
        },
    )?;
    world.record("10:30", "10:30", EventKind::EntryResumed { entry })?;
    // Another device ended it at 10:15 and synced after the resume.
    world.record("10:40", "10:15", EventKind::EntryEnded { entry })?;
    let state = world.state();
    let held = &state.entry(entry).ok_or("entry")?.holds[0];
    assert_eq!(held.end, Some(berlin("10:30")?));

    let batch = world.run(
        "11:00",
        &Command::Adjust(Adjust {
            entry: Some(entry),
            note: Change::Set("Write offer".parse()?),
            ..Adjust::default()
        }),
    )?;

    assert_eq!(kinds(&batch), ["entry.adjusted"]);
    Ok(())
}

#[test]
fn adjust_of_end_to_start_is_refused() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::End(End::default()))?;
    let entry = world.only_entry(EntryState::Ended)?;

    let error = failure(world.run(
        "11:00",
        &Command::Adjust(Adjust {
            entry: Some(entry),
            end: Some(berlin("09:00")?),
            ..Adjust::default()
        }),
    ))?;

    assert_eq!(
        error.to_string(),
        "the entry would end at 09:00, the moment it starts"
    );
    Ok(())
}

#[test]
fn adjust_does_not_end_an_open_entry() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;

    let error = failure(world.run(
        "11:00",
        &Command::Adjust(Adjust {
            end: Some(berlin("10:00")?),
            ..Adjust::default()
        }),
    ))?;

    assert_eq!(error.hint(), Some("end it with `pace end --at 10:00`"));
    Ok(())
}

#[test]
fn adjust_into_the_future_is_refused() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::End(End::default()))?;
    let entry = world.only_entry(EntryState::Ended)?;

    let error = failure(world.run(
        "11:00",
        &Command::Adjust(Adjust {
            entry: Some(entry),
            end: Some(berlin("11:30")?),
            ..Adjust::default()
        }),
    ))?;

    assert_eq!(
        error.to_string(),
        "the entry would end at 11:30, in the future"
    );
    Ok(())
}

/// A device with "Write offer" from 09:00 to 10:00 and "Call" from 10:00 to
/// 11:00, and the ID of the call.
fn offer_then_call() -> Result<(World, EntryId), Box<dyn std::error::Error>> {
    let mut world = World::new()?;
    _ = world.run("18:00", &log("Write offer", "09:00", "10:00")?)?;
    _ = world.run("18:00", &log("Call", "10:00", "11:00")?)?;
    let call = world
        .state()
        .entries()
        .find(|entry| entry.note.as_deref() == Some("Call"))
        .map(|entry| entry.id)
        .ok_or("the call")?;
    Ok((world, call))
}

fn log(note: &str, start: &str, end: &str) -> Result<Command, Box<dyn std::error::Error>> {
    Ok(Command::Log(Log {
        note: Some(note.parse()?),
        ..Log::new(berlin(start)?, berlin(end)?)
    }))
}

fn call_moved_to(entry: EntryId, start: &str, allow_overlap: bool) -> Result<Command, jiff::Error> {
    Ok(Command::Adjust(Adjust {
        entry: Some(entry),
        start: Some(berlin(start)?),
        allow_overlap,
        ..Adjust::default()
    }))
}

#[test]
fn adjust_into_another_entry_is_an_overlap() -> TestResult {
    let (mut world, call) = offer_then_call()?;

    let error = failure(world.run("18:05", &call_moved_to(call, "09:30", false)?))?;

    assert!(
        error.to_string().starts_with("the entry would overlap "),
        "{error}"
    );
    assert!(error.to_string().ends_with(" (09:00–10:00)"), "{error}");
    assert_eq!(
        error.hint(),
        Some("move it, or allow it with `--allow-overlap`")
    );
    assert!(error.is_code(OVERLAP), "{error}");
    Ok(())
}

#[test]
fn adjust_into_another_entry_can_be_allowed() -> TestResult {
    let (mut world, call) = offer_then_call()?;

    let batch = world.run("18:05", &call_moved_to(call, "09:30", true)?)?;

    assert_eq!(kinds(&batch), ["entry.adjusted"]);
    Ok(())
}

#[test]
fn begin_in_the_future_is_refused() -> TestResult {
    let mut world = World::new()?;

    let error = failure(world.run("09:00", &begin_at("Write offer", "09:30")?))?;

    assert_eq!(
        error.to_string(),
        "the entry would start at 09:30, in the future"
    );
    Ok(())
}

// moments within one minute

#[test]
fn begin_within_running_entrys_first_minute_shows_seconds() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("10:00:30", &begin("Write offer"))?;

    let error = failure(world.run("10:05", &begin_at("Call", "10:00")?))?;

    assert_eq!(
        error.to_string(),
        "the running entry began at 10:00:30, after the new entry's start at 10:00:00"
    );
    assert_eq!(
        error.hint(),
        Some("begin after 10:00:30, or move the running entry with `pace adjust --start`")
    );
    Ok(())
}

#[test]
fn moments_in_different_minutes_show_without_seconds() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("10:00:30", &begin("Write offer"))?;

    let error = failure(world.run("10:05", &begin_at("Call", "09:59:50")?))?;

    assert_eq!(
        error.to_string(),
        "the running entry began at 10:00, after the new entry's start at 09:59"
    );
    Ok(())
}

#[test]
fn begin_within_last_minute_of_closed_break_shows_seconds() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Pause(Pause::default()))?;
    _ = world.run("10:30:20", &Command::Resume(Resume::default()))?;

    let error = failure(world.run("10:40", &begin_at("Call ACME", "10:30")?))?;

    assert_eq!(
        error.to_string(),
        "the running entry has a break until 10:30:20, after the new entry's start at 10:30:00"
    );
    assert_eq!(error.hint(), Some("begin at 10:30:20 or later with `--at`"));
    Ok(())
}

#[test]
fn hold_within_entrys_first_minute_shows_seconds() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00:30", &begin("Write offer"))?;

    let error = failure(world.run(
        "10:00",
        &Command::Hold(Hold {
            at: Some(berlin("09:00")?),
            ..Hold::default()
        }),
    ))?;

    assert_eq!(
        error.to_string(),
        "the entry began at 09:00:30, after 09:00:00"
    );
    assert_eq!(error.hint(), Some("give a time after 09:00:30"));
    Ok(())
}

#[test]
fn hold_within_last_minute_of_closed_break_shows_seconds() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Pause(Pause::default()))?;
    _ = world.run("10:30:20", &Command::Resume(Resume::default()))?;

    let error = failure(world.run(
        "10:40",
        &Command::Hold(Hold {
            at: Some(berlin("10:30")?),
            ..Hold::default()
        }),
    ))?;

    assert_eq!(
        error.to_string(),
        "the last break ended at 10:30:20, after 10:30:00"
    );
    Ok(())
}

#[test]
fn end_within_first_minute_of_open_break_shows_seconds() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00:20", &Command::Hold(Hold::default()))?;

    let error = failure(world.run(
        "10:30",
        &Command::End(End {
            at: Some(berlin("10:00")?),
            ..End::default()
        }),
    ))?;

    assert_eq!(
        error.to_string(),
        "the break began at 10:00:20, after 10:00:00"
    );
    Ok(())
}

#[test]
fn log_ending_within_its_first_minute_before_start_shows_seconds() -> TestResult {
    let mut world = World::new()?;

    let error = failure(world.run(
        "18:00",
        &Command::Log(Log::new(berlin("15:30:40")?, berlin("15:30:10")?)),
    ))?;

    assert_eq!(
        error.to_string(),
        "the entry would end at 15:30:10, before it starts at 15:30:40"
    );
    Ok(())
}

#[test]
fn adjust_of_start_past_break_within_its_minute_shows_seconds() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00:20", &Command::Hold(Hold::default()))?;
    _ = world.run("10:30", &Command::Resume(Resume::default()))?;
    _ = world.run("11:00", &Command::End(End::default()))?;
    let entry = world.only_entry(EntryState::Ended)?;

    let error = failure(world.run(
        "11:30",
        &Command::Adjust(Adjust {
            entry: Some(entry),
            start: Some(berlin("10:00:40")?),
            ..Adjust::default()
        }),
    ))?;

    assert_eq!(
        error.to_string(),
        "the entry has a break at 10:00:20, before the new start at 10:00:40"
    );
    Ok(())
}

#[test]
fn adjust_of_end_before_break_end_within_its_minute_shows_seconds() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Hold(Hold::default()))?;
    _ = world.run("10:30:20", &Command::Resume(Resume::default()))?;
    _ = world.run("11:00", &Command::End(End::default()))?;
    let entry = world.only_entry(EntryState::Ended)?;

    let error = failure(world.run(
        "11:30",
        &Command::Adjust(Adjust {
            entry: Some(entry),
            end: Some(berlin("10:30")?),
            ..Adjust::default()
        }),
    ))?;

    assert_eq!(
        error.to_string(),
        "the entry has a break until 10:30:20, after the new end at 10:30:00"
    );
    Ok(())
}

#[test]
fn begin_within_last_minute_of_ended_entry_shows_seconds() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00:30", &Command::End(End::default()))?;

    let error = failure(world.run("10:30", &begin_at("Call", "10:00")?))?;

    assert!(error.to_string().ends_with("(09:00–10:00:30)"), "{error}");
    assert_eq!(
        error.hint(),
        Some("begin at 10:00:30 or later, or allow it with `--allow-overlap`")
    );
    Ok(())
}

#[test]
fn begin_later_within_current_minute_shows_seconds() -> TestResult {
    let mut world = World::new()?;

    let error = failure(world.run("10:00:10", &begin_at("Write offer", "10:00:30")?))?;

    assert_eq!(
        error.to_string(),
        "the entry would start at 10:00:30, in the future"
    );
    Ok(())
}

// undo

#[test]
fn undo_reverts_the_last_command_of_this_device() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::End(End::default()))?;

    let batch = world.run("10:01", &Command::Undo)?;

    assert_eq!(kinds(&batch), ["batch.undone"]);
    _ = world.only_entry(EntryState::Running)?;
    Ok(())
}

#[test]
fn undo_again_goes_one_command_further_back() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::End(End::default()))?;

    _ = world.run("10:01", &Command::Undo)?;
    _ = world.run("10:02", &Command::Undo)?;

    assert_eq!(world.state().entries().count(), 0);
    Ok(())
}

#[test]
fn undo_with_nothing_left_has_nothing_to_do() -> TestResult {
    let mut world = World::new()?;

    let error = failure(world.run("10:00", &Command::Undo))?;

    assert_eq!(error.to_string(), "this device has nothing left to undo");
    assert_eq!(error.exit_code(), ExitCode::NothingToDo);
    Ok(())
}

#[test]
fn undo_leaves_other_devices_alone() -> TestResult {
    let mut laptop = World::new()?;
    let mut desktop = World::on_device(2)?;
    _ = desktop.run("09:00", &begin("on the desktop"))?;
    laptop.log.extend(desktop.log);

    let error = failure(laptop.run("10:00", &Command::Undo))?;

    assert_eq!(error.exit_code(), ExitCode::NothingToDo);
    Ok(())
}

#[test]
fn undo_is_filed_with_every_period_its_batch_touched() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("2026-10-31T23:00[Europe/Berlin]", &begin("late"))?;
    let switch = world.run("2026-11-01T00:30[Europe/Berlin]", &begin("early"))?;

    let undo = world.run("2026-11-01T00:31[Europe/Berlin]", &Command::Undo)?;

    assert_eq!(kinds(&undo), ["batch.undone", "batch.undone"]);
    let abouts: Vec<_> = undo
        .events
        .iter()
        .map(|event| event.about.clone())
        .collect();
    assert_eq!(
        abouts,
        [
            at("2026-10-31T23:00[Europe/Berlin]")?,
            at("2026-11-01T00:30[Europe/Berlin]")?
        ]
    );
    assert!(undo.events.iter().all(|event| matches!(
        event.kind,
        EventKind::BatchUndone { batch } if batch == switch.id
    )));
    Ok(())
}

// forget

#[test]
fn forget_writes_tombstone_that_undo_skips() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    let entry = world.only_entry(EntryState::Running)?;

    let batch = world.run("10:00", &Command::Forget(Forget { entry }))?;
    let undo = failure(world.run("10:01", &Command::Undo))?;

    assert_eq!(kinds(&batch), ["entry.forgotten"]);
    assert_eq!(undo.exit_code(), ExitCode::NothingToDo);
    assert_eq!(world.state().entries().count(), 0);
    Ok(())
}

// filing

#[test]
fn an_entrys_events_are_filed_under_its_first_start() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run(
        "09:10",
        &Command::Adjust(Adjust {
            start: Some(berlin("08:30")?),
            ..Adjust::default()
        }),
    )?;

    let batch = world.run("10:00", &Command::Hold(Hold::default()))?;

    assert_eq!(batch.events[0].at, berlin("10:00")?);
    assert_eq!(batch.events[0].about, berlin("09:00")?);
    Ok(())
}

// the open entry

/// Two devices with an entry open on each, merged on the laptop.
fn two_open() -> Result<World, Box<dyn std::error::Error>> {
    let mut laptop = World::new()?;
    let mut desktop = World::on_device(2)?;
    _ = laptop.run("09:00", &begin("on the laptop"))?;
    _ = desktop.run("09:30", &begin("on the desktop"))?;
    laptop.log.extend(desktop.log);
    Ok(laptop)
}

#[test]
fn several_open_entries_are_listed_by_label() -> TestResult {
    let world = two_open()?;
    let state = world.state();
    let open: Vec<String> = state.open().map(|entry| state.short_id(entry.id)).collect();

    let error = state
        .only_open(EntryVerb::Forget, &berlin("10:00")?)
        .err()
        .ok_or("two open entries are no only one")?;

    assert_eq!(
        error.to_string(),
        format!(
            "several entries are open: {} \"on the laptop\" (since 09:00); {} \"on the desktop\" (since 09:30)",
            open[0], open[1]
        )
    );
    assert_eq!(error.exit_code(), ExitCode::Error);
    Ok(())
}

#[test]
fn adjust_and_forget_name_one_of_several_open_entries_alike() -> TestResult {
    let mut world = two_open()?;
    let state = world.state();

    let forget = state
        .only_open(EntryVerb::Forget, &berlin("10:00")?)
        .err()
        .ok_or("two open entries are no only one")?;
    let adjust = failure(world.run(
        "10:00",
        &Command::Adjust(Adjust {
            note: Change::Set("Call".parse()?),
            ..Adjust::default()
        }),
    ))?;

    assert_eq!(forget.hint(), Some("name one with `pace forget <id>`"));
    assert_eq!(adjust.hint(), Some("name one with `pace adjust <id>`"));
    assert_eq!(adjust.to_string(), forget.to_string());
    Ok(())
}

#[test]
fn nothing_running_names_flags_of_command_that_takes_them() -> TestResult {
    let state = World::new()?.state();
    let now = berlin("10:00")?;

    let forget = state
        .only_open(EntryVerb::Forget, &now)
        .err()
        .ok_or("nothing is open")?;
    let hold = state
        .only_open(EntryVerb::Hold, &now)
        .err()
        .ok_or("nothing is open")?;

    assert_eq!(forget.to_string(), "nothing is running");
    assert_eq!(
        forget.hint(),
        Some(
            "name the entry with `pace forget <id>`, or take the last one with `pace forget --last`"
        )
    );
    assert_eq!(forget.exit_code(), ExitCode::NothingToDo);
    assert_eq!(hold.hint(), Some("`pace begin` starts an entry"));
    assert_eq!(hold.exit_code(), ExitCode::NothingToDo);
    Ok(())
}

#[test]
fn only_open_entry_is_the_one() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    let state = world.state();

    let entry = state.only_open(EntryVerb::End, &berlin("10:00")?)?;

    assert_eq!(entry.note.as_deref(), Some("Write offer"));
    Ok(())
}

#[rstest]
#[case::begin(EntryVerb::Begin, "begin")]
#[case::hold(EntryVerb::Hold, "hold")]
#[case::pause(EntryVerb::Pause, "pause")]
#[case::resume(EntryVerb::Resume, "resume")]
#[case::catch(EntryVerb::Catch, "catch")]
#[case::end(EntryVerb::End, "end")]
#[case::adjust(EntryVerb::Adjust, "adjust")]
#[case::edit(EntryVerb::Edit, "edit")]
#[case::forget(EntryVerb::Forget, "forget")]
fn entry_verb_is_named_as_user_types_it(#[case] verb: EntryVerb, #[case] name: &str) {
    assert_eq!(verb.name(), name);
}

// the changes of an adjustment

#[test]
fn adjustment_changes_only_values_entry_lacks() -> TestResult {
    let mut world = World::new()?;
    _ = world.run(
        "09:00",
        &Command::Begin(Begin {
            note: Some("Write offer".parse()?),
            tags: BTreeSet::from(["client".parse()?]),
            ..Begin::default()
        }),
    )?;
    let state = world.state();
    let entry = state.open().next().ok_or("the open entry")?;

    let same = Adjust {
        start: Some(berlin("09:00")?),
        note: Change::Set("Write offer".parse()?),
        project: Change::Clear,
        tag: BTreeSet::from(["client".parse()?]),
        ..Adjust::default()
    };
    let other = Adjust {
        note: Change::Set("Call".parse()?),
        untag: BTreeSet::from(["client".parse()?]),
        ..same.clone()
    };

    assert!(same.changes(entry).is_empty(), "{:?}", same.changes(entry));
    let changes = other.changes(entry);
    assert_eq!(changes.start, None);
    assert_eq!(changes.note, Change::Set("Call".to_owned()));
    assert_eq!(changes.project, Change::Keep);
    assert_eq!(changes.tags, Some(BTreeSet::new()));
    Ok(())
}

// role, determined by, billable, allowed overlap

const WORK_ID: &str = "0193a8d0-000f-7000-8000-000000000001";
const FAMILY_ID: &str = "0193a8d0-000f-7000-8000-000000000002";

fn role_id(id: &str) -> Result<RoleId, Box<dyn std::error::Error>> {
    Ok(id.parse()?)
}

fn role(id: &str, key: &str) -> Result<Role, Box<dyn std::error::Error>> {
    Ok(Role {
        id: id.parse()?,
        key: key.parse()?,
        kind: RoleKind::Area,
        name: key.to_owned(),
        purpose: None,
        weekly_target: None,
        archived: None,
    })
}

/// The project `acme-webshop` with the role `work`, billable, and whether
/// its entries may overlap others.
fn webshop(allow_overlap: bool) -> Result<ProjectRecord, Box<dyn std::error::Error>> {
    Ok(ProjectRecord {
        id: "0193a8d0-000c-7000-8000-000000000001".parse()?,
        key: "acme-webshop".parse()?,
        name: "Webshop relaunch".to_owned(),
        client: None,
        role: Some("work".parse()?),
        billable: true,
        budget: None,
        starts_on: None,
        ends_on: None,
        done_when: None,
        allow_overlap,
        archived: None,
    })
}

/// A world whose catalog holds the roles `work` and `family` and the
/// project `acme-webshop`.
fn world_with_webshop(allow_overlap: bool) -> Result<World, Box<dyn std::error::Error>> {
    let mut world = World::new()?;
    world.catalog = Catalog::new(
        vec![role(WORK_ID, "work")?, role(FAMILY_ID, "family")?],
        Vec::new(),
        vec![webshop(allow_overlap)?],
    )?;
    Ok(world)
}

fn webshop_key() -> Result<ProjectRef, Box<dyn std::error::Error>> {
    Ok(ProjectRef::Key("acme-webshop".parse()?))
}

/// A begin at `time` in `acme-webshop`, without `--allow-overlap`.
fn begin_in_webshop(time: &str) -> Result<Command, Box<dyn std::error::Error>> {
    Ok(Command::Begin(Begin {
        at: Some(berlin(time)?),
        project: Some(webshop_key()?),
        ..Begin::default()
    }))
}

#[test]
fn begin_in_project_that_allows_overlap_overlaps_without_flag() -> TestResult {
    let mut world = world_with_webshop(true)?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::End(End::default()))?;

    let batch = world.run("10:30", &begin_in_webshop("09:30")?)?;

    assert_eq!(kinds(&batch), ["entry.begun"]);
    let [
        Event {
            kind: EventKind::EntryBegun { allow_overlap, .. },
            ..
        },
    ] = batch.events.as_slice()
    else {
        return Err("one entry.begun".into());
    };
    assert!(
        !allow_overlap,
        "the project allows it, the flag wasn't given"
    );
    Ok(())
}

#[test]
fn begin_in_project_without_allow_overlap_is_still_refused() -> TestResult {
    let mut world = world_with_webshop(false)?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::End(End::default()))?;

    let error = world
        .run("10:30", &begin_in_webshop("09:30")?)
        .err()
        .ok_or("the overlap must be refused")?;

    assert!(error.is_code(OVERLAP), "{error}");
    Ok(())
}

#[test]
fn log_in_project_that_allows_overlap_overlaps_without_flag() -> TestResult {
    let mut world = world_with_webshop(true)?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::End(End::default()))?;

    let batch = world.run(
        "11:00",
        &Command::Log(Log {
            project: Some(webshop_key()?),
            ..Log::new(berlin("09:30")?, berlin("10:30")?)
        }),
    )?;

    assert_eq!(kinds(&batch), ["entry.logged"]);
    Ok(())
}

#[test]
fn adjust_into_project_that_allows_overlap_moves_without_flag() -> TestResult {
    let mut world = world_with_webshop(true)?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::End(End::default()))?;
    _ = world.run("10:00", &begin("Review"))?;
    let review = world.only_entry(EntryState::Running)?;

    let batch = world.run(
        "10:30",
        &Command::Adjust(Adjust {
            entry: Some(review),
            start: Some(berlin("09:30")?),
            project: Change::Set(webshop_key()?),
            ..Adjust::default()
        }),
    )?;

    assert_eq!(kinds(&batch), ["entry.adjusted"]);
    Ok(())
}

/// The role, determined-by, billable and allow-overlap of a new entry.
type EntryFields = (Option<RoleId>, Option<DeterminedBy>, Option<bool>, bool);

/// The [`EntryFields`] of the `entry.begun` or `entry.logged` of `batch`.
fn entry_fields(batch: &Batch) -> Result<EntryFields, Box<dyn std::error::Error>> {
    batch
        .events
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::EntryBegun {
                role,
                determined_by,
                billable,
                allow_overlap,
                ..
            }
            | EventKind::EntryLogged {
                role,
                determined_by,
                billable,
                allow_overlap,
                ..
            } => Some((*role, *determined_by, *billable, *allow_overlap)),
            _ => None,
        })
        .ok_or_else(|| format!("expected a new entry, found {:?}", batch.events).into())
}

#[test]
fn begin_writes_role_and_billable_only_where_they_differ_from_project() -> TestResult {
    let mut world = world_with_webshop(false)?;
    let begin = |role: &str, billable: bool| -> Result<Command, Box<dyn std::error::Error>> {
        Ok(Command::Begin(Begin {
            project: Some(webshop_key()?),
            role: Some(role_id(role)?),
            billable: Some(billable),
            ..Begin::default()
        }))
    };

    let same = world.run("09:00", &begin(WORK_ID, true)?)?;
    let other = world.run("10:00", &begin(FAMILY_ID, false)?)?;

    assert_eq!(entry_fields(&same)?, (None, None, None, false));
    assert_eq!(
        entry_fields(&other)?,
        (Some(role_id(FAMILY_ID)?), None, Some(false), false)
    );
    Ok(())
}

#[test]
fn log_without_project_writes_role_and_billable_but_not_billable_false() -> TestResult {
    let mut world = world_with_webshop(false)?;
    let log =
        |billable: bool, start: &str, end: &str| -> Result<Command, Box<dyn std::error::Error>> {
            Ok(Command::Log(Log {
                role: Some(role_id(WORK_ID)?),
                determined_by: Some(DeterminedBy::Others),
                billable: Some(billable),
                allow_overlap: true,
                ..Log::new(berlin(start)?, berlin(end)?)
            }))
        };

    let billable = world.run("12:00", &log(true, "09:00", "10:00")?)?;
    let not_billable = world.run("12:00", &log(false, "10:00", "11:00")?)?;

    assert_eq!(
        entry_fields(&billable)?,
        (
            Some(role_id(WORK_ID)?),
            Some(DeterminedBy::Others),
            Some(true),
            true
        )
    );
    assert_eq!(
        entry_fields(&not_billable)?,
        (
            Some(role_id(WORK_ID)?),
            Some(DeterminedBy::Others),
            None,
            true
        )
    );
    Ok(())
}

#[test]
fn effective_role_and_billable_fall_back_to_project_record() -> TestResult {
    let mut world = world_with_webshop(false)?;
    _ = world.run("09:00", &begin_in_webshop("09:00")?)?;
    _ = world.run(
        "10:00",
        &Command::Begin(Begin {
            project: Some(webshop_key()?),
            role: Some(role_id(FAMILY_ID)?),
            billable: Some(false),
            ..Begin::default()
        }),
    )?;
    _ = world.run("11:00", &begin("Without project"))?;

    let state = world.state();
    let entries: Vec<_> = state.entries().collect();
    let effective: Vec<_> = entries
        .iter()
        .map(|entry| {
            (
                entry.effective_role(&world.catalog),
                entry.effective_billable(&world.catalog),
            )
        })
        .collect();

    assert_eq!(
        effective,
        [
            (Some(role_id(WORK_ID)?), true),
            (Some(role_id(FAMILY_ID)?), false),
            (None, false),
        ]
    );
    Ok(())
}

#[test]
fn changed_project_role_applies_to_old_entries() -> TestResult {
    let mut world = world_with_webshop(false)?;
    _ = world.run("09:00", &begin_in_webshop("09:00")?)?;
    let mut moved = webshop(false)?;
    moved.role = Some("family".parse()?);
    world.catalog = Catalog::new(
        vec![role(WORK_ID, "work")?, role(FAMILY_ID, "family")?],
        Vec::new(),
        vec![moved],
    )?;

    let state = world.state();
    let entry = state
        .entry(world.only_entry(EntryState::Running)?)
        .ok_or("entry")?;

    assert_eq!(
        entry.effective_role(&world.catalog),
        Some(role_id(FAMILY_ID)?)
    );
    Ok(())
}

#[test]
fn adjust_with_allow_overlap_records_it_once() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    let allow = Command::Adjust(Adjust {
        allow_overlap: true,
        ..Adjust::default()
    });

    let batch = world.run("09:10", &allow)?;
    let again = world
        .run("09:20", &allow)
        .err()
        .ok_or("the second must have nothing to do")?;

    let [
        Event {
            kind: EventKind::EntryAdjusted { changes, .. },
            ..
        },
    ] = batch.events.as_slice()
    else {
        return Err("one entry.adjusted".into());
    };
    assert!(changes.allow_overlap);
    assert_eq!(again.exit_code(), ExitCode::NothingToDo);
    Ok(())
}

#[test]
fn adjust_changes_role_determined_by_and_billable_only_where_they_differ() -> TestResult {
    let mut world = World::new()?;
    _ = world.run(
        "09:00",
        &Command::Begin(Begin {
            role: Some(role_id(WORK_ID)?),
            determined_by: Some(DeterminedBy::Me),
            ..Begin::default()
        }),
    )?;
    let entry = world.only_entry(EntryState::Running)?;
    let state = world.state();
    let entry = state.entry(entry).ok_or("entry")?;
    let adjust = Adjust {
        role: Change::Set(role_id(WORK_ID)?),
        determined_by: Change::Set(DeterminedBy::Others),
        billable: Change::Clear,
        ..Adjust::default()
    };

    let changes = adjust.changes(entry);

    assert_eq!(changes.role, Change::Keep);
    assert_eq!(changes.determined_by, Change::Set(DeterminedBy::Others));
    assert_eq!(changes.billable, Change::Keep);
    Ok(())
}
