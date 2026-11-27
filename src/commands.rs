//! The command line: the entry point and its subcommands.

pub mod client;
pub mod completions;
pub mod config;
pub mod corrections;
pub mod edit;
pub mod now;
pub mod project;
pub mod role;
pub mod store;
pub mod tracking;

use std::path::PathBuf;

use abscissa_core::{Command, Configurable, Runnable};

use self::{
    client::ClientCmd,
    completions::CompletionsCmd,
    config::ConfigCmd,
    corrections::{AdjustCmd, ForgetCmd, LogCmd, UndoCmd},
    edit::EditCmd,
    now::{NowCmd, NowFormat},
    project::ProjectCmd,
    role::RoleCmd,
    store::StoreCmd,
    tracking::{BeginCmd, CatchCmd, EndCmd, HoldCmd, PauseCmd, ResumeCmd},
};
use crate::application::exit_on_error;
use crate::print::print_lines;

/// pace: a local-first command-line time tracker for work and daily life.
#[derive(clap::Parser, Command, Debug)]
#[command(name = "pace", version, about)]
pub struct EntryPoint {
    #[command(subcommand)]
    cmd: Option<PaceCmd>,

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

    /// Show what is running, held or paused
    #[command(visible_alias = "n")]
    Now(NowCmd),

    /// Change an entry: its start, end, note or tags; `--project` moves it
    #[command(visible_alias = "a")]
    Adjust(AdjustCmd),

    /// Enter an entry afterwards
    Log(LogCmd),

    /// Change an entry in your editor
    Edit(EditCmd),

    /// Take back the last command of this device
    Undo(UndoCmd),

    /// Remove an entry for good
    Forget(ForgetCmd),

    /// Keep the roles: the lasting responsibilities time goes to
    #[command(subcommand)]
    Role(RoleCmd),

    /// Keep the clients: who gets billed, in which currency, at which rates
    #[command(subcommand)]
    Client(ClientCmd),

    /// Keep the project records: client, role, billing and budget
    #[command(subcommand)]
    Project(ProjectCmd),

    /// Look at pace's configuration.
    #[command(subcommand)]
    Config(ConfigCmd),

    /// Look after how pace keeps its data.
    #[command(subcommand)]
    Store(StoreCmd),

    /// Print the completion script for your shell
    Completions(CompletionsCmd),
}

/// Bare `pace` shows what is running and how to get help.
impl Runnable for EntryPoint {
    fn run(&self) {
        match &self.cmd {
            Some(cmd) => cmd.run(),
            None => exit_on_error(
                now::show(NowFormat::Text)
                    .and_then(|()| print_lines(["Run `pace help` to see all commands."])),
            ),
        }
    }
}

impl Configurable<()> for EntryPoint {}

impl EntryPoint {
    /// Whether the command mends a file of the catalog, so that it runs
    /// even when the catalog doesn't read: `pace role edit` opens the
    /// broken roles file to fix it.
    pub(crate) const fn mends_catalog(&self) -> bool {
        matches!(self.cmd, Some(PaceCmd::Role(RoleCmd::Edit(_))))
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    fn mends(args: &[&str]) -> bool {
        EntryPoint::try_parse_from(args).unwrap().mends_catalog()
    }

    #[test]
    fn only_role_edit_mends_catalog() {
        assert!(mends(&["pace", "role", "edit"]));
        assert!(!mends(&["pace", "role", "list"]));
        assert!(!mends(&["pace", "now"]));
        assert!(!mends(&["pace"]));
    }
}
