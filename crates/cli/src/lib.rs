//! `pace_cli` contains utilities for the `pace` command-line interface

/// Contains the main logic for prompting the user for input
pub(crate) mod prompt;
/// Contains the main logic for the `setup` command
pub(crate) mod setup;

// Public API
pub use crate::{
    prompt::confirmation_or_break,
    setup::{setup_config, PathOptions},
};
