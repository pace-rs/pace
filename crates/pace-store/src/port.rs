//! The event store port: the trait every backend implements.

use pace_core::{Appended, Batch, ChangeMarker, Device, ReadScope, RecordedEvent};
use pace_error::PaceResult;

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
    /// each event its [`Hlc`](pace_core::Hlc): later than every event of
    /// every device it knows, even if this machine's clock is behind.
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
    /// overlaps the scope's range (an open entry lasts until now, whatever
    /// the range) or, with [`ReadScope::recorded_since`], that has an event
    /// recorded since then, and the `batch.undone` events of their batches.
    /// A backend may return more events, never fewer, and in any order: the
    /// state sorts them by [`RecordedEvent::merge_key`]. The contract tests
    /// in `testing::contract` check this.
    ///
    /// # Errors
    ///
    /// If the store can't be read.
    fn read(&self, scope: &ReadScope) -> PaceResult<Vec<RecordedEvent>>;

    /// The devices that have written to the log.
    ///
    /// No command asks for them yet: phase 5 (remotes, push) keeps a cursor
    /// per device.
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
