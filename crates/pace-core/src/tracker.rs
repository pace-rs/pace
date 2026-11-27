//! The tracker: runs a command over the ports and says what it did.

use std::sync::Arc;

use jiff::ZonedDateTime;
use pace_error::{ErrorKind, PaceError, PaceResult};
use pace_time::TimeRange;

use crate::{
    Appended, Clock, Command, DeviceId, EventKind, EventStore, Ids, Policy, RecordedEvent, Rules,
    State,
};

/// How often [`Tracker::run`] reads and decides before it gives up on
/// commands that write at the same time.
const ATTEMPTS: usize = 5;

/// The use cases over the ports: read the state, decide, append.
///
/// Each command reads only what its rules need: tracking commands the open
/// entries and the time from their `--at` to now, `log` its own time, and
/// `adjust`, `forget` and `undo` everything.
pub struct Tracker<S> {
    store: S,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn Ids>,
    policy: Policy,
    device: DeviceId,
}

/// What a command did.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// The events it appended, as recorded.
    pub recorded: Vec<RecordedEvent>,
    /// The state before the command, as far as it read it.
    pub before: State,
    /// The state afterwards, as far as the command read it.
    pub state: State,
    /// The events of the batches the command undid, if it was an undo.
    pub undone: Vec<RecordedEvent>,
    /// When the command ran.
    pub now: ZonedDateTime,
}

impl<S: EventStore> Tracker<S> {
    /// A tracker for `device`.
    pub fn new(
        store: S,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn Ids>,
        policy: Policy,
        device: DeviceId,
    ) -> Self {
        Self {
            store,
            clock,
            ids,
            policy,
            device,
        }
    }

    /// The store the tracker works on, for what only its backend does.
    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }

    /// Now, by the tracker's clock.
    #[must_use]
    pub fn now(&self) -> ZonedDateTime {
        self.clock.now()
    }

    /// Decide `command` and append its events.
    ///
    /// If another command wrote between the read and the append, the store
    /// refuses the batch, and the tracker reads and decides again, up to
    /// five times in all. So of two commands at once, the later one
    /// decides on what the earlier one wrote.
    ///
    /// # Errors
    ///
    /// If the store fails, the command doesn't fit the state, or other
    /// commands wrote in between five times; then nothing is appended.
    pub async fn run(&self, command: &Command) -> PaceResult<Outcome> {
        for _ in 0..ATTEMPTS {
            let now = self.clock.now();
            let marker = self.store.marker().await?;
            let events = self.store.read(&scope(command, &now)?).await?;
            let state = State::from_events(&events);
            let batch = Rules {
                state: &state,
                now: &now,
                device: self.device,
                ids: &*self.ids,
                policy: &self.policy,
            }
            .decide(command)?;
            if let Appended::Recorded(recorded) = self.store.append(batch, marker).await? {
                return Ok(outcome(events, state, recorded, now));
            }
        }
        Err(concurrent_writes())
    }

    /// The state of the entries that overlap `range`, and of the open ones.
    ///
    /// # Errors
    ///
    /// If the store fails.
    pub async fn state(&self, range: &TimeRange) -> PaceResult<State> {
        Ok(State::from_events(&self.store.read(range).await?))
    }
}

/// What a command did, from the events it read and the ones it appended.
fn outcome(
    events: Vec<RecordedEvent>,
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
    let mut all = events;
    all.extend(recorded.iter().cloned());
    Outcome {
        recorded,
        before,
        state: State::from_events(&all),
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

/// The time a command's rules need to see.
fn scope(command: &Command, now: &ZonedDateTime) -> PaceResult<TimeRange> {
    match command {
        Command::Begin(begin) => {
            let start = begin.at.as_ref().unwrap_or(now).min(now).clone();
            TimeRange::new(Some(start), Some(now.clone()))
        }
        Command::Hold(_)
        | Command::Pause(_)
        | Command::Resume(_)
        | Command::Catch(_)
        | Command::End(_) => TimeRange::new(Some(now.clone()), Some(now.clone())),
        // The rules refuse a log that ends before it starts, with a better
        // message than the range would.
        Command::Log(log) => TimeRange::new(
            Some((&log.start).min(&log.end).clone()),
            Some((&log.start).max(&log.end).clone()),
        ),
        Command::Adjust(_) | Command::Undo | Command::Forget(_) => TimeRange::new(None, None),
    }
}
