//! The step machine of a command: what to read, the batch to append, and
//! what the append means, without I/O.
//!
//! The machine decides, and its caller does the I/O. [`Reading::start`]
//! takes a command and the time, and names the scope to read. With the
//! store's marker and the events of that scope, [`Reading::decide`] lets the
//! [`Rules`] decide and gives the batch to append; if the read was too
//! narrow for the rules, it gives a reading of everything instead. A
//! command about an entry it names reads the last month first, and `undo`
//! what was recorded in it: the likely targets, and much less to read than
//! all of the history. With what the store did,
//! [`Appending::appended`] gives the [`Outcome`] or, after a stale read, a
//! new [`Reading`] at the new time; after five stale reads, it fails.
//!
//! The machine reads no clock and calls no store, so a test drives it with
//! values, a race included, and each driver picks how a command stays
//! atomic; `pace_store` holds the blocking driver. Each step consumes the
//! state before it: a batch exists only after a decided read, and an
//! outcome only after its append.

use std::fmt;

use jiff::{ToSpan, ZonedDateTime};
use pace_error::{ErrorKind, PaceError, PaceResult};
use pace_time::TimeRange;

use crate::{
    Appended, Batch, ChangeMarker, Command, DeviceId, Entry, EntryId, EventKind, Ids, Policy,
    ReadScope, RecordedEvent, Rules, State,
    command::{Adjust, End, Forget, Resume},
};

/// How often a command reads and decides before it gives up on commands
/// that write at the same time.
const ATTEMPTS: usize = 5;

/// How far back, in days, a command about an entry it names, or `undo`,
/// reads before it reads everything.
const LOOK_BACK_DAYS: i64 = 31;

/// What a command did.
#[derive(Debug)]
pub struct Outcome {
    /// The events it appended, as recorded.
    pub recorded: Vec<RecordedEvent>,
    /// What went wrong after the events were recorded, for the caller to
    /// show as warnings; see [`Appended::Recorded`].
    pub warnings: Vec<PaceError>,
    /// The state before the command, as far as it read it.
    pub before: State,
    /// The state afterwards, as far as the command read it.
    pub state: State,
    /// The events of the batches the command undid, if it was an undo.
    pub undone: Vec<RecordedEvent>,
    /// When the command ran.
    pub now: ZonedDateTime,
}

/// The command and what its rules need, through all its attempts.
struct CommandRun<'a> {
    command: &'a Command,
    device: DeviceId,
    ids: &'a dyn Ids,
    policy: &'a Policy,
    /// The attempt under way, from 1 to [`ATTEMPTS`].
    attempt: usize,
}

impl fmt::Debug for CommandRun<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommandRun")
            .field("command", self.command)
            .field("device", &self.device)
            .field("policy", self.policy)
            .field("attempt", &self.attempt)
            .finish_non_exhaustive()
    }
}

/// A command that waits for the events of its [`scope`](Self::scope).
#[derive(Debug)]
pub struct Reading<'a> {
    run: CommandRun<'a>,
    now: ZonedDateTime,
    scope: ReadScope,
}

/// A decided read: the batch to append, and what the append means.
#[derive(Debug)]
pub struct Decided<'a> {
    /// The events of the command.
    pub batch: Batch,
    /// The marker of the read the batch was decided on, for the store's
    /// append.
    pub read: ChangeMarker,
    /// The step that takes what the append did.
    pub appending: Appending<'a>,
}

/// A batch on its way into the store.
#[derive(Debug)]
pub struct Appending<'a> {
    run: CommandRun<'a>,
    now: ZonedDateTime,
    events: Vec<RecordedEvent>,
    before: State,
}

