//! For tests of pace and of store backends: a settable clock, an in-memory
//! [`EventStore`], and the store contract every backend must keep.

use std::{
    collections::BTreeMap,
    future::Future,
    sync::{Arc, Mutex, MutexGuard},
};

use jiff::ZonedDateTime;
use pace_error::{ErrorKind, PaceError, PaceResult};

use crate::{Batch, ChangeMarker, Clock, Device, DeviceId, EventStore, Hlc, RecordedEvent};

/// A clock that tests set.
#[derive(Debug)]
pub struct TestClock(Mutex<ZonedDateTime>);

impl TestClock {
    /// A clock that shows `now`.
    #[must_use]
    pub const fn new(now: ZonedDateTime) -> Self {
        Self(Mutex::new(now))
    }

    /// Move the clock to `now`, also backwards.
    pub fn set(&self, now: ZonedDateTime) {
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = now;
    }
}

impl Clock for TestClock {
    fn now(&self) -> ZonedDateTime {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

#[derive(Debug, Default)]
struct Storage {
    events: Vec<RecordedEvent>,
    devices: BTreeMap<DeviceId, Device>,
    appends: u64,
}

/// An event store in memory. Stores made with [`MemoryStore::for_device`]
/// share one storage, like devices that sync `PACE_HOME`. `read` returns
/// everything, which keeps the contract trivially.
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
    /// [`contract::run`].
    pub fn opener() -> impl Fn(DeviceId, Arc<TestClock>) -> Self + Sync {
        let storage: Arc<Mutex<Storage>> = Arc::default();
        move |device, clock| Self {
            storage: storage.clone(),
            device,
            clock,
        }
    }

    fn storage(&self) -> PaceResult<MutexGuard<'_, Storage>> {
        self.storage.lock().map_err(|_| {
            PaceError::new(ErrorKind::Internal, "the memory store's lock is poisoned").ask_report()
        })
    }
}

impl MemoryStore {
    fn append_now(&self, batch: Batch) -> PaceResult<Vec<RecordedEvent>> {
        let now = self.clock.now();
        let mut storage = self.storage()?;
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
        Ok(recorded)
    }
}

// Everything happens in memory at once, so the futures are ready right away.
impl EventStore for MemoryStore {
    fn append(&self, batch: Batch) -> impl Future<Output = PaceResult<Vec<RecordedEvent>>> + Send {
        std::future::ready(self.append_now(batch))
    }

    fn read(
        &self,
        _range: &pace_time::TimeRange,
    ) -> impl Future<Output = PaceResult<Vec<RecordedEvent>>> + Send {
        std::future::ready(self.storage().map(|storage| storage.events.clone()))
    }

    fn devices(&self) -> impl Future<Output = PaceResult<Vec<Device>>> + Send {
        std::future::ready(
            self.storage()
                .map(|storage| storage.devices.values().cloned().collect()),
        )
    }

    fn marker(&self) -> impl Future<Output = PaceResult<ChangeMarker>> + Send {
        std::future::ready(
            self.storage()
                .map(|storage| ChangeMarker::new(storage.appends)),
        )
    }
}

/// The contract of [`EventStore`], as checks that every backend runs.
pub mod contract {
    use std::sync::Arc;

    use jiff::ZonedDateTime;
    use pace_error::{ErrorKind, PaceError, PaceResult};
    use pace_time::TimeRange;

    use super::TestClock;
    use crate::{
        Clock, Command, DeviceId, EntryState, EventStore, Policy, SequentialIds, State, Tracker,
        command::{Begin, End, Hold, Log},
    };

    /// Run every check. `new_storage` gives fresh, empty storage for each
    /// check, as a function that opens it for a device with a clock; stores
    /// opened from one storage share it, like devices syncing `PACE_HOME`.
    ///
    /// # Errors
    ///
    /// The first broken promise, naming the check.
    pub async fn run<O, S>(new_storage: impl Fn() -> O) -> PaceResult<()>
    where
        O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
        S: EventStore,
    {
        appended_events_come_back_with_their_device_and_batch(&new_storage()).await?;
        the_hlc_never_goes_back_when_the_clock_does(&new_storage()).await?;
        the_hlc_comes_after_what_other_devices_wrote(&new_storage()).await?;
        open_entries_are_read_whatever_the_range(&new_storage()).await?;
        entries_overlapping_the_range_are_read_wherever_they_are_filed(&new_storage()).await?;
        an_undo_is_read_with_its_batch(&new_storage()).await?;
        devices_are_the_ones_that_wrote(&new_storage()).await?;
        the_marker_changes_with_the_data(&new_storage()).await?;
        Ok(())
    }

