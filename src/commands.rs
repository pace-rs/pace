//! The command line: the entry point and its subcommands.

use abscissa_core::{Command, Configurable, Runnable};

/// pace: a local-first command-line time tracker for work and daily life.
#[derive(clap::Parser, Command, Debug)]
#[command(name = "pace", version, about, arg_required_else_help = true)]
pub struct EntryPoint {}

impl Runnable for EntryPoint {
    fn run(&self) {}
}

impl Configurable<()> for EntryPoint {}
