//! The domain of pace: its rules, its step machine and its commands. No I/O.
//!
//! The core is the hexagon of a ports-and-adapters architecture. Adapters
//! (CLI, later TUI and daemon) drive it with [`Command`] values. What the
//! core needs from the outside world comes in as values, or through the
//! [`Clock`] and [`Ids`] traits; the [`Notifier`] says where notices go.
//!
//! A command runs as a step machine without I/O: [`Reading`] names the
//! [`ReadScope`] to read, [`Reading::decide`] lets the [`Rules`] decide, and
//! [`Appending::appended`] gives the [`Outcome`] or a new read. The event
//! store and the blocking driver of the machine live in `pace_store`.
//!
//! Every timestamp is a jiff [`ZonedDateTime`](jiff::ZonedDateTime), so
//! entries keep the zone they were recorded in. All IDs are UUIDv7.
//!
//! A `pace begin` on an empty log, decided without a store; the IDs come
//! from `testing` (feature `testing`):
//!
//! ```
//! use jiff::ZonedDateTime;
//! use pace_core::{
//!     AfterRead, ChangeMarker, Command, EventKind, Policy, Reading, command::Begin,
//!     testing::SequentialIds,
//! };
//!
//! let now: ZonedDateTime = "2026-11-26T09:00[Europe/Berlin]".parse()?;
//! let ids = SequentialIds::starting_at(&now, 1);
//! let policy = Policy::default();
//! let begin = Command::Begin(Begin {
//!     note: Some("Write offer".parse()?),
//!     ..Begin::default()
//! });
//! let device = "0193a8d0-000d-7000-8000-000000000001".parse()?;
//!
//! let reading = Reading::start(&begin, now, device, &ids, &policy)?;
//! // A driver reads `reading.scope()` from its store; this log is empty.
//! let after = reading.decide(ChangeMarker::new(0), Vec::new())?;
//! let AfterRead::Decided(decided) = after else {
//!     panic!("a begin needs no more than the open entries");
//! };
//!
//! let events = &decided.batch.events;
//! let kinds: Vec<&EventKind> = events.iter().map(|event| &event.kind).collect();
//! assert!(matches!(kinds[..], [EventKind::EntryBegun { .. }]));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod command;
mod entry;
mod event;
mod id;
mod key;
mod open;
mod port;
mod rules;
mod scope;
mod short_id;
mod state;
mod steps;
#[cfg(feature = "testing")]
pub mod testing;
mod text;
mod tracking;

pub use command::Command;
pub use entry::{Entry, EntryCatch, EntryHold, EntryOrigin, EntryPause, EntryState, ProjectRef};
pub use event::{
    Batch, Change, ChangeMarker, Device, EntryChanges, Event, EventKind, Hlc, RecordedEvent,
};
pub use id::{BatchId, DeviceId, EntryId, EventId, ProjectId};
pub use key::{ProjectKey, Tag};
pub use open::EntryVerb;
pub use port::{Appended, Clock, FixedClock, Ids, Notice, Notifier};
pub use rules::{OVERLAP, Policy, Rules};
pub use scope::ReadScope;
pub use state::{Overlap, State, UndoTarget};
pub use steps::{AfterAppend, AfterRead, Appending, Decided, Outcome, Reading};
pub use text::Text;
pub use tracking::{HoldSource, PauseKind};
