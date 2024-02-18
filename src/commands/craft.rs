//! `craft` subcommand

use abscissa_core::{Command, Runnable};
use clap::{Parser, Subcommand};

mod completions;
mod project;
mod setup;
mod show;

/// `craft` subcommand
#[derive(Subcommand, Command, Debug, Runnable)]
pub enum CraftSubCmd {
    /// Craft a new pace setup
    Setup(setup::SetupSubCmd),
    // TODO! Show command
    // /// Show the current pace configuration
    // Show(show::ShowSubCmd),
    // /// Generate shell completions for the specified shell
    // TODO! Project command
    // /// Craft a new pace project
    // Project(project::ProjectSubCmd),
    /// Generate shell completions for the specified shell
    Completions(completions::CompletionsCmd),
}

/// `craft` subcommand
#[derive(Command, Debug, Parser, Runnable)]
pub struct CraftCmd {
    #[clap(subcommand)]
    commands: CraftSubCmd,
}
