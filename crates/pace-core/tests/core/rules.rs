//! What each command turns into: the events of one batch, or an error that
//! says how to fix it. Scenarios marked "old pace" come from the 19
//! state-machine tests of the old in-memory backend.

use std::collections::BTreeSet;

use jiff::ZonedDateTime;
use pace_core::{
    Batch, Change, Command, DeviceId, EntryId, EntryState, EventKind, Hlc, HoldSource, Ids,
    PauseKind, Policy, ProjectRef, RecordedEvent, Rules, SequentialIds, State,
    command::{Adjust, Begin, Catch, End, Forget, Hold, Log, Pause, Resume},
    short_ids,
};
use pace_error::{ExitCode, PaceError};

use crate::TestResult;

/// A device that decides commands against its state and appends the
/// batches to an in-memory log.
struct World {
    device: DeviceId,
    ids: SequentialIds,
    policy: Policy,
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
        .map_err(|error| PaceError::from(pace_error::ErrorKind::Internal, error))?;
        let state = self.state();
        let batch = Rules {
            state: &state,
            now: &now,
            device: self.device,
            ids: &self.ids,
            policy: &self.policy,
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
        note: Some(note.to_owned()),
        ..Begin::default()
    })
}

fn begin_at(note: &str, time: &str) -> Result<Command, jiff::Error> {
    Ok(Command::Begin(Begin {
        at: Some(berlin(time)?),
        note: Some(note.to_owned()),
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
            _ => "unknown",
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
fn begin_starts_a_running_entry_now() -> TestResult {
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
        error.to_string().starts_with("2 entries are open"),
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
            reason: Some("phone call".to_owned()),
            ..Hold::default()
        }),
    )?;

    assert_eq!(kinds(&batch), ["entry.held"]);
    _ = world.only_entry(EntryState::Held)?;
    Ok(())
}

#[test]
fn hold_without_a_source_takes_the_setting() -> TestResult {
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
fn a_second_hold_without_a_reason_has_nothing_to_do() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Hold(Hold::default()))?;

    let error = failure(world.run("10:05", &Command::Hold(Hold::default())))?;

    assert_eq!(error.to_string(), "the entry is held since 10:00");
    assert_eq!(error.exit_code(), ExitCode::NothingToDo);
    Ok(())
}

#[test]
fn a_second_hold_with_a_reason_updates_it() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    _ = world.run("10:00", &Command::Hold(Hold::default()))?;

    _ = world.run(
        "10:05",
        &Command::Hold(Hold {
            reason: Some("Slack".to_owned()),
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
fn pause_while_held_turns_the_hold_into_a_pause() -> TestResult {
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

// catch

#[test]
fn catch_notes_a_thought_on_the_running_entry() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;

    let batch = world.run(
        "09:30",
        &Command::Catch(Catch {
            text: "call the tax office".to_owned(),
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
            text: "call the tax office".to_owned(),
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

    assert!(
        failure(laptop.run("10:00", &Command::End(End::default())))?
            .to_string()
            .starts_with("2 entries are open")
    );
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

// log

#[test]
fn log_enters_an_ended_entry_afterwards() -> TestResult {
    let mut world = World::new()?;

    let batch = world.run(
        "18:00",
        &Command::Log(Log {
            note: Some("Call with ACME".to_owned()),
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
            note: Change::Set("Write offer".to_owned()),
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
fn adjust_without_a_change_has_nothing_to_do() -> TestResult {
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
fn forget_writes_a_tombstone_that_undo_skips() -> TestResult {
    let mut world = World::new()?;
    _ = world.run("09:00", &begin("Write offer"))?;
    let entry = world.only_entry(EntryState::Running)?;

    let batch = world.run("10:00", &Command::Forget(Forget { entry }))?;
    _ = world.run("10:01", &Command::Undo)?;

    assert_eq!(kinds(&batch), ["entry.forgotten"]);
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

// short IDs

#[test]
fn short_ids_are_the_last_four_characters_or_more_when_needed() -> TestResult {
    let ids: Vec<EntryId> = [
        "0193a8d0-6c1e-7f3a-9b2d-4e5f60719a2c",
        "0193a8d0-6c1e-7f3a-9b2d-4e5f60729a2c",
        "0193a8d0-6c1e-7f3a-9b2d-4e5f607100ff",
    ]
    .iter()
    .map(|id| id.parse())
    .collect::<Result<_, _>>()?;

    let short = short_ids(&ids);

    assert_eq!(short[&ids[0]], "19a2c");
    assert_eq!(short[&ids[1]], "29a2c");
    assert_eq!(short[&ids[2]], "00ff");
    Ok(())
}

#[test]
fn sequential_ids_differ_by_seed() -> TestResult {
    let laptop = SequentialIds::starting_at(&berlin("08:00")?, 1);
    let desktop = SequentialIds::starting_at(&berlin("08:00")?, 2);

    assert_ne!(laptop.next_id(), desktop.next_id());
    Ok(())
}

#[test]
fn sequential_ids_are_uuidv7_and_distinct() -> TestResult {
    let ids = SequentialIds::starting_at(&berlin("08:00")?, 0);

    let first = ids.next_id();
    let second = ids.next_id();

    assert_ne!(first, second);
    assert_eq!(first.get_version_num(), 7);
    Ok(())
}
