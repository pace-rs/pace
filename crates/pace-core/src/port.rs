//! The ports of the core: the clock, the source of IDs, the event store and
//! the notifier.

use std::sync::atomic::{AtomicU64, Ordering};

use jiff::ZonedDateTime;
use pace_error::{PaceError, PaceResult};
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
/// The CLI uses random IDs, also with `PACE_TEST_NOW`; tests that build a
/// tracker in-process use [`SequentialIds`], one seed per simulated device.
pub trait Ids: Send + Sync {
    /// A new UUIDv7.
    fn next_id(&self) -> Uuid;
}

/// Deterministic IDs: the timestamp of a fixed moment, and where a UUIDv7
/// has random bits, a mix of that moment, a seed and a counter.
///
/// IDs differ in their last characters, like random ones, so short IDs stay
/// short; simulated devices take different seeds.
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

/// A well-mixed, deterministic 64-bit value (the `SplitMix64` finalizer).
const fn mix(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

impl Ids for SequentialIds {
    fn next_id(&self) -> Uuid {
        let count = self.counter.fetch_add(1, Ordering::Relaxed) + 1;
        let mixed = mix(self.millis ^ (u64::from(self.seed) << 48) ^ count);
        let mut bytes = [0; 10];
        bytes[..2].copy_from_slice(&self.seed.to_be_bytes());
        bytes[2..].copy_from_slice(&mixed.to_be_bytes());
        Builder::from_unix_timestamp_millis(self.millis, &bytes).into_uuid()
    }
}

/// The append-only event log: one log per device, read merged.
///
/// Backends: TOML files (default), SQLite, later Postgres. Every backend
/// holds the same events, so the same command gives the same result with
/// each. The methods block until the store is done. The SQL backends bring
/// an async trait behind a feature. Once there is more than one backend, the
/// app picks it through an enum, not a trait object; so far it uses the files
/// backend directly.
pub trait EventStore: Send + Sync {
    /// Append the events of one command to this device's log, all or none,
    /// under the store's lock, and return them as recorded. The store gives
    /// each event its [`Hlc`](crate::Hlc): later than every event of every
    /// device it knows, even if this machine's clock is behind.
    ///
    /// `read` is the [`marker`](Self::marker) taken before the read the
    /// batch was decided on. If the store changed since, another command
    /// wrote in between: the store appends nothing and returns
    /// [`Appended::Stale`], and the caller reads and decides again. The
    /// store compares the markers under the same lock as the append.
    ///
    /// # Errors
    ///
    /// If the store can't be read or written; then nothing is appended.
    fn append(&self, batch: Batch, read: ChangeMarker) -> PaceResult<Appended>;

    /// From the logs of all devices: every event of every entry whose time
    /// overlaps `range` (an open entry lasts until now, whatever the range),
    /// and the `batch.undone` events of their batches. A backend may return
    /// more events, never fewer, and in any order: the state sorts them by
    /// [`RecordedEvent::merge_key`]. The contract tests in
    /// `testing::contract` check this.
    ///
    /// # Errors
    ///
    /// If the store can't be read.
    fn read(&self, range: &TimeRange) -> PaceResult<Vec<RecordedEvent>>;

    /// The devices that have written to the log.
    ///
    /// # Errors
    ///
    /// If the store can't be read.
    fn devices(&self) -> PaceResult<Vec<Device>>;

    /// The current state of the store. Comparing it with an earlier marker
    /// says whether the data changed since.
    ///
    /// # Errors
    ///
    /// If the store can't be read.
    fn marker(&self) -> PaceResult<ChangeMarker>;
}

/// What [`EventStore::append`] did.
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
