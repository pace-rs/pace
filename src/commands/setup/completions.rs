//! `completions` subcommand

use abscissa_core::{Command, Runnable};

use std::io::Write;

use clap::CommandFactory;

use clap_complete::{generate, shells, Generator};
use clap_complete_nushell::Nushell;

/// `completions` subcommand
#[derive(clap::Parser, Command, Debug)]
pub struct CompletionsCmd {
    /// The shell to generate completions for
    #[clap(value_enum, name = "Shell Variant")]
    sh: ShellVariant,
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum ShellVariant {
    /// Bash shell
    Bash,

    /// Fish shell
    Fish,

    // Nushell
    Nushell,

    /// PowerShell
    Powershell,

    /// Zsh shell
    Zsh,
}

impl Runnable for CompletionsCmd {
    fn run(&self) {
        match self.sh {
            ShellVariant::Bash => generate_completion(shells::Bash, &mut std::io::stdout()),
            ShellVariant::Fish => generate_completion(shells::Fish, &mut std::io::stdout()),
            ShellVariant::Zsh => generate_completion(shells::Zsh, &mut std::io::stdout()),
            ShellVariant::Powershell => {
                generate_completion(shells::PowerShell, &mut std::io::stdout())
            }
            ShellVariant::Nushell => generate_completion(Nushell, &mut std::io::stdout()),
        }
    }
}

// We need to use `#[cfg(not(tarpaulin_include))]` to exclude this from coverage reports
#[cfg(not(tarpaulin_include))]
pub fn generate_completion<G: Generator>(shell: G, buf: &mut dyn Write) {
    let mut command = crate::commands::EntryPoint::command();
    generate(
        shell,
        &mut command,
        option_env!("CARGO_BIN_NAME").unwrap_or("pace"),
        buf,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completions() {
        generate_completion(shells::Bash, &mut std::io::sink());
        generate_completion(shells::Fish, &mut std::io::sink());
        generate_completion(Nushell, &mut std::io::sink());
        generate_completion(shells::PowerShell, &mut std::io::sink());
        generate_completion(shells::Zsh, &mut std::io::sink());
    }
}