/// What comes after a read.
#[derive(Debug)]
pub enum AfterRead<'a> {
    /// The rules decided on the read; boxed, since it holds the state.
    Decided(Box<Decided<'a>>),
    /// The read was too narrow for the rules: read everything, and decide
    /// on that.
    ReadEverything(Box<Reading<'a>>),
}

/// What comes after an append.
#[derive(Debug)]
pub enum AfterAppend<'a> {
    /// The batch is in the store; boxed, since an outcome holds two states.
    Done(Box<Outcome>),
    /// Another command wrote since the read: read and decide again; boxed,
    /// since a reading holds its scope.
    ReadAgain(Box<Reading<'a>>),
}

impl<'a> Reading<'a> {
    /// The first read of `command` at `now`, on `device`, with new IDs from
    /// `ids` and the settings in `policy`.
    ///
    /// # Errors
    ///
    /// If the command's time makes no range, which [`TimeRange::new`]
    /// refuses.
    pub fn start(
        command: &'a Command,
        now: ZonedDateTime,
        device: DeviceId,
        ids: &'a dyn Ids,
        policy: &'a Policy,
    ) -> PaceResult<Self> {
        Self::at(
            CommandRun {
                command,
                device,
                ids,
                policy,
                attempt: 1,
            },
            now,
        )
    }

    fn at(run: CommandRun<'a>, now: ZonedDateTime) -> PaceResult<Self> {
        let scope = scope(run.command, &now)?;
        Ok(Self { run, now, scope })
    }

    /// What the command's rules need to see: read the events of this
    /// scope from the store.
    #[must_use]
    pub const fn scope(&self) -> &ReadScope {
        &self.scope
    }

    /// Decide the command on the `events` of the [`scope`](Self::scope),
    /// read after the store gave `marker`; or, if they lack what the rules
    /// need, read everything.
    ///
    /// # Errors
    ///
    /// If the command doesn't fit the state; see [`Rules::decide`].
    pub fn decide(
        self,
        marker: ChangeMarker,
        events: Vec<RecordedEvent>,
    ) -> PaceResult<AfterRead<'a>> {
        let before = State::from_events(&events);
        if !self.scope.is_everything()
            && !suffices(self.run.command, &before, &self.scope, self.run.device)
        {
            return Ok(AfterRead::ReadEverything(Box::new(Self {
                scope: ReadScope::everything(),
                ..self
            })));
        }
        let batch = Rules {
            state: &before,
            now: &self.now,
            device: self.run.device,
            ids: self.run.ids,
            policy: self.run.policy,
        }
        .decide(self.run.command)?;
        Ok(AfterRead::Decided(Box::new(Decided {
            batch,
            read: marker,
            appending: Appending {
                run: self.run,
                now: self.now,
                events,
                before,
            },
        })))
    }
}

impl<'a> Appending<'a> {
    /// What the store's `appended` means, with `now` for the next read.
    ///
    /// # Errors
    ///
    /// If the append was stale for the fifth time: other commands wrote
    /// between each read and append.
    pub fn appended(self, appended: Appended, now: ZonedDateTime) -> PaceResult<AfterAppend<'a>> {
        match appended {
            Appended::Recorded {
                events: recorded,
                warnings,
            } => Ok(AfterAppend::Done(Box::new(Outcome {
                warnings,
                ..outcome(&self.events, self.before, recorded, self.now)
            }))),
            Appended::Stale if self.run.attempt < ATTEMPTS => Reading::at(
                CommandRun {
                    attempt: self.run.attempt + 1,
                    ..self.run
                },
                now,
            )
            .map(|again| AfterAppend::ReadAgain(Box::new(again))),
            Appended::Stale => Err(concurrent_writes()),
        }
    }
}

/// What a command did, from the events it read and the ones it appended.
fn outcome(
    events: &[RecordedEvent],
    before: State,
    recorded: Vec<RecordedEvent>,
    now: ZonedDateTime,
) -> Outcome {
    let undone_batches: Vec<_> = recorded
        .iter()
        .filter_map(|recorded| match recorded.event.kind {
            EventKind::BatchUndone { batch } => Some(batch),
            _ => None,
        })
        .collect();
    let undone = events
        .iter()
        .filter(|event| undone_batches.contains(&event.batch))
        .cloned()
        .collect();
    let state = before.clone().followed_by(&recorded, events);
    Outcome {
        recorded,
        warnings: Vec::new(),
        before,
        state,
        undone,
        now,
    }
}

