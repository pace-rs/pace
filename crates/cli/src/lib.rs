//! `pace_cli` contains utilities for the `pace` command-line interface

/// Contains the main logic for prompting the user for input
pub(crate) mod prompt;
/// Contains the main logic for the `setup` command
pub(crate) mod setup;

pub(crate) static PACE_ART: &str = include_str!("pace.art");

// Public API
pub use crate::{
    prompt::{confirmation_or_break, prompt_resume_activity, prompt_time_zone},
    setup::{setup_config, PathOptions},
};
