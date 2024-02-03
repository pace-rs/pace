//! `begin` subcommand

use abscissa_core::{Command, Runnable};
use clap::Parser;

/// `begin` subcommand
///
/// The `Parser` proc macro generates an option parser based on the struct
/// definition, and is defined in the `clap` crate. See their documentation
/// for a more comprehensive example:
///
/// <https://docs.rs/clap/>
#[derive(Command, Debug, Parser)]
pub struct BeginCmd {
    // Baz path
    #[clap(short, long)]
    projects: Option<Vec<String>>,

    // "free" arguments don't need a macro
    description: String,
}

impl Runnable for BeginCmd {
    /// Start the application.
    fn run(&self) {
        // Your code goes here
    }
}
