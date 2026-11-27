//! `pace completions`: the completion script for a shell.

use abscissa_core::{Command, Runnable};
use clap::{CommandFactory, ValueEnum};
use clap_complete::generate;
use clap_complete_nushell::Nushell;

use crate::application::exit_on_error;
use crate::commands::EntryPoint;
use crate::print::print_bytes;

/// `pace completions`
#[derive(clap::Args, Command, Debug)]
#[command(
    after_help = "Write the script where your shell looks for completions; pace's \
                        documentation says where for each shell."
)]
pub struct CompletionsCmd {
    #[arg(value_enum, help = "The shell to complete in")]
    shell: Shell,
}

/// The shells pace completes in.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum Shell {
    Bash,
    Zsh,
    Fish,
    Powershell,
    Elvish,
    Nushell,
}

impl Runnable for CompletionsCmd {
    fn run(&self) {
        exit_on_error(print_bytes(&self.script()));
    }
}

impl CompletionsCmd {
    /// The completion script, whole, so a failed write is pace's error and
    /// not a panic in the generator.
    fn script(&self) -> Vec<u8> {
        let mut command = EntryPoint::command();
        let mut script = Vec::new();
        let name = "pace";
        match self.shell {
            Shell::Bash => generate(clap_complete::Shell::Bash, &mut command, name, &mut script),
            Shell::Zsh => generate(clap_complete::Shell::Zsh, &mut command, name, &mut script),
            Shell::Fish => generate(clap_complete::Shell::Fish, &mut command, name, &mut script),
            Shell::Powershell => generate(
                clap_complete::Shell::PowerShell,
                &mut command,
                name,
                &mut script,
            ),
            Shell::Elvish => generate(
                clap_complete::Shell::Elvish,
                &mut command,
                name,
                &mut script,
            ),
            Shell::Nushell => generate(Nushell, &mut command, name, &mut script),
        }
        script
    }
}
