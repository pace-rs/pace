//! The ports of the core: the clock, the source of IDs and the notifier.

use jiff::ZonedDateTime;
use pace_error::{PaceError, PaceResult};
use uuid::Uuid;

use crate::RecordedEvent;

/// The source of "now".
///
/// The CLI uses the system clock in the configured time zone; tests and
/// journey tests use a [`FixedClock`].
pub trait Clock: std::fmt::Debug + Send + Sync {
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

/// The source of new IDs, each a UUIDv7.
///
/// The CLI uses random IDs, also with `PACE_TEST_NOW`; tests that build a
/// tracker in-process use `pace_core::testing::SequentialIds`, one seed per
/// simulated device.
pub trait Ids: std::fmt::Debug + Send + Sync {
    /// A new UUIDv7.
    fn next_id(&self) -> Uuid;
}

/// What the event store did with a batch: recorded it, or refused a stale
/// one.
#[derive(Debug)]
pub enum Appended {
    /// The batch is in the log.
    Recorded {
        /// The batch's events, as recorded.
        events: Vec<RecordedEvent>,
        /// What went wrong after the batch was recorded, such as a cleaning
        /// of forgotten entries that failed. The append succeeded, so the
        /// caller shows them as warnings.
        warnings: Vec<PaceError>,
    },
    /// Nothing: the store changed since the read the batch was decided on.
    Stale,
}

/// Something to tell the user outside the command's output, for example
/// that a Pomodoro phase ended.
///
/// No command gives notices yet: phase 7 (TUI and Pomodoro) brings live
/// alerts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// One line.
    pub summary: String,
    /// More detail, if any.
    pub body: Option<String>,
}

/// Where notices go: the terminal for the CLI, the OS for a later daemon.
///
/// The port exists from the start; its first adapter comes with phase 7
/// (TUI and Pomodoro), and the trait moves beside it then.
pub trait Notifier: std::fmt::Debug + Send + Sync {
    /// Show `notice`.
    ///
    /// # Errors
    ///
    /// If the notice could not be shown.
    fn notify(&self, notice: &Notice) -> PaceResult<()>;
}
