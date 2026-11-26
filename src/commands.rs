//! The command line: the entry point and its subcommands.

pub mod config;

use std::path::PathBuf;

use abscissa_core::{Command, Configurable, Runnable};

use self::config::ConfigCmd;

/// pace: a local-first command-line time tracker for work and daily life.
#[derive(clap::Parser, Command, Debug)]
#[command(name = "pace", version, about, arg_required_else_help = true)]
pub struct EntryPoint {
    #[command(subcommand)]
    cmd: PaceCmd,

    #[arg(
        long,
        global = true,
        value_name = "FILE",
        help = "Read the global configuration from FILE [env: PACE_CONFIG]"
    )]
    pub(crate) config: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        value_name = "DIR",
        help = "Keep pace's data in DIR [env: PACE_HOME]"
    )]
    pub(crate) home: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        value_name = "KEY",
        help = "Work on the project KEY instead of the one of the working directory [env: PACE_PROJECT]"
    )]
    pub(crate) project: Option<String>,

    #[arg(long, global = true, help = "Plain output: no colour, no symbols")]
    pub(crate) plain: bool,
}

/// The subcommands.
#[derive(clap::Subcommand, Command, Debug, Runnable)]
pub enum PaceCmd {
    /// Look at pace's configuration.
    #[command(subcommand)]
    Config(ConfigCmd),
}

impl Runnable for EntryPoint {
    fn run(&self) {
        self.cmd.run();
    }
}

impl Configurable<()> for EntryPoint {}
