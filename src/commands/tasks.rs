//! `tasks` subcommand

use abscissa_core::{Command, Runnable};
use clap::Parser;

/// `tasks` subcommand
///
/// The `Parser` proc macro generates an option parser based on the struct
/// definition, and is defined in the `clap` crate. See their documentation
/// for a more comprehensive example:
///
/// <https://docs.rs/clap/>
#[derive(Command, Debug, Parser)]
pub struct TasksCmd {
    // /// Option foobar. Doc comments are the help description
    // #[clap(short)]
    // foobar: Option<PathBuf>

    // /// Baz path
    // #[clap(long)]
    // baz: Option<PathBuf>

    // "free" arguments don't need a macro
    // free_args: Vec<String>,
}

impl Runnable for TasksCmd {
    fn run(&self) {
        // Your code goes here
    }
}
