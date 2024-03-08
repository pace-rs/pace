//! `show` subcommand

use abscissa_core::{Application, Command, Runnable};
use clap::Parser;

use crate::prelude::PACE_APP;

/// `show` subcommand
#[derive(Command, Debug, Parser)]
pub struct ShowSubCmd {}

// TODO: Show also the paths to the configuration file and the activity log that are currently being used
impl Runnable for ShowSubCmd {
    fn run(&self) {
        let config = PACE_APP.config();

        println!("\n");
        println!("{}", config);
    }
}