/// The error when other commands wrote between each read and append.
fn concurrent_writes() -> Box<PaceError> {
    PaceError::new(
        ErrorKind::InputOutput,
        "other pace commands wrote to the event log {attempts} times while this one decided",
    )
    .attach_context("attempts", ATTEMPTS.to_string())
    .attach_hint("run the command again once the other pace commands are done")
    .attach_error_code("pace::core::concurrent-write")
}

/// What a command's rules need to see, or the part of it that holds their
/// likely target; [`suffices`] says whether it was enough.
fn scope(command: &Command, now: &ZonedDateTime) -> PaceResult<ReadScope> {
    let now_to_now = || TimeRange::new(Some(now.clone()), Some(now.clone()));
    let look_back = now.checked_sub(LOOK_BACK_DAYS.days()).ok();
    Ok(match command {
        Command::Begin(begin) => {
            let start = begin.at.as_ref().unwrap_or(now).min(now).clone();
            TimeRange::new(Some(start), Some(now.clone()))?.into()
        }
        // The entry may have ended long ago, or adjust may move it there.
        Command::Resume(Resume { entry: Some(_), .. })
        | Command::End(End { entry: Some(_), .. })
        | Command::Forget(_) => TimeRange::new(look_back, None)?.into(),
        Command::Adjust(adjust) => {
            let start = look_back.map(|back| {
                [&adjust.start, &adjust.end]
                    .into_iter()
                    .flatten()
                    .fold(back, |earliest, moment| earliest.min(moment.clone()))
            });
            TimeRange::new(start, None)?.into()
        }
        // The last batch may be about any time, so the read takes what was
        // recorded lately.
        Command::Undo => match look_back {
            Some(back) => ReadScope::of(now_to_now()?).with_recorded_since(back.timestamp()),
            None => ReadScope::everything(),
        },
        Command::Hold(_)
        | Command::Pause(_)
        | Command::Resume(_)
        | Command::Catch(_)
        | Command::End(_) => now_to_now()?.into(),
        // The rules refuse a log that ends before it starts, with a better
        // message than the range would.
        Command::Log(log) => TimeRange::new(
            Some((&log.start).min(&log.end).clone()),
            Some((&log.start).max(&log.end).clone()),
        )?
        .into(),
    })
}

/// Whether the `state` read for `scope` holds all that the rules of
/// `command` look at, so they decide as on everything.
///
/// A read has every event of the entries that overlap its range and of
/// those recorded since its moment. So a named entry must overlap the range,
/// an adjust's checks for overlaps must lie inside it, and the batch `undo`
/// takes back must be recorded since the moment: then every later batch is in
/// the read too.
fn suffices(command: &Command, state: &State, scope: &ReadScope, device: DeviceId) -> bool {
    let from = scope.range().start();
    let meets =
        |entry: &Entry| from.is_none_or(|from| entry.end.as_ref().is_none_or(|end| end > from));
    let named = |id: EntryId| state.entry(id).is_some_and(meets);
    match command {
        Command::Resume(Resume {
            entry: Some(id), ..
        })
        | Command::End(End {
            entry: Some(id), ..
        })
        | Command::Forget(Forget { entry: id }) => named(*id),
        Command::Adjust(adjust) => match (adjust.entry, adjusted_entry(state, adjust)) {
            (_, Some(entry)) => {
                let checked_from = adjust.start.as_ref().unwrap_or(&entry.start);
                meets(entry) && from.is_none_or(|from| checked_from >= from)
            }
            (Some(_), None) => false,
            // No open entry, or several: every read has all open entries.
            (None, None) => true,
        },
        Command::Undo => state.last_undoable(device).is_some_and(|target| {
            scope
                .recorded_since()
                .is_none_or(|since| target.recorded.wall() >= since)
        }),
        Command::Begin(_)
        | Command::Hold(_)
        | Command::Pause(_)
        | Command::Resume(_)
        | Command::Catch(_)
        | Command::End(_)
        | Command::Log(_) => true,
    }
}

/// The entry `adjust` names, or the only open one.
fn adjusted_entry<'s>(state: &'s State, adjust: &Adjust) -> Option<&'s Entry> {
    adjust.entry.map_or_else(
        || {
            let mut open = state.open();
            open.next().filter(|_| open.next().is_none())
        },
        |id| state.entry(id),
    )
}
