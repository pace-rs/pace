//! The event store of pace: its port, its driver, its backends, the reader
//! of the catalog, and pace's TOML file reader and writer.
//!
//! - [`EventStore`]: the port, the trait every backend implements.
//! - [`Tracker`]: the blocking driver of `pace_core`'s tracking machine; it
//!   reads and appends through an [`EventStore`].
//! - [`FilesStore`]: the `files` backend, TOML event files in `PACE_HOME`,
//!   one directory per device, one file per period ([`Split`]), read
//!   through a machine-local index.
//! - [`device_id`]: this machine's device ID, kept in the machine directory;
//!   [`existing_device_id`] reads it without creating one.
//! - [`catalog`]: [`read_catalog`](catalog::read_catalog) reads the roles,
//!   the clients and the project records of `PACE_HOME` into the `Catalog`.
//! - The checks of `pace doctor`, which name every mistake where a command
//!   stops at the first: [`event_file_paths`] lists the event files,
//!   [`parse_events_collecting`] reads one, [`detect_cloned_device`] and
//!   [`last_written_hlc`] give the clone check without an append, and
//!   [`check_catalog_files`](catalog::check_catalog_files) reads the
//!   catalog's files.
//! - [`toml_file`]: span-aware reading of pace's TOML files, so errors name
//!   the file and the line.
//! - [`field_edit`]: the field writer of pace's TOML files, which keeps
//!   their comments and order.
//! - `testing` (feature `testing`): a scratch directory for tests that write
//!   through a backend, the in-memory `MemoryStore`, `Raced`, and the store
//!   contract every backend passes.
//!
//! A tracker over the `files` backend runs `pace begin`; the scratch
//! directory and the IDs come from the `testing` features:
//!
//! ```
//! use std::sync::Arc;
//!
//! use jiff::ZonedDateTime;
//! use pace_core::{
//!     Catalog, Command, FixedClock, Policy, command::Begin, testing::SequentialIds,
//! };
//! use pace_store::{FilesStore, Split, Tracker, testing::scratch_dir};
//!
//! let dir = scratch_dir()?;
//! let now: ZonedDateTime = "2026-11-26T09:00[Europe/Berlin]".parse()?;
//! let ids = Arc::new(SequentialIds::starting_at(&now, 1));
//! let clock = Arc::new(FixedClock::new(now));
//! let device = "0193a8d0-000d-7000-8000-000000000001".parse()?;
//! let home = dir.path().join("home");
//! let machine = dir.path().join("machine");
//! let name = "laptop".to_owned();
//! let store = FilesStore::new(home, machine, device, name, clock.clone(), Split::Month);
//! let tracker = Tracker::new(store, clock, ids, Policy::default(), Catalog::default(), device);
//!
//! let begin = Begin {
//!     note: Some("Write offer".parse()?),
//!     ..Begin::default()
//! };
//! let outcome = tracker.run(&Command::Begin(begin))?;
//!
//! assert_eq!(outcome.state.open().count(), 1);
//! assert!(dir.path().join("home/events").join(device.to_string()).is_dir());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod catalog;
mod disk;
pub mod field_edit;
mod files;
mod format;
mod hlc;
mod index;
mod machine;
mod port;
mod renames;
mod split;
#[cfg(feature = "testing")]
pub mod testing;
pub mod toml_file;
mod tracker;

pub use disk::write_atomically;
pub use files::{FilesStore, Resplit, event_file_paths};
pub use format::{EventFileRead, parse_events_collecting};
pub use machine::{
    DeviceIdentity, detect_cloned_device, device_id, existing_device_id, last_written_hlc,
};
pub use port::EventStore;
pub use split::Split;
pub use tracker::Tracker;
