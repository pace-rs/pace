use std::collections::BTreeSet;

use jiff::{Timestamp, ZonedDateTime};

use crate::{BatchId, DeviceId, EntryId, EventId, HoldSource, PauseKind, ProjectId, Tag};

/// Something that happened, as the core decided it.
///
/// The store gives it its place in the log: the device, the batch and the
/// [`Hlc`] (see [`RecordedEvent`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Identity of the event.
    pub id: EventId,
    /// When it happened, for example the begin of an entry.
    pub at: ZonedDateTime,
    /// The moment whose period the event is filed under: the start of the
    /// entry for entry events, otherwise `at`. A `pace log` for October typed
    /// in November is filed under October.
    pub about: ZonedDateTime,
    /// What happened.
    pub kind: EventKind,
}

/// The event types of the tracking commands.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EventKind {
    /// `entry.begun`
    EntryBegun {
        /// The new entry.
        entry: EntryId,
        /// Its project.
        project: Option<ProjectId>,
        /// What it is about.
        note: Option<String>,
        /// Its tags.
        tags: BTreeSet<Tag>,
    },
    /// `entry.held`: an interruption began.
    EntryHeld {
        /// The interrupted entry.
        entry: EntryId,
        /// Where the interruption came from.
        source: HoldSource,
        /// Why.
        reason: Option<String>,
    },
    /// `entry.paused`: a planned break began.
    EntryPaused {
        /// The entry.
        entry: EntryId,
        /// The kind of break.
        kind: PauseKind,
        /// Why.
        reason: Option<String>,
    },
    /// `entry.resumed`: the hold or the pause ended.
    EntryResumed {
        /// The entry.
        entry: EntryId,
    },
    /// `entry.ended`
    EntryEnded {
        /// The entry.
        entry: EntryId,
    },
    /// `catch.noted`: a distraction was written down.
    CatchNoted {
        /// The entry that was running.
        entry: EntryId,
        /// The thought.
        text: String,
    },
}

/// The events of one command. They are appended together and undone
/// together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    /// Identity of the batch.
    pub id: BatchId,
    /// The events, in order.
    pub events: Vec<Event>,
}

/// An event as read back from the log of a device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedEvent {
    /// The device that wrote it.
    pub device: DeviceId,
    /// The batch it belongs to.
    pub batch: BatchId,
    /// Its place in the merged order of all devices.
    pub hlc: Hlc,
    /// The event.
    pub event: Event,
}

/// A hybrid logical clock: the wall time of recording plus a counter.
///
/// It orders the events of all devices the same way on every device, and
/// keeps the order of one device when its clock jumps back. Ties between
/// devices are broken by the device ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hlc {
    wall: Timestamp,
    counter: u32,
}

impl Hlc {
    /// A clock reading.
    #[must_use]
    pub const fn new(wall: Timestamp, counter: u32) -> Self {
        Self { wall, counter }
    }

    /// The wall time of recording.
    #[must_use]
    pub const fn wall(&self) -> Timestamp {
        self.wall
    }

    /// The counter for events with the same wall time.
    #[must_use]
    pub const fn counter(&self) -> u32 {
        self.counter
    }
}

/// A device that writes its own log, as listed by `pace device list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// Its identity.
    pub id: DeviceId,
    /// Its readable name, from the hostname at first run.
    pub name: String,
    /// When it wrote its first event.
    pub created: ZonedDateTime,
}

/// The state of a store at one point. If the marker differs from an earlier
/// one, the data changed, for example through another CLI call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChangeMarker(u64);

impl ChangeMarker {
    /// A marker from a backend-specific value, such as a hash of file
    /// modification times or SQLite's `data_version`.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}
