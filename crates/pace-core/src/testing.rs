//! For tests of pace and of store backends: a settable clock, an in-memory
//! [`EventStore`], and the store contract every backend must keep.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicUsize, Ordering},
    },
};

use jiff::ZonedDateTime;
use pace_error::{ErrorKind, PaceError, PaceResult};

use crate::{
    Appended, Batch, BatchId, ChangeMarker, Clock, Command, Device, DeviceId, Entry, EntryId,
    EventKind, EventStore, Hlc, RecordedEvent, State, Tracker,
};

pub mod contract;
#[cfg(test)]
mod tests;

/// A clock that tests set.
///
/// A clock whose lock a panicked thread held panics when it is read or
/// set, with the error that [`MemoryStore`] returns for its own lock:
/// [`Clock::now`] cannot return an error.
#[derive(Debug)]
pub struct TestClock(Mutex<ZonedDateTime>);

impl TestClock {
    /// A clock that shows `now`.
    #[must_use]
    pub const fn new(now: ZonedDateTime) -> Self {
        Self(Mutex::new(now))
    }

    /// Move the clock to `now`, also backwards.
    ///
    /// # Panics
    ///
    /// If a thread panicked while it held the clock's lock.
    pub fn set(&self, now: ZonedDateTime) {
        *self.time() = now;
    }

    fn time(&self) -> MutexGuard<'_, ZonedDateTime> {
        match locked(&self.0, "test clock") {
            Ok(time) => time,
            Err(error) => panic!("{error}"),
        }
    }
}

impl Clock for TestClock {
    fn now(&self) -> ZonedDateTime {
        self.time().clone()
    }
}

/// The value behind `lock`, or an error that names the `what` it guards.
///
/// A thread that panicked while it held the lock may have left the value
/// half-changed, so the test doubles never read it.
fn locked<'lock, T>(lock: &'lock Mutex<T>, what: &str) -> PaceResult<MutexGuard<'lock, T>> {
    lock.lock().map_err(|_| {
        PaceError::new(ErrorKind::Internal, "the {what}'s lock is poisoned")
            .attach_context("what", what)
            .ask_report()
    })
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

impl std::fmt::Debug for dyn Clock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Clock({})", self.now())
    }
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

    fn read(&self, range: &pace_time::TimeRange) -> PaceResult<Vec<RecordedEvent>> {
        self.storage()
            .map(|storage| events_meeting(&storage.events, range))
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

/// What [`EventStore::read`] must return for `range`: every event of the
/// entries that meet it or are open, and the undos of their batches. An
/// entry that is gone, undone or forgotten, has no time; its events come
/// with every read, so a read of everything returns every event.
fn events_meeting(events: &[RecordedEvent], range: &pace_time::TimeRange) -> Vec<RecordedEvent> {
    let state = State::from_events(events);
    let kept = |entry: EntryId| state.entry(entry).is_none_or(|entry| meets(entry, range));
    let batches: BTreeSet<BatchId> = events
        .iter()
        .filter(|recorded| entry_of(&recorded.event.kind).is_some_and(kept))
        .map(|recorded| recorded.batch)
        .collect();
    events
        .iter()
        .filter(|recorded| match recorded.event.kind {
            EventKind::BatchUndone { batch } => batches.contains(&batch),
            ref kind => entry_of(kind).is_some_and(kept),
        })
        .cloned()
        .collect()
}

/// Whether `entry` is open or its time meets `range`, ends included.
fn meets(entry: &Entry, range: &pace_time::TimeRange) -> bool {
    entry.end.as_ref().is_none_or(|end| {
        range.start().is_none_or(|start| start <= end)
            && range
                .end()
                .is_none_or(|range_end| entry.start <= *range_end)
    })
}

/// The entry an event is about; `None` for an undo, which is about a batch.
const fn entry_of(kind: &EventKind) -> Option<EntryId> {
    match kind {
        EventKind::EntryBegun { entry, .. }
        | EventKind::EntryHeld { entry, .. }
        | EventKind::EntryPaused { entry, .. }
        | EventKind::EntryResumed { entry }
        | EventKind::EntryEnded { entry }
        | EventKind::CatchNoted { entry, .. }
        | EventKind::EntryLogged { entry, .. }
        | EventKind::EntryAdjusted { entry, .. }
        | EventKind::EntryForgotten { entry } => Some(*entry),
        EventKind::BatchUndone { .. } => None,
    }
}

/// A store whose appends race a rival tracker on the same storage.
///
/// Before each of its next `races` appends, the rival runs its command. So
/// one thread plays two pace commands at once: the rival writes between the
/// read and the append of the other.
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

    fn read(&self, range: &pace_time::TimeRange) -> PaceResult<Vec<RecordedEvent>> {
        self.store.read(range)
    }

    fn devices(&self) -> PaceResult<Vec<Device>> {
        self.store.devices()
    }

    fn marker(&self) -> PaceResult<ChangeMarker> {
        self.store.marker()
    }
}
