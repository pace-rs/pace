//! The tracker: the blocking driver of the tracking machine, over the event
//! store.

use std::sync::Arc;

use jiff::ZonedDateTime;
use pace_core::{
    AfterAppend, Clock, Command, Decided, DeviceId, Ids, Outcome, Policy, Reading, State,
};
use pace_error::PaceResult;
use pace_time::TimeRange;

use crate::EventStore;

/// The blocking driver of the tracking machine: it does the I/O for the
/// step machine that starts with [`Reading`], and holds no rule.
///
/// Each command reads only what its rules need: tracking commands the open
/// entries and the time from their `--at` to now, `log` its own time, and
/// `adjust`, `forget` and `undo` everything; [`Reading::range`] says which.
pub struct Tracker<S> {
    store: S,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn Ids>,
    policy: Policy,
    device: DeviceId,
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
    /// refuses the batch, and the step machine reads and decides again, up
    /// to five times in all. So of two commands at once, the later one
    /// decides on what the earlier one wrote.
    ///
    /// # Errors
    ///
    /// If the store fails, the command doesn't fit the state, or other
    /// commands wrote in between five times; then nothing is appended.
    pub fn run(&self, command: &Command) -> PaceResult<Outcome> {
        let mut reading = Reading::start(
            command,
            self.clock.now(),
            self.device,
            &*self.ids,
            &self.policy,
        )?;
        loop {
            let marker = self.store.marker()?;
            let events = self.store.read(reading.range())?;
            let Decided {
                batch,
                read,
                appending,
            } = reading.decide(marker, events)?;
            match appending.appended(self.store.append(batch, read)?, self.clock.now())? {
                AfterAppend::Done(outcome) => return Ok(outcome),
                AfterAppend::ReadAgain(again) => reading = again,
            }
        }
    }

    /// The state of the entries that overlap `range`, and of the open ones.
    ///
    /// # Errors
    ///
    /// If the store fails.
    pub fn state(&self, range: &TimeRange) -> PaceResult<State> {
        Ok(State::from_events(&self.store.read(range)?))
    }
}
