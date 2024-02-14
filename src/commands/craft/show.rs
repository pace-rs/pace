//! `show` subcommand

use abscissa_core::{Command, Runnable};
use clap::Parser;

/// `show` subcommand
#[derive(Command, Debug, Parser)]
pub struct ShowSubCmd {
    // /// Option foobar. Doc comments are the help description
    // #[clap(short)]
    // foobar: Option<PathBuf>

    // /// Baz path
    // #[clap(long)]
    // baz: Option<PathBuf>

    // "free" arguments don't need a macro
    // free_args: Vec<String>,
}

impl Runnable for ShowSubCmd {
    /// Start the application.
    fn run(&self) {
        // Your code goes here
    }
}
