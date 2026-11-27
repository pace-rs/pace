//! The events of the log: what happened to an entry or a batch, as the core
//! decided it.

use std::collections::BTreeSet;

use jiff::{Timestamp, ZonedDateTime};

use crate::{
    BatchId, Billing, DeterminedBy, DeviceId, EntryId, EventId, HoldSource, PauseKind, ProjectRef,
    RoleId, Tag,
};

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

/// The event types of the tracking commands. Each variant names its type as
/// written in the event log.
///
/// The list is closed: every match on it names each variant, so a new
/// variant is a compile error wherever pace writes, reads or describes an
/// event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    /// `entry.begun`; `at` is the start.
    EntryBegun {
        /// The new entry.
        entry: EntryId,
        /// Its project.
        project: Option<ProjectRef>,
        /// What it is about.
        note: Option<String>,
        /// Its tags.
        tags: BTreeSet<Tag>,
        /// Its role, only where it differs from its project's.
        role: Option<RoleId>,
        /// Who determined its time, when marked.
        determined_by: Option<DeterminedBy>,
        /// Whether it is billable, only where it differs from its project.
        billable: Option<bool>,
        /// Whether `--allow-overlap` was given.
        allow_overlap: bool,
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
    /// `entry.logged`: an entry entered afterwards; `at` is the start.
    EntryLogged {
        /// The new entry.
        entry: EntryId,
        /// When it ended.
        end: ZonedDateTime,
        /// Its project.
        project: Option<ProjectRef>,
        /// What it was about.
        note: Option<String>,
        /// Its tags.
        tags: BTreeSet<Tag>,
        /// Its role, only where it differs from its project's.
        role: Option<RoleId>,
        /// Who determined its time, when marked.
        determined_by: Option<DeterminedBy>,
        /// Whether it is billable, only where it differs from its project.
        billable: Option<bool>,
        /// Whether `--allow-overlap` was given.
        allow_overlap: bool,
    },
    /// `entry.adjusted`: some fields of an entry changed. Per field, the
    /// later change by HLC wins.
    EntryAdjusted {
        /// The entry.
        entry: EntryId,
        /// The fields that changed.
        changes: EntryChanges,
    },
    /// `entry.forgotten`: a tombstone. Every device removes the entry's
    /// events from its own files.
    EntryForgotten {
        /// The entry.
        entry: EntryId,
    },
    /// `billing.marked`: the entry went on an invoice, with its amounts
    /// frozen.
    BillingMarked {
        /// The billed entry.
        entry: EntryId,
        /// The invoice and the frozen values.
        billing: Billing,
    },
    /// `batch.undone`: the events of a batch count as never written.
    BatchUndone {
        /// The batch.
        batch: BatchId,
    },
}

impl EventKind {
    /// The entry the event is about; an undo is about a batch.
    #[must_use]
    pub const fn entry(&self) -> Option<EntryId> {
        match self {
            Self::EntryBegun { entry, .. }
            | Self::EntryHeld { entry, .. }
            | Self::EntryPaused { entry, .. }
            | Self::EntryResumed { entry }
            | Self::EntryEnded { entry }
            | Self::CatchNoted { entry, .. }
            | Self::EntryLogged { entry, .. }
            | Self::EntryAdjusted { entry, .. }
            | Self::EntryForgotten { entry }
            | Self::BillingMarked { entry, .. } => Some(*entry),
            Self::BatchUndone { .. } => None,
        }
    }
}

/// What happens to a field in an adjustment.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Change<T> {
    /// The field stays as it is.
    #[default]
    Keep,
    /// The field gets this value.
    Set(T),
    /// The field is emptied.
    Clear,
}

impl<T> Change<T> {
    /// Whether the field stays as it is.
    #[must_use]
    pub const fn is_keep(&self) -> bool {
        matches!(self, Self::Keep)
    }

    /// The same change with the value turned by `f`.
    #[must_use]
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Change<U> {
        match self {
            Self::Keep => Change::Keep,
            Self::Set(value) => Change::Set(f(value)),
            Self::Clear => Change::Clear,
        }
    }
}

/// The fields an `entry.adjusted` event changes; the others stay.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntryChanges {
    /// A new start.
    pub start: Option<ZonedDateTime>,
    /// A new end, for an entry that has ended.
    pub end: Option<ZonedDateTime>,
    /// A new note, or none.
    pub note: Change<String>,
    /// A new project, or none.
    pub project: Change<ProjectRef>,
    /// The new set of tags.
    pub tags: Option<BTreeSet<Tag>>,
    /// A new role, or none: the project's role again.
    pub role: Change<RoleId>,
    /// Who determined the time, or unmarked.
    pub determined_by: Change<DeterminedBy>,
    /// Billable or not, or none: the project's flag again.
    pub billable: Change<bool>,
    /// Whether the entry's overlap is marked as intended from now on:
    /// `--allow-overlap` was given to an entry without the mark. Nothing
    /// takes the mark back.
    pub allow_overlap: bool,
}

impl EntryChanges {
    /// Whether nothing changes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start.is_none()
            && self.end.is_none()
            && self.note.is_keep()
            && self.project.is_keep()
            && self.tags.is_none()
            && self.role.is_keep()
            && self.determined_by.is_keep()
            && self.billable.is_keep()
            && !self.allow_overlap
    }
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

impl RecordedEvent {
    /// The key of the merged order: the HLC, ties broken by device ID and
    /// then by event ID. No two events share the key, so the order is total
    /// and the same on every device, whatever order the events come in. Two
    /// events of one device share an HLC when two machines run with a copy of
    /// one machine directory.
    #[must_use]
    pub const fn merge_key(&self) -> (Hlc, DeviceId, EventId) {
        (self.hlc, self.device, self.event.id)
    }
}

/// A hybrid logical clock: the wall time of recording plus a counter.
///
/// It orders the events of all devices the same way on every device, and
/// keeps the order of one device when its clock jumps back. Ties are broken
/// by [`RecordedEvent::merge_key`].
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

    /// The reading for the next event: later than `last`, the latest reading
    /// of all devices' logs, and not before the wall clock `now`, in
    /// milliseconds. If the clock has jumped back, the counter goes on.
    #[must_use]
    pub fn tick(last: Option<Self>, now: Timestamp) -> Self {
        let wall = Timestamp::from_millisecond(now.as_millisecond()).unwrap_or(now);
        let Some(last) = last.filter(|last| last.wall >= wall) else {
            return Self { wall, counter: 0 };
        };
        last.counter.checked_add(1).map_or_else(
            || Self {
                wall: last
                    .wall
                    .checked_add(jiff::SignedDuration::from_millis(1))
                    .unwrap_or(last.wall),
                counter: 0,
            },
            |counter| Self {
                wall: last.wall,
                counter,
            },
        )
    }
}

/// A device that writes its own log, as the event store lists it.
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
