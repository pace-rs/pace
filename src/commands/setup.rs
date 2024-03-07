//! `setup` subcommand

use abscissa_core::{Command, Runnable};
use clap::{Parser, Subcommand};

mod completions;
mod config;
mod project;
mod show;

// TODO! Project command
// /// Setup a new pace project
// Project(project::ProjectSubCmd),

// TODO!: Explain subcommand, to show the current pace configuration and explain what each field means

/// `setup` subcommand
#[derive(Subcommand, Command, Debug, Runnable)]
pub enum SetupSubCmd {
    /// Create a new pace config and activity log
    #[clap(alias = "init", alias = "new", alias = "i", alias = "c")]
    Config(config::ConfigSubCmd),

    /// Show the current pace configuration
    Show(show::ShowSubCmd),
    /// Generate shell completions for the specified shell
    #[clap(alias = "comp")]
    Completions(completions::CompletionsCmd),
}

/// `setup` subcommand
#[derive(Command, Debug, Parser, Runnable)]
pub struct SetupCmd {
    #[clap(subcommand)]
    commands: SetupSubCmd,
}