    fn broken(check: &str, detail: impl std::fmt::Display) -> Box<PaceError> {
        PaceError::new(
            ErrorKind::Internal,
            "the store breaks its contract: {check}: {detail}",
        )
        .attach_context("check", check)
        .attach_context("detail", detail.to_string())
    }

    fn ensure(condition: bool, check: &str, detail: impl std::fmt::Display) -> PaceResult<()> {
        if condition {
            Ok(())
        } else {
            Err(broken(check, detail))
        }
    }

    fn at(rfc9557: &str) -> PaceResult<ZonedDateTime> {
        rfc9557
            .parse()
            .map_err(|error| PaceError::from(ErrorKind::Internal, error))
    }

    fn device(number: u16) -> PaceResult<DeviceId> {
        format!("0193a8d0-000d-7000-8000-{number:012x}").parse()
    }

    /// A device: its tracker and its clock.
    struct Device<S> {
        tracker: Tracker<S>,
        clock: Arc<TestClock>,
        id: DeviceId,
    }

    fn open<O, S>(open: &O, number: u16, now: &str) -> PaceResult<Device<S>>
    where
        O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
        S: EventStore,
    {
        let clock = Arc::new(TestClock::new(at(now)?));
        let id = device(number)?;
        let tracker = Tracker::new(
            open(id, clock.clone()),
            clock.clone(),
            Arc::new(SequentialIds::starting_at(&clock.now(), number)),
            Policy::default(),
            id,
        );
        Ok(Device { tracker, clock, id })
    }

    fn range(start: &str, end: &str) -> PaceResult<TimeRange> {
        TimeRange::new(Some(at(start)?), Some(at(end)?))
    }

    async fn appended_events_come_back_with_their_device_and_batch<O, S>(
        open_store: &O,
    ) -> PaceResult<()>
    where
        O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
        S: EventStore,
    {
        let check = "appended events come back with their device and batch";
        let laptop = open(open_store, 1, "2026-11-26T09:00[Europe/Berlin]")?;
        let outcome = laptop
            .tracker
            .run(&Command::Begin(Begin::default()))
            .await?;
        let appended = outcome
            .recorded
            .first()
            .ok_or_else(|| broken(check, "append returned nothing"))?;

        let read = laptop.tracker.state(&TimeRange::new(None, None)?).await?;
        let found = State::from_events(std::slice::from_ref(appended));
        ensure(appended.device == laptop.id, check, "wrong device")?;
        ensure(
            read.entries().count() == 1,
            check,
            "the entry was not read back",
        )?;
        ensure(
            found.entries().next() == read.entries().next(),
            check,
            "the entry changed on the way",
        )
    }

    async fn the_hlc_never_goes_back_when_the_clock_does<O, S>(open_store: &O) -> PaceResult<()>
    where
        O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
        S: EventStore,
    {
        let check = "the HLC never goes back when the clock does";
        let laptop = open(open_store, 1, "2026-11-26T10:00[Europe/Berlin]")?;
        let first = laptop
            .tracker
            .run(&Command::Begin(Begin::default()))
            .await?;
        laptop.clock.set(at("2026-11-26T09:00[Europe/Berlin]")?);
        let second = laptop
            .tracker
            .run(&Command::Hold(Hold {
                at: Some(at("2026-11-26T10:05[Europe/Berlin]")?),
                ..Hold::default()
            }))
            .await;
        // The hold lies in the future of the jumped-back clock, which the rules
        // don't check; the store must still order it after the begin.
        let second = second?;
        let (Some(first), Some(second)) = (first.recorded.first(), second.recorded.first()) else {
            return Err(broken(check, "append returned nothing"));
        };
        ensure(
            second.hlc > first.hlc,
            check,
            format!("{:?} after {:?}", second.hlc, first.hlc),
        )
    }

    async fn the_hlc_comes_after_what_other_devices_wrote<O, S>(open_store: &O) -> PaceResult<()>
    where
        O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
        S: EventStore,
    {
        let check = "the HLC comes after what other devices wrote";
        let laptop = open(open_store, 1, "2026-11-26T10:00[Europe/Berlin]")?;
        let desktop = open(open_store, 2, "2026-11-26T09:00[Europe/Berlin]")?;
        let began = laptop
            .tracker
            .run(&Command::Begin(Begin {
                at: Some(at("2026-11-26T08:00[Europe/Berlin]")?),
                ..Begin::default()
            }))
            .await?;
        let ended = desktop
            .tracker
            .run(&Command::End(End {
                at: Some(at("2026-11-26T08:30[Europe/Berlin]")?),
                ..End::default()
            }))
            .await?;
        let (Some(began), Some(ended)) = (began.recorded.first(), ended.recorded.first()) else {
            return Err(broken(check, "append returned nothing"));
        };
        ensure(
            ended.hlc > began.hlc,
            check,
            "the desktop's clock is behind, but its event must come later",
        )
    }

