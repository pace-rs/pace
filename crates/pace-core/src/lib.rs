//! The domain of pace: its rules, its step machine and its commands. No I/O.
//!
//! The core is the hexagon of a ports-and-adapters architecture. Adapters
//! (CLI, later TUI and daemon) drive it with [`Command`] values. What the
//! core needs from the outside world comes in as values, or through the
//! [`Clock`] and [`Ids`] traits; the [`Notifier`] says where notices go.
//!
//! A command runs as a step machine without I/O: [`Reading`] names the range
//! to read, [`Reading::decide`] lets the [`Rules`] decide, and
//! [`Appending::appended`] gives the [`Outcome`] or a new read. The event
//! store and the blocking driver of the machine live in `pace_store`.
//!
//! Every timestamp is a jiff [`ZonedDateTime`](jiff::ZonedDateTime), so
//! entries keep the zone they were recorded in. All IDs are UUIDv7.

pub mod command;
mod entry;
mod event;
mod id;
mod key;
mod port;
mod rules;
mod short_id;
mod state;
mod steps;
#[cfg(feature = "testing")]
pub mod testing;
mod text;
mod tracking;

pub use command::Command;
pub use entry::{Catch, Entry, EntryOrigin, EntryState, Hold, Pause, ProjectRef};
pub use event::{
    Batch, Change, ChangeMarker, Device, EntryChanges, Event, EventKind, Hlc, RecordedEvent,
};
pub use id::{BatchId, DeviceId, EntryId, EventId, ProjectId};
pub use key::{ProjectKey, Tag};
pub use port::{Appended, Clock, FixedClock, Ids, Notice, Notifier, SequentialIds};
pub use rules::{Policy, Rules};
pub use state::{State, UndoTarget};
pub use steps::{AfterAppend, Appending, Decided, Outcome, Reading};
pub use text::Text;
pub use tracking::{HoldSource, PauseKind};
