//! The step machine of the tracker, driven with values only: no store, no
//! clock, fixed times and sequential IDs.

use jiff::ToSpan;
use jiff::ZonedDateTime;
use pace_core::{
    AfterAppend, AfterRead, Appended, Batch, Change, ChangeMarker, Command, Decided, DeviceId,
    EntryId, EventKind, Hlc, Policy, ReadScope, Reading, RecordedEvent, SequentialIds,
    command::{Adjust, Begin, Catch, End, Forget, Hold, Log, Pause, Resume},
};
use pace_error::{ErrorKind, ExitCode, PaceError};
use pace_time::TimeRange;

use crate::TestResult;

fn berlin(time: &str) -> Result<ZonedDateTime, jiff::Error> {
    format!("2026-11-26T{time}[Europe/Berlin]").parse()
}

fn device() -> Result<DeviceId, Box<PaceError>> {
    "0193a8d0-000d-7000-8000-000000000001".parse()
}

fn entry() -> Result<EntryId, Box<PaceError>> {
    "0193a8d0-000d-7000-8000-0000deadbeef".parse()
}

/// The events of `batch` as the store would record them on `device`.
fn recorded(batch: &Batch, device: DeviceId) -> Vec<RecordedEvent> {
    batch
        .events
        .iter()
        .zip(0..)
        .map(|(event, counter)| RecordedEvent {
            device,
            batch: batch.id,
            hlc: Hlc::new(event.at.timestamp(), counter),
            event: event.clone(),
        })
        .collect()
}

/// The events of `begin` at `at`, recorded then on [`device`].
fn begun_at(
    at: ZonedDateTime,
    ids: &SequentialIds,
    policy: &Policy,
) -> Result<Vec<RecordedEvent>, Box<dyn std::error::Error>> {
    let command = Command::Begin(Begin::default());
    let reading = Reading::start(&command, at, device()?, ids, policy)?;
    let Decided { batch, .. } = decided(reading.decide(ChangeMarker::new(1), Vec::new())?)?;
    Ok(recorded(&batch, device()?))
}

/// The entry that `events` begin.
fn begun_entry(events: &[RecordedEvent]) -> Result<EntryId, Box<dyn std::error::Error>> {
    events
        .iter()
        .find_map(|recorded| match recorded.event.kind {
            EventKind::EntryBegun { entry, .. } => Some(entry),
            _ => None,
        })
        .ok_or_else(|| "the events begin no entry".into())
}