    async fn open_entries_are_read_whatever_the_range<O, S>(open_store: &O) -> PaceResult<()>
    where
        O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
        S: EventStore,
    {
        let check = "open entries are read whatever the range";
        let laptop = open(open_store, 1, "2026-10-15T09:00[Europe/Berlin]")?;
        _ = laptop
            .tracker
            .run(&Command::Begin(Begin::default()))
            .await?;
        laptop.clock.set(at("2026-11-26T09:00[Europe/Berlin]")?);

        let state = laptop
            .tracker
            .state(&range(
                "2026-11-26T00:00[Europe/Berlin]",
                "2026-11-27T00:00[Europe/Berlin]",
            )?)
            .await?;
        ensure(
            state.open().count() == 1,
            check,
            "an entry begun in October and still open is missing",
        )
    }

    async fn entries_overlapping_the_range_are_read_wherever_they_are_filed<O, S>(
        open_store: &O,
    ) -> PaceResult<()>
    where
        O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
        S: EventStore,
    {
        let check = "entries overlapping the range are read wherever they are filed";
        let laptop = open(open_store, 1, "2026-11-02T09:00[Europe/Berlin]")?;
        _ = laptop
            .tracker
            .run(&Command::Log(Log::new(
                at("2026-10-31T22:00[Europe/Berlin]")?,
                at("2026-11-01T02:00[Europe/Berlin]")?,
            )))
            .await?;

        let state = laptop
            .tracker
            .state(&range(
                "2026-11-01T00:00[Europe/Berlin]",
                "2026-11-01T01:00[Europe/Berlin]",
            )?)
            .await?;
        ensure(
            state.entries().count() == 1,
            check,
            "an entry from 31 October 22:00 to 1 November 02:00 is missing in 1 November",
        )
    }

    async fn an_undo_is_read_with_its_batch<O, S>(open_store: &O) -> PaceResult<()>
    where
        O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
        S: EventStore,
    {
        let check = "an undo is read with its batch";
        let laptop = open(open_store, 1, "2026-10-15T09:00[Europe/Berlin]")?;
        _ = laptop
            .tracker
            .run(&Command::Begin(Begin::default()))
            .await?;
        laptop.clock.set(at("2026-10-15T10:00[Europe/Berlin]")?);
        _ = laptop.tracker.run(&Command::End(End::default())).await?;
        laptop.clock.set(at("2026-11-26T09:00[Europe/Berlin]")?);
        _ = laptop.tracker.run(&Command::Undo).await?;

        let state = laptop
            .tracker
            .state(&range(
                "2026-10-15T00:00[Europe/Berlin]",
                "2026-10-16T00:00[Europe/Berlin]",
            )?)
            .await?;
        let entry = state
            .entries()
            .next()
            .ok_or_else(|| broken(check, "the entry is missing"))?;
        ensure(
            entry.state() == EntryState::Running,
            check,
            "the end was undone in November, but reading October still shows it",
        )
    }

    async fn devices_are_the_ones_that_wrote<O, S>(open_store: &O) -> PaceResult<()>
    where
        O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
        S: EventStore,
    {
        let check = "devices are the ones that wrote";
        let laptop = open(open_store, 1, "2026-11-26T09:00[Europe/Berlin]")?;
        let desktop = open(open_store, 2, "2026-11-26T10:00[Europe/Berlin]")?;
        _ = laptop
            .tracker
            .run(&Command::Begin(Begin::default()))
            .await?;
        _ = desktop.tracker.run(&Command::End(End::default())).await?;

        let store = open_store(laptop.id, laptop.clock.clone());
        let mut ids: Vec<DeviceId> = store
            .devices()
            .await?
            .iter()
            .map(|device| device.id)
            .collect();
        ids.sort();
        let mut expected = vec![laptop.id, desktop.id];
        expected.sort();
        ensure(ids == expected, check, format!("{ids:?}"))
    }

    async fn the_marker_changes_with_the_data<O, S>(open_store: &O) -> PaceResult<()>
    where
        O: Fn(DeviceId, Arc<TestClock>) -> S + Sync,
        S: EventStore,
    {
        let check = "the marker changes with the data";
        let laptop = open(open_store, 1, "2026-11-26T09:00[Europe/Berlin]")?;
        let store = open_store(laptop.id, laptop.clock.clone());
        let before = store.marker().await?;
        ensure(
            store.marker().await? == before,
            check,
            "it changed without a change",
        )?;
        _ = laptop
            .tracker
            .run(&Command::Begin(Begin::default()))
            .await?;
        ensure(
            store.marker().await? != before,
            check,
            "it stayed after an append",
        )
    }
}
