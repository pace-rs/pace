//! The event store backends of pace, and its TOML file reader.
//!
//! - [`FilesStore`]: the `files` backend, TOML event files in `PACE_HOME`,
//!   one directory per device, one file per period ([`Split`]), read
//!   through a machine-local index.
//! - [`device_id`]: this machine's device ID, kept in the machine directory;
//!   [`existing_device_id`] reads it without creating one.
//! - [`toml_file`]: span-aware reading of pace's TOML files, so errors name
//!   the file and the line.
//! - `testing` (feature `testing`): a scratch directory for tests that write
//!   through a backend.

mod disk;
mod files;
mod format;
mod hlc;
mod index;
mod machine;
mod split;
#[cfg(feature = "testing")]
pub mod testing;
pub mod toml_file;

pub use files::{FilesStore, Resplit};
pub use machine::{DeviceIdentity, device_id, existing_device_id};
pub use split::Split;
