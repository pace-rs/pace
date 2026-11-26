//! The event store backends of pace, and its TOML file reader.
//!
//! - [`FilesStore`]: the `files` backend, TOML event files in `PACE_HOME`,
//!   one directory per device, one file per period ([`Split`]).
//! - [`toml_file`]: span-aware reading of pace's TOML files, so errors name
//!   the file and the line.

mod files;
mod format;
mod split;
pub mod toml_file;

pub use files::FilesStore;
pub use split::Split;
