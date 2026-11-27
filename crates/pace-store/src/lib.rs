//! The event store of pace: its port, its driver, its backends, and pace's
//! TOML file reader.
//!
//! - [`EventStore`]: the port, the trait every backend implements.
//! - [`Tracker`]: the blocking driver of `pace_core`'s tracking machine; it
//!   reads and appends through an [`EventStore`].
//! - [`FilesStore`]: the `files` backend, TOML event files in `PACE_HOME`,
//!   one directory per device, one file per period ([`Split`]), read
//!   through a machine-local index.
//! - [`device_id`]: this machine's device ID, kept in the machine directory;
//!   [`existing_device_id`] reads it without creating one.
//! - [`toml_file`]: span-aware reading of pace's TOML files, so errors name
//!   the file and the line.
//! - `testing` (feature `testing`): a scratch directory for tests that write
//!   through a backend, the in-memory `MemoryStore`, `Raced`, and the store
//!   contract every backend passes.

mod disk;
mod files;
mod format;
mod hlc;
mod index;
mod machine;
mod port;
mod split;
#[cfg(feature = "testing")]
pub mod testing;
pub mod toml_file;
mod tracker;

pub use files::{FilesStore, Resplit};
pub use machine::{DeviceIdentity, device_id, existing_device_id};
pub use port::EventStore;
pub use split::Split;
pub use tracker::Tracker;
