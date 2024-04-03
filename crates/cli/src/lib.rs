//! `pace_cli` contains utilities for the `pace` command-line interface

/// Contains the main logic for prompting the user for input
pub mod prompt;
/// Contains the main logic for the `setup` command
pub mod setup;

pub mod commands;

pub(crate) static PACE_ART: &str = include_str!("pace.art");
