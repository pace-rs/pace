//! `show` subcommand

use abscissa_core::{Application, Command, Runnable};
use clap::Parser;

use crate::prelude::PACE_APP;

/// `show` subcommand
#[derive(Command, Debug, Parser)]
pub struct ShowSubCmd {}

impl Runnable for ShowSubCmd {
    fn run(&self) {
        let config = PACE_APP.config();

        println!("\n");
        println!("{}", config);
    }
}
