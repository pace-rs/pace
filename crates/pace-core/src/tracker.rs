use std::sync::Arc;

use jiff::ZonedDateTime;
use pace_error::PaceResult;
use pace_time::TimeRange;

use crate::{
    Clock, Command, DeviceId, EventKind, EventStore, Ids, Policy, RecordedEvent, Rules, State,
};

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
    /// # Errors
    ///
    /// If the store fails, or the command doesn't fit the state; then
    /// nothing is appended.
    pub async fn run(&self, command: &Command) -> PaceResult<Outcome> {
        let now = self.clock.now();
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
        let recorded = self.store.append(batch).await?;

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
        Ok(Outcome {
            recorded,
            before: state,
            state: State::from_events(&all),
            undone,
            now,
        })
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