/// The decision of a read that was wide enough.
fn decided(after: AfterRead<'_>) -> Result<Decided<'_>, Box<dyn std::error::Error>> {
    match after {
        AfterRead::Decided(decided) => Ok(*decided),
        AfterRead::ReadEverything(again) => {
            Err(format!("the read was too narrow: {:?}", again.scope()).into())
        }
    }
}

/// The scope the machine reads for `command` at `now`.
fn scope_of(
    command: &Command,
    now: &ZonedDateTime,
) -> Result<ReadScope, Box<dyn std::error::Error>> {
    let ids = SequentialIds::starting_at(now, 1);
    let policy = Policy::default();
    let reading = Reading::start(command, now.clone(), device()?, &ids, &policy)?;
    Ok(reading.scope().clone())
}

/// The range the machine reads for `command` at `now`.
fn range_of(
    command: &Command,
    now: &ZonedDateTime,
) -> Result<TimeRange, Box<dyn std::error::Error>> {
    let ids = SequentialIds::starting_at(now, 1);
    let policy = Policy::default();
    let reading = Reading::start(command, now.clone(), device()?, &ids, &policy)?;
    Ok(reading.scope().range().clone())
}

#[test]
fn begin_at_past_moment_reads_from_that_moment_to_now() -> TestResult {
    let now = berlin("10:00")?;
    let command = Command::Begin(Begin {
        at: Some(berlin("09:00")?),
        ..Begin::default()
    });

    let range = range_of(&command, &now)?;

    assert_eq!(range, TimeRange::new(Some(berlin("09:00")?), Some(now))?);
    Ok(())
}

#[test]
fn tracking_commands_without_entry_read_now_to_now() -> TestResult {
    let now = berlin("10:00")?;
    let commands = [
        Command::Hold(Hold::default()),
        Command::Pause(Pause::default()),
        Command::Catch(Catch {
            text: "call the bank".parse()?,
        }),
        Command::End(End::default()),
        Command::Resume(Resume::default()),
    ];

    for command in &commands {
        let range = range_of(command, &now)?;

        assert_eq!(
            range,
            TimeRange::new(Some(now.clone()), Some(now.clone()))?,
            "{command:?}"
        );
    }
    Ok(())
}

#[test]
fn commands_about_named_entry_read_from_month_ago() -> TestResult {
    let now = berlin("10:00")?;
    let commands = [
        Command::End(End {
            entry: Some(entry()?),
            ..End::default()
        }),
        Command::Resume(Resume {
            entry: Some(entry()?),
            ..Resume::default()
        }),
        Command::Adjust(Adjust::default()),
        Command::Forget(Forget { entry: entry()? }),
    ];

    for command in &commands {
        let range = range_of(command, &now)?;

        assert_eq!(
            range,
            TimeRange::new(Some(now.checked_sub(31.days())?), None)?,
            "{command:?}"
        );
    }
    Ok(())
}

#[test]
fn adjust_to_earlier_time_reads_from_it() -> TestResult {
    let command = Command::Adjust(Adjust {
        start: Some("2026-09-01T09:00[Europe/Berlin]".parse()?),
        end: Some("2026-09-01T08:00[Europe/Berlin]".parse()?),
        ..Adjust::default()
    });

    let range = range_of(&command, &berlin("10:00")?)?;

    assert_eq!(
        range,
        TimeRange::new(Some("2026-09-01T08:00[Europe/Berlin]".parse()?), None)?
    );
    Ok(())
}

#[test]
fn undo_reads_open_entries_and_what_was_recorded_in_last_month() -> TestResult {
    let now = berlin("10:00")?;

    let scope = scope_of(&Command::Undo, &now)?;

    assert_eq!(
        scope,
        ReadScope::of(TimeRange::new(Some(now.clone()), Some(now.clone()))?)
            .with_recorded_since(now.checked_sub(31.days())?.timestamp())
    );
    Ok(())
}

#[test]
fn undo_within_month_of_earliest_time_reads_everything() -> TestResult {
    let now: ZonedDateTime = "-009999-01-10T00:00:00+00:00[UTC]".parse()?;

    let scope = scope_of(&Command::Undo, &now)?;

    assert_eq!(scope, ReadScope::everything());
    Ok(())
}

#[test]
fn named_entry_missing_from_read_makes_command_read_everything() -> TestResult {
    let now = berlin("10:00")?;
    let ids = SequentialIds::starting_at(&now, 1);
    let policy = Policy::default();
    let command = Command::Forget(Forget { entry: entry()? });
    let reading = Reading::start(&command, now, device()?, &ids, &policy)?;

    let after = reading.decide(ChangeMarker::new(7), Vec::new())?;

    let AfterRead::ReadEverything(again) = after else {
        return Err("a forget of an entry the read lacks was decided".into());
    };
    assert_eq!(again.scope(), &ReadScope::everything());
    let error = again
        .decide(ChangeMarker::new(7), Vec::new())
        .err()
        .ok_or("a forget of an unknown entry was decided")?;
    assert!(
        error.to_string().starts_with("pace knows no entry"),
        "{error}"
    );
    Ok(())
}

#[test]
fn named_entry_in_read_is_decided_on_it() -> TestResult {
    let now = berlin("10:00")?;
    let ids = SequentialIds::starting_at(&now, 1);
    let policy = Policy::default();
    let begun = begun_at(berlin("09:00")?, &ids, &policy)?;
    let entry = begun_entry(&begun)?;
    let command = Command::Forget(Forget { entry });
    let reading = Reading::start(&command, now, device()?, &ids, &policy)?;

    let Decided { batch, .. } = decided(reading.decide(ChangeMarker::new(7), begun)?)?;

    assert!(matches!(
        batch.events[0].kind,
        EventKind::EntryForgotten { entry: forgotten } if forgotten == entry
    ));
    Ok(())
}

#[test]
fn adjust_of_entry_begun_before_read_makes_it_read_everything() -> TestResult {
    let now = berlin("10:00")?;
    let ids = SequentialIds::starting_at(&now, 1);
    let policy = Policy::default();
    // Open, so every read has it, but begun before the read's month.
    let begun = begun_at("2026-10-01T09:00[Europe/Berlin]".parse()?, &ids, &policy)?;
    let command = Command::Adjust(Adjust {
        note: Change::Set("Longer than a month".parse()?),
        ..Adjust::default()
    });
    let reading = Reading::start(&command, now, device()?, &ids, &policy)?;

    let after = reading.decide(ChangeMarker::new(7), begun)?;

    assert!(
        matches!(&after, AfterRead::ReadEverything(again) if again.scope().is_everything()),
        "{after:?}"
    );
    Ok(())
}

#[test]
fn undo_of_batch_recorded_before_read_makes_it_read_everything() -> TestResult {
    let now = berlin("10:00")?;
    let ids = SequentialIds::starting_at(&now, 1);
    let policy = Policy::default();
    // Open, so every read has it, but recorded before the read's month: a
    // later batch could lie outside the read.
    let begun = begun_at("2026-10-01T09:00[Europe/Berlin]".parse()?, &ids, &policy)?;
    let reading = Reading::start(&Command::Undo, now, device()?, &ids, &policy)?;

    let after = reading.decide(ChangeMarker::new(7), begun)?;

    assert!(
        matches!(&after, AfterRead::ReadEverything(again) if again.scope().is_everything()),
        "{after:?}"
    );
    Ok(())
}

#[test]
fn log_reads_its_own_time() -> TestResult {
    let command = Command::Log(Log::new(berlin("08:00")?, berlin("09:00")?));

    let range = range_of(&command, &berlin("10:00")?)?;

    assert_eq!(
        range,
        TimeRange::new(Some(berlin("08:00")?), Some(berlin("09:00")?))?
    );
    Ok(())
}

#[test]
fn log_ending_before_its_start_reads_its_own_time() -> TestResult {
    let command = Command::Log(Log::new(berlin("09:00")?, berlin("08:00")?));

    let range = range_of(&command, &berlin("10:00")?)?;

    assert_eq!(
        range,
        TimeRange::new(Some(berlin("08:00")?), Some(berlin("09:00")?))?
    );
    Ok(())
}

#[test]
fn refusal_of_rules_is_error_without_batch() -> TestResult {
    let now = berlin("10:00")?;
    let ids = SequentialIds::starting_at(&now, 1);
    let policy = Policy::default();
    let command = Command::End(End::default());
    let reading = Reading::start(&command, now, device()?, &ids, &policy)?;

    let error = reading
        .decide(ChangeMarker::new(7), Vec::new())
        .err()
        .ok_or("an end with nothing open was decided")?;

    assert_eq!(error.to_string(), "nothing is running");
    assert_eq!(error.exit_code(), ExitCode::NothingToDo);
    Ok(())
}

#[test]
fn decided_begin_gives_batch_with_begun_entry_and_marker_of_read() -> TestResult {
    let now = berlin("10:00")?;
    let ids = SequentialIds::starting_at(&now, 1);
    let policy = Policy::default();
    let command = Command::Begin(Begin::default());
    let reading = Reading::start(&command, now, device()?, &ids, &policy)?;

    let Decided { batch, read, .. } = decided(reading.decide(ChangeMarker::new(7), Vec::new())?)?;

    assert_eq!(read, ChangeMarker::new(7));
    assert_eq!(batch.events.len(), 1);
    assert!(matches!(batch.events[0].kind, EventKind::EntryBegun { .. }));
    Ok(())
}

#[test]
fn reading_shows_its_command_and_attempt_in_debug() -> TestResult {
    let now = berlin("10:00")?;
    let ids = SequentialIds::starting_at(&now, 1);
    let policy = Policy::default();
    let reading = Reading::start(&Command::Undo, now, device()?, &ids, &policy)?;

    let shown = format!("{reading:?}");

    assert!(
        shown.contains("CommandRun { command: Undo, device: "),
        "{shown}"
    );
    assert!(shown.contains("attempt: 1, .. }"), "{shown}");
    Ok(())
}

#[test]
fn stale_append_gives_new_read_at_new_time() -> TestResult {
    let ids = SequentialIds::starting_at(&berlin("10:00")?, 1);
    let policy = Policy::default();
    let command = Command::Begin(Begin {
        at: Some(berlin("09:00")?),
        ..Begin::default()
    });
    let reading = Reading::start(&command, berlin("10:00")?, device()?, &ids, &policy)?;
    let Decided { appending, .. } = decided(reading.decide(ChangeMarker::new(7), Vec::new())?)?;

    let after = appending.appended(Appended::Stale, berlin("10:05")?)?;

    let AfterAppend::ReadAgain(again) = after else {
        return Err(format!("a stale append gave {after:?}").into());
    };
    assert_eq!(
        again.scope().range(),
        &TimeRange::new(Some(berlin("09:00")?), Some(berlin("10:05")?))?
    );
    Ok(())
}

#[test]
fn fifth_stale_append_fails_naming_concurrent_writes() -> TestResult {
    let now = berlin("10:00")?;
    let ids = SequentialIds::starting_at(&now, 1);
    let policy = Policy::default();
    let command = Command::Begin(Begin::default());
    let mut reading = Reading::start(&command, now.clone(), device()?, &ids, &policy)?;

    for _ in 1..5 {
        let Decided { appending, .. } = decided(reading.decide(ChangeMarker::new(7), Vec::new())?)?;
        match appending.appended(Appended::Stale, now.clone())? {
            AfterAppend::ReadAgain(again) => reading = *again,
            done @ AfterAppend::Done(_) => {
                return Err(format!("a stale append gave {done:?}").into());
            }
        }
    }
    let Decided { appending, .. } = decided(reading.decide(ChangeMarker::new(7), Vec::new())?)?;
    let error = appending
        .appended(Appended::Stale, now)
        .err()
        .ok_or("the fifth stale append didn't fail")?;

    assert!(error.is_code("pace::core::concurrent-write"), "{error}");
    assert_eq!(error.context_value("attempts"), Some("5"));
    Ok(())
}

#[test]
fn recorded_append_gives_outcome_of_command() -> TestResult {
    let now = berlin("10:00")?;
    let ids = SequentialIds::starting_at(&now, 1);
    let policy = Policy::default();
    let command = Command::Begin(Begin::default());
    let reading = Reading::start(&command, now.clone(), device()?, &ids, &policy)?;
    let Decided {
        batch, appending, ..
    } = decided(reading.decide(ChangeMarker::new(7), Vec::new())?)?;
    let events = recorded(&batch, device()?);
    let warning = PaceError::new(ErrorKind::InputOutput, "the cleaning failed");

    let after = appending.appended(
        Appended::Recorded {
            events: events.clone(),
            warnings: vec![*warning],
        },
        berlin("10:05")?,
    )?;

    let AfterAppend::Done(outcome) = after else {
        return Err(format!("a recorded append gave {after:?}").into());
    };
    assert_eq!(outcome.recorded, events);
    assert_eq!(outcome.warnings.len(), 1);
    assert_eq!(outcome.before.entries().count(), 0);
    assert_eq!(outcome.state.open().count(), 1);
    assert_eq!(outcome.undone, Vec::new());
    assert_eq!(outcome.now, now);
    Ok(())
}

#[test]
fn recorded_undo_gives_events_it_undid() -> TestResult {
    let ids = SequentialIds::starting_at(&berlin("09:00")?, 1);
    let policy = Policy::default();
    let earlier = Command::Begin(Begin::default());
    let reading = Reading::start(&earlier, berlin("09:00")?, device()?, &ids, &policy)?;
    let Decided { batch, .. } = decided(reading.decide(ChangeMarker::new(7), Vec::new())?)?;
    let begun = recorded(&batch, device()?);
    let undo = Command::Undo;
    let reading = Reading::start(&undo, berlin("10:00")?, device()?, &ids, &policy)?;
    let Decided {
        batch, appending, ..
    } = decided(reading.decide(ChangeMarker::new(8), begun.clone())?)?;

    let after = appending.appended(
        Appended::Recorded {
            events: recorded(&batch, device()?),
            warnings: Vec::new(),
        },
        berlin("10:05")?,
    )?;

    let AfterAppend::Done(outcome) = after else {
        return Err(format!("a recorded append gave {after:?}").into());
    };
    assert_eq!(outcome.undone, begun);
    assert_eq!(outcome.before.open().count(), 1);
    assert_eq!(outcome.state.open().count(), 0);
    Ok(())
}
