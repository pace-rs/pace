use std::{
    future::Future,
    sync::atomic::{AtomicU64, Ordering},
};

use jiff::ZonedDateTime;
use pace_error::PaceResult;
use pace_time::TimeRange;
use uuid::{Builder, Uuid};

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

/// The source of new IDs, each a UUIDv7.
///
/// The CLI uses random IDs; tests and `PACE_TEST_NOW` use
/// [`SequentialIds`], so snapshots stay the same.
pub trait Ids: Send + Sync {
    /// A new UUIDv7.
    fn next_id(&self) -> Uuid;
}

/// Deterministic IDs: the timestamp of a fixed moment, then a seed and a
/// counter where a UUIDv7 has random bits.
///
/// IDs are distinct for distinct moments or seeds; simulated devices take
/// different seeds.
#[derive(Debug)]
pub struct SequentialIds {
    millis: u64,
    seed: u16,
    counter: AtomicU64,
}

impl SequentialIds {
    /// IDs with the timestamp of `moment` and the given seed.
    #[must_use]
    pub fn starting_at(moment: &ZonedDateTime, seed: u16) -> Self {
        Self {
            millis: u64::try_from(moment.timestamp().as_millisecond()).unwrap_or_default(),
            seed,
            counter: AtomicU64::new(0),
        }
    }
}

impl Ids for SequentialIds {
    fn next_id(&self) -> Uuid {
        let count = self.counter.fetch_add(1, Ordering::Relaxed) + 1;
        let mut bytes = [0; 10];
        bytes[..2].copy_from_slice(&self.seed.to_be_bytes());
        bytes[2..].copy_from_slice(&count.to_be_bytes());
        Builder::from_unix_timestamp_millis(self.millis, &bytes).into_uuid()
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
