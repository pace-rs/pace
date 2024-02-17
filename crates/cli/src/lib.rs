//! `pace_cli` contains utilities for the `pace` command-line interface

/// Contains the main logic for prompting the user for input
pub(crate) mod prompt;
/// Contains the main logic for the `craft setup` command
pub(crate) mod setup;

// Public API
pub use crate::setup::craft_setup;
