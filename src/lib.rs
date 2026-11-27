//! pace: a local-first command-line time tracker for work and daily life.
//!
//! This crate is the application layer: an [Abscissa] application with the
//! commands, the configuration and the project discovery. The domain lives in
//! `pace_core`.
//!
//! [`commands::EntryPoint`] is the command line; clap reads it:
//!
//! ```
//! use clap::Parser;
//! use pace_rs::commands::EntryPoint;
//!
//! let begin = EntryPoint::try_parse_from(["pace", "begin", "Write offer", "--tag", "x"]);
//! assert!(begin.is_ok());
//!
//! let wrong = EntryPoint::try_parse_from(["pace", "begin", "--no-such-flag"]);
//! assert!(wrong.is_err());
//! ```
//!
//! [Abscissa]: https://github.com/iqlusioninc/abscissa

pub mod application;
pub mod clock;
pub mod commands;
pub mod config;
pub mod dirs;
pub mod doctor;
pub mod editor;
pub mod origin;
pub mod output;
pub mod print;
pub mod project;
pub mod prompt;
pub mod questions;
pub mod render;
pub mod select;
pub mod session;
pub mod setup;
