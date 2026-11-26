//! `pace completions`: the completion script for a shell.

use abscissa_core::{Command, Runnable};
use clap::{CommandFactory, ValueEnum};
use clap_complete::generate;
use clap_complete_nushell::Nushell;

use crate::commands::EntryPoint;

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
        let mut command = EntryPoint::command();
        let mut out = std::io::stdout();
        let name = "pace";
        match self.shell {
            Shell::Bash => generate(clap_complete::Shell::Bash, &mut command, name, &mut out),
            Shell::Zsh => generate(clap_complete::Shell::Zsh, &mut command, name, &mut out),
            Shell::Fish => generate(clap_complete::Shell::Fish, &mut command, name, &mut out),
            Shell::Powershell => {
                generate(
                    clap_complete::Shell::PowerShell,
                    &mut command,
                    name,
                    &mut out,
                );
            }
            Shell::Elvish => generate(clap_complete::Shell::Elvish, &mut command, name, &mut out),
            Shell::Nushell => generate(Nushell, &mut command, name, &mut out),
        }
    }
}
