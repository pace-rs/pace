//! For tests of pace and of store backends: a scratch directory, an
//! in-memory [`EventStore`], and the store contract every backend must keep.

use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    path::Path,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicUsize, Ordering},
    },
};

use pace_core::{
    Appended, Batch, BatchId, ChangeMarker, Clock, Command, Device, DeviceId, Entry, EntryId,
    EventKind, Hlc, ReadScope, RecordedEvent, State,
    testing::{TestClock, locked},
};
use pace_error::PaceResult;
use pace_time::TimeRange;
use tempfile::TempDir;

use crate::{EventStore, Tracker};

pub mod contract;
#[cfg(test)]
mod tests;

/// A temporary directory in memory where the system has one (`/dev/shm` on
/// Linux), else in the system's temporary folder.
///
/// The `files` backend syncs each file it writes. On a disk under load one
/// sync costs tens of milliseconds, and a journey test of eight steps then
/// takes longer than codegate's slow-tests threshold of 0.5 s; in memory the
/// sync costs nothing.
///
/// # Errors
///
/// The directory cannot be created.
pub fn scratch_dir() -> io::Result<TempDir> {
    let memory = Path::new("/dev/shm");
    if memory.is_dir() {
        tempfile::tempdir_in(memory)
    } else {
        tempfile::tempdir()
    }
}

#[derive(Debug, Default)]
struct Storage {
    events: Vec<RecordedEvent>,
    devices: BTreeMap<DeviceId, Device>,
    appends: u64,
}

/// An event store in memory.
///
/// Stores made with [`MemoryStore::for_device`] share one storage, like
/// devices that sync `PACE_HOME`. `read` returns what the contract asks
/// for, plus the events of undone or forgotten entries and of entries that
/// only touch the range's ends, so a command that reads too little fails
/// its tests.
#[derive(Debug, Clone)]
pub struct MemoryStore {
    storage: Arc<Mutex<Storage>>,
    device: DeviceId,
    clock: Arc<dyn Clock>,
}

impl MemoryStore {
    /// An empty store for `device`.
    #[must_use]
    pub fn new(device: DeviceId, clock: Arc<dyn Clock>) -> Self {
        Self {
            storage: Arc::default(),
            device,
            clock,
        }
    }

    /// Another device on the same storage, with the same clock.
    #[must_use]
    pub fn for_device(&self, device: DeviceId) -> Self {
        Self {
            storage: self.storage.clone(),
            device,
            clock: self.clock.clone(),
        }
    }

    /// Fresh storage, and a way to open it for a device with a clock; for
    /// [`contract::run_store_contract`].
    pub fn opener() -> impl Fn(DeviceId, Arc<TestClock>) -> Self + Sync {
        let storage: Arc<Mutex<Storage>> = Arc::default();
        move |device, clock| Self {
            storage: storage.clone(),
            device,
            clock,
        }
    }

    fn storage(&self) -> PaceResult<MutexGuard<'_, Storage>> {
        locked(&self.storage, "memory store")
    }
}

impl EventStore for MemoryStore {
    fn append(&self, batch: Batch, read: ChangeMarker) -> PaceResult<Appended> {
        let now = self.clock.now();
        let mut storage = self.storage()?;
        if ChangeMarker::new(storage.appends) != read {
            return Ok(Appended::Stale);
        }
        let mut last = storage.events.iter().map(|recorded| recorded.hlc).max();
        let recorded: Vec<RecordedEvent> = batch
            .events
            .into_iter()
            .map(|event| {
                let hlc = Hlc::tick(last, now.timestamp());
                last = Some(hlc);
                RecordedEvent {
                    device: self.device,
                    batch: batch.id,
                    hlc,
                    event,
                }
            })
            .collect();
        storage.events.extend(recorded.iter().cloned());
        _ = storage
            .devices
            .entry(self.device)
            .or_insert_with(|| Device {
                id: self.device,
                name: "memory".to_owned(),
                created: now.clone(),
            });
        storage.appends += 1;
        drop(storage);
        Ok(Appended::Recorded {
            events: recorded,
            warnings: Vec::new(),
        })
    }

    fn read(&self, scope: &ReadScope) -> PaceResult<Vec<RecordedEvent>> {
        self.storage()
            .map(|storage| events_meeting(&storage.events, scope))
    }

    fn devices(&self) -> PaceResult<Vec<Device>> {
        self.storage()
            .map(|storage| storage.devices.values().cloned().collect())
    }

    fn marker(&self) -> PaceResult<ChangeMarker> {
        self.storage()
            .map(|storage| ChangeMarker::new(storage.appends))
    }
}

/// What [`EventStore::read`] must return for `scope`: every event of the
/// entries that meet its range, are open, or have an event recorded since
/// its moment, and the undos of their batches. An entry that is gone,
/// undone or forgotten, has no time; its events come with every read, so a
/// read of everything returns every event.
fn events_meeting(events: &[RecordedEvent], scope: &ReadScope) -> Vec<RecordedEvent> {
    let state = State::from_events(events);
    let recorded: BTreeSet<EntryId> = events
        .iter()
        .filter(|recorded| {
            scope
                .recorded_since()
                .is_some_and(|since| recorded.hlc.wall() >= since)
        })
        .filter_map(|recorded| recorded.event.kind.entry())
        .collect();
    let kept = |entry: EntryId| {
        recorded.contains(&entry)
            || state
                .entry(entry)
                .is_none_or(|found| meets(found, scope.range()))
    };
    let batches: BTreeSet<BatchId> = events
        .iter()
        .filter(|recorded| recorded.event.kind.entry().is_some_and(kept))
        .map(|recorded| recorded.batch)
        .collect();
    events
        .iter()
        .filter(|recorded| match recorded.event.kind {
            EventKind::BatchUndone { batch } => batches.contains(&batch),
            ref kind => kind.entry().is_some_and(kept),
        })
        .cloned()
        .collect()
}

/// Whether `entry` is open or its time meets `range`, ends included.
fn meets(entry: &Entry, range: &TimeRange) -> bool {
    entry.end.as_ref().is_none_or(|end| {
        range.start().is_none_or(|start| start <= end)
            && range
                .end()
                .is_none_or(|range_end| entry.start <= *range_end)
    })
}

/// A store whose appends race a rival tracker on the same storage.
///
/// Before each of its next `races` appends, the rival runs its command. So
/// one thread plays two pace commands at once: the rival writes between the
/// read and the append of the other.
#[derive(Debug)]
pub struct Raced<S, R> {
    store: S,
    rival: Tracker<R>,
    command: Command,
    races: AtomicUsize,
}

impl<S, R> Raced<S, R> {
    /// `store`, raced `races` times by `rival` running `command`.
    #[must_use]
    pub const fn new(store: S, rival: Tracker<R>, command: Command, races: usize) -> Self {
        Self {
            store,
            rival,
            command,
            races: AtomicUsize::new(races),
        }
    }
}

impl<S: EventStore, R: EventStore> EventStore for Raced<S, R> {
    fn append(&self, batch: Batch, read: ChangeMarker) -> PaceResult<Appended> {
        let raced = self
            .races
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                left.checked_sub(1)
            })
            .is_ok();
        if raced {
            _ = self.rival.run(&self.command)?;
        }
        self.store.append(batch, read)
    }

    fn read(&self, scope: &ReadScope) -> PaceResult<Vec<RecordedEvent>> {
        self.store.read(scope)
    }

    fn devices(&self) -> PaceResult<Vec<Device>> {
        self.store.devices()
    }

    fn marker(&self) -> PaceResult<ChangeMarker> {
        self.store.marker()
    }
}
