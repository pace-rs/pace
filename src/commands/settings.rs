//! `settings` subcommand

/// Getters for the `settings` subcommand
pub mod get;
/// Setters for the `settings` subcommand
pub mod set;

use abscissa_core::{Command, Runnable};
use clap::{Parser, Subcommand};

/// `settings` (sub-) subcommand
#[derive(Subcommand, Command, Debug, Runnable)]
pub enum SettingsSubCmd {
    /// Set values in the pace configuration
    Set(set::SetChoiceCmd),

    /// Get values from the pace configuration
    Get(get::GetChoiceCmd),
}

/// `settings` subcommand
#[derive(Command, Debug, Parser, Runnable)]
pub struct SettingsCmd {
    #[clap(subcommand)]
    commands: SettingsSubCmd,
}

// `pace settings`
