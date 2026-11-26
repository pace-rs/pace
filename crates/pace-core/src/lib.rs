//! The domain of pace: its rules, ports and commands. No I/O.
//!
//! The core is the hexagon of a ports-and-adapters architecture. Adapters
//! (CLI, later TUI and daemon) drive it with [`Command`] values. Everything
//! the core needs from the outside world goes through a port: the
//! [`Clock`], the [`EventStore`] and the [`Notifier`].
//!
//! Every timestamp is a jiff [`ZonedDateTime`](jiff::ZonedDateTime), so
//! entries keep the zone they were recorded in. All IDs are UUIDv7.

pub mod command;
mod entry;
mod event;
mod id;
mod key;
mod port;
mod state;
mod tracking;

pub use command::Command;
pub use entry::{Catch, Entry, EntryOrigin, EntryState, Hold, Pause, ProjectRef};
pub use event::{
    Batch, Change, ChangeMarker, Device, EntryChanges, Event, EventKind, Hlc, RecordedEvent,
};
pub use id::{BatchId, DeviceId, EntryId, EventId, ProjectId};
pub use key::{ProjectKey, Tag};
pub use port::{Clock, EventStore, FixedClock, Notice, Notifier};
pub use state::State;
pub use tracking::{HoldSource, PauseKind};
