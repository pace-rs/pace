//! pace: a local-first command-line time tracker for work and daily life.
//!
//! This crate is the application layer: an [Abscissa] application with the
//! commands, the configuration and the project discovery. The domain lives in
//! `pace_core`.
//!
//! [Abscissa]: https://github.com/iqlusioninc/abscissa

pub mod application;
pub mod clock;
pub mod commands;
pub mod config;
pub mod dirs;
pub mod editor;
pub mod origin;
pub mod output;
pub mod project;
pub mod prompt;
pub mod select;
pub mod session;
