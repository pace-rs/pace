//! The command line: the entry point and its subcommands.

pub mod config;
pub mod tracking;

use std::path::PathBuf;

use abscissa_core::{Command, Configurable, Runnable};

use self::{
    config::ConfigCmd,
    tracking::{BeginCmd, CatchCmd, EndCmd, HoldCmd, PauseCmd, ResumeCmd},
};

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
        help = "The project to work on, instead of discovering it [env: PACE_PROJECT]"
    )]
    pub(crate) project: Option<String>,

    #[arg(long, global = true, help = "Plain output: no colour, no symbols")]
    pub(crate) plain: bool,
}

/// The subcommands.
#[derive(clap::Subcommand, Command, Debug, Runnable)]
pub enum PaceCmd {
    /// Begin an entry; a running entry ends at the new start
    #[command(visible_alias = "b")]
    Begin(BeginCmd),

    /// Note an interruption of the running entry
    #[command(visible_alias = "h")]
    Hold(HoldCmd),

    /// Take a planned break from the running entry
    #[command(visible_alias = "p")]
    Pause(PauseCmd),

    /// Go on after a hold or a pause
    #[command(visible_alias = "r")]
    Resume(ResumeCmd),

    /// Write a distraction down without breaking off
    #[command(visible_alias = "c")]
    Catch(CatchCmd),

    /// End the running entry
    #[command(visible_alias = "e")]
    End(EndCmd),

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
