//! The step machine of the tracker, driven with values only: no store, no
//! clock, fixed times and sequential IDs.

use jiff::ZonedDateTime;
use pace_core::{
    AfterAppend, Appended, Batch, ChangeMarker, Command, Decided, DeviceId, EntryId, EventKind,
    Hlc, Policy, Reading, RecordedEvent, SequentialIds,
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
fn commands_about_named_entry_or_last_batch_read_everything() -> TestResult {
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
        Command::Undo,
        Command::Forget(Forget { entry: entry()? }),
    ];

    for command in &commands {
        let range = range_of(command, &now)?;

        assert_eq!(range, TimeRange::new(None, None)?, "{command:?}");
    }
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

    let Decided { batch, read, .. } = reading.decide(ChangeMarker::new(7), Vec::new())?;

    assert_eq!(read, ChangeMarker::new(7));
    assert_eq!(batch.events.len(), 1);
    assert!(matches!(batch.events[0].kind, EventKind::EntryBegun { .. }));
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
    let Decided { appending, .. } = reading.decide(ChangeMarker::new(7), Vec::new())?;

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
        let Decided { appending, .. } = reading.decide(ChangeMarker::new(7), Vec::new())?;
        match appending.appended(Appended::Stale, now.clone())? {
            AfterAppend::ReadAgain(again) => reading = *again,
            done @ AfterAppend::Done(_) => {
                return Err(format!("a stale append gave {done:?}").into());
            }
        }
    }
    let Decided { appending, .. } = reading.decide(ChangeMarker::new(7), Vec::new())?;
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
    } = reading.decide(ChangeMarker::new(7), Vec::new())?;
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
    let Decided { batch, .. } = reading.decide(ChangeMarker::new(7), Vec::new())?;
    let begun = recorded(&batch, device()?);
    let undo = Command::Undo;
    let reading = Reading::start(&undo, berlin("10:00")?, device()?, &ids, &policy)?;
    let Decided {
        batch, appending, ..
    } = reading.decide(ChangeMarker::new(8), begun.clone())?;

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
