//! `pomo` subcommand

use abscissa_core::{Command, Runnable};
use clap::Parser;

/// `pomo` subcommand
///
/// The `Parser` proc macro generates an option parser based on the struct
/// definition, and is defined in the `clap` crate. See their documentation
/// for a more comprehensive example:
///
/// <https://docs.rs/clap/>
#[derive(Command, Debug, Parser)]
pub struct PomoCmd {
    // /// Option foobar. Doc comments are the help description
    // #[clap(short)]
    // foobar: Option<PathBuf>

    // /// Baz path
    // #[clap(long)]
    // baz: Option<PathBuf>

    // "free" arguments don't need a macro
    // free_args: Vec<String>,

    // /// Allow human-readable durations
    // #[arg(long)]
    // break: Option<humantime::Duration>,
}

impl Runnable for PomoCmd {
    /// Start the application.
    fn run(&self) {
        // Your code goes here
    }
}
