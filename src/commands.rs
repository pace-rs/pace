//! The command line: the entry point and its subcommands.

pub mod client;
pub mod completions;
pub mod config;
pub mod corrections;
pub mod edit;
pub mod entry_fields;
pub mod now;
pub mod project;
pub mod project_init;
pub mod role;
pub mod setup;
pub mod store;
pub mod tracking;

use std::path::{Path, PathBuf};

use abscissa_core::{Command, Configurable, Runnable};
use pace_error::PaceError;
use pace_store::catalog::{CLIENTS_FILE, PROJECTS_DIR, ROLES_FILE};

use self::{
    client::ClientCmd,
    completions::CompletionsCmd,
    config::ConfigCmd,
    corrections::{AdjustCmd, ForgetCmd, LogCmd, UndoCmd},
    edit::EditCmd,
    now::{NowCmd, NowFormat},
    project::ProjectCmd,
    role::RoleCmd,
    setup::SetupCmd,
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

    /// Walk through the settings, roles, clients and this directory's project
    Setup(SetupCmd),

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
    /// Whether the command writes the global config file, which it
    /// creates when it is missing: `pace config set` and `pace setup`.
    pub(crate) const fn writes_config_file(&self) -> bool {
        matches!(
            &self.cmd,
            Some(PaceCmd::Config(ConfigCmd::Set(_)) | PaceCmd::Setup(_))
        )
    }

    /// The catalog file in `home` that the command mends, so that it runs
    /// even when a mistake in that file keeps the catalog from reading:
    /// `pace role edit` opens the broken roles file to fix it, `pace client
    /// edit` the clients file and `pace project edit KEY` the record of
    /// `KEY`.
    pub(crate) fn mended_catalog_file(&self, home: &Path) -> Option<PathBuf> {
        match &self.cmd {
            Some(PaceCmd::Role(RoleCmd::Edit(_))) => Some(home.join(ROLES_FILE)),
            Some(PaceCmd::Client(ClientCmd::Edit(_))) => Some(home.join(CLIENTS_FILE)),
            Some(PaceCmd::Project(ProjectCmd::Edit(edit))) => edit.record_file(home),
            _ => None,
        }
    }
}

/// The command that mends the catalog file `file` of `home`, the other
/// way round from [`EntryPoint::mended_catalog_file`]: `pace role edit`,
/// `pace client edit KEY` or `pace project edit KEY`; `None` for a file
/// that is no catalog file.
pub(crate) fn mending_command(home: &Path, file: &Path) -> Option<String> {
    if file == home.join(ROLES_FILE) {
        return Some("pace role edit".to_owned());
    }
    if file == home.join(CLIENTS_FILE) {
        return Some("pace client edit KEY".to_owned());
    }
    let is_record = file.parent() == Some(home.join(PROJECTS_DIR).as_path())
        && file
            .extension()
            .is_some_and(|extension| extension == "toml");
    let key = file.file_stem()?.to_str()?;
    is_record.then(|| format!("pace project edit {key}"))
}

/// The hint for a key that no record has, when no key is close: `list`
/// shows the `listed` records. While `left`, the mistake of another
/// catalog file of its home, stops `list`, the hint names the command that
/// fixes that file instead.
pub(crate) fn listing_hint(list: &str, listed: &str, left: Option<(&Path, &PaceError)>) -> String {
    left.and_then(|(home, mistake)| {
        let file = mistake.snippet_name()?;
        mending_command(home, Path::new(file))
    })
    .map_or_else(
        || format!("`{list}` shows the {listed}"),
        |command| format!("`{command}` fixes the mistake that stops `{list}`"),
    )
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    fn mended(args: &[&str]) -> Option<PathBuf> {
        EntryPoint::try_parse_from(args)
            .unwrap()
            .mended_catalog_file(Path::new("home"))
    }

    #[test]
    fn edit_commands_mend_the_catalog_file_they_open() {
        assert_eq!(
            mended(&["pace", "role", "edit"]),
            Some(PathBuf::from("home/roles.toml"))
        );
        assert_eq!(
            mended(&["pace", "client", "edit", "acme"]),
            Some(PathBuf::from("home/clients.toml"))
        );
        assert_eq!(
            mended(&["pace", "project", "edit", "webshop"]),
            Some(PathBuf::from("home/projects/webshop.toml"))
        );
    }

    #[test]
    fn only_config_set_and_setup_write_the_config_file() {
        let writes = |args: &[&str]| {
            EntryPoint::try_parse_from(args)
                .unwrap()
                .writes_config_file()
        };

        assert!(writes(&["pace", "config", "set", "week-start", "sunday"]));
        assert!(writes(&["pace", "setup"]));
        assert!(!writes(&["pace", "config", "get", "week-start"]));
        assert!(!writes(&["pace", "config", "show"]));
    }

    #[test]
    fn each_catalog_file_names_command_that_mends_it() {
        let home = Path::new("home");
        let command = |file: &str| mending_command(home, &home.join(file));

        assert_eq!(command("roles.toml").as_deref(), Some("pace role edit"));
        assert_eq!(
            command("clients.toml").as_deref(),
            Some("pace client edit KEY")
        );
        assert_eq!(
            command("projects/webshop.toml").as_deref(),
            Some("pace project edit webshop")
        );
        assert_eq!(
            mended(&["pace", "project", "edit", "webshop"]),
            Some(home.join("projects/webshop.toml"))
        );
    }

    #[test]
    fn file_outside_catalog_names_no_command() {
        let home = Path::new("home");

        assert_eq!(mending_command(home, Path::new("other/roles.toml")), None);
        assert_eq!(
            mending_command(home, &home.join("projects/notes.txt")),
            None
        );
        assert_eq!(mending_command(home, &home.join("pace.toml")), None);
    }

    #[test]
    fn other_commands_and_project_edit_of_no_key_mend_nothing() {
        assert_eq!(mended(&["pace", "project", "edit", "Web Shop"]), None);
        assert_eq!(mended(&["pace", "role", "list"]), None);
        assert_eq!(mended(&["pace", "client", "list"]), None);
        assert_eq!(mended(&["pace", "now"]), None);
        assert_eq!(mended(&["pace"]), None);
    }
}
