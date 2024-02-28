//! `begin` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Parser;

use crate::prelude::PACE_APP;

use pace_core::BeginOptions;

/// `begin` subcommand
#[derive(Command, Debug, Parser)]
pub struct BeginCmd {
    #[clap(flatten)]
    begin_opts: BeginOptions,
}

impl Runnable for BeginCmd {
    fn run(&self) {
        if let Err(err) = self.begin_opts.handle_begin(&PACE_APP.config()) {
            status_err!("{}", err);
            PACE_APP.shutdown(Shutdown::Crash);
        };
    }
}
