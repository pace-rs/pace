//! `begin` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Parser;

use crate::prelude::PACE_APP;

use pace_core::BeginCommandOptions;

/// `begin` subcommand
#[derive(Command, Debug, Parser)]
pub struct BeginCmd {
    #[clap(flatten)]
    begin_opts: BeginCommandOptions,
}

impl Runnable for BeginCmd {
    fn run(&self) {
        match self.begin_opts.handle_begin(&PACE_APP.config()) {
            Ok(user_message) => user_message.display(),
            Err(err) => {
                status_err!("{}", err);
                PACE_APP.shutdown(Shutdown::Crash);
            }
        };
    }
}
