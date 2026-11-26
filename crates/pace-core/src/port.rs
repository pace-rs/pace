use std::future::Future;

use jiff::ZonedDateTime;
use pace_error::PaceResult;
use pace_time::TimeRange;

use crate::{Batch, ChangeMarker, Device, RecordedEvent};

/// The source of "now".
///
/// The CLI uses the system clock in the configured time zone; tests and
/// journey tests use a [`FixedClock`].
pub trait Clock: Send + Sync {
    /// Now, in whole seconds and in the configured time zone.
    fn now(&self) -> ZonedDateTime;
}

/// A clock that always shows the same moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedClock(ZonedDateTime);

impl FixedClock {
    /// A clock that shows `now`.
    #[must_use]
    pub const fn new(now: ZonedDateTime) -> Self {
        Self(now)
    }
}

impl Clock for FixedClock {
    fn now(&self) -> ZonedDateTime {
        self.0.clone()
    }
}

/// The append-only event log: one log per device, read merged.
///
/// Backends: TOML files (default), SQLite, later Postgres. Every backend
/// holds the same events, so the same command gives the same result with
/// each. The methods are async because the SQL backends are; a backend is
/// picked through an enum, not a trait object.
pub trait EventStore: Send + Sync {
    /// Append the events of one command to this device's log, all or none,
    /// under the store's lock. The store gives each event its [`Hlc`](crate::Hlc).
    fn append(&self, batch: Batch) -> impl Future<Output = PaceResult<()>> + Send;

    /// The events filed under `range` from the logs of all devices, merged
    /// in [`Hlc`](crate::Hlc) order. A backend may return more events than
    /// the range (whole files), never fewer.
    fn read(
        &self,
        range: &TimeRange,
    ) -> impl Future<Output = PaceResult<Vec<RecordedEvent>>> + Send;

    /// The devices that have written to the log.
    fn devices(&self) -> impl Future<Output = PaceResult<Vec<Device>>> + Send;

    /// The current state of the store. Comparing it with an earlier marker
    /// says whether the data changed since.
    fn marker(&self) -> impl Future<Output = PaceResult<ChangeMarker>> + Send;
}

/// Something to tell the user outside the command's output, for example
/// that a Pomodoro phase ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// One line.
    pub summary: String,
    /// More detail, if any.
    pub body: Option<String>,
}

/// Where notices go: the terminal for the CLI, the OS for a later daemon.
pub trait Notifier: Send + Sync {
    /// Show `notice`.
    ///
    /// # Errors
    ///
    /// If the notice could not be shown.
    fn notify(&self, notice: &Notice) -> PaceResult<()>;
}
