//! `adjust` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Parser;

use crate::prelude::PACE_APP;

use pace_core::AdjustCommandOptions;

/// `adjust` subcommand
#[derive(Command, Debug, Parser)]
pub struct AdjustCmd {
    #[clap(flatten)]
    adjust_opts: AdjustCommandOptions,
}

impl Runnable for AdjustCmd {
    fn run(&self) {
        if let Err(err) = self.adjust_opts.handle_adjust(&PACE_APP.config()) {
            status_err!("{}", err);
            PACE_APP.shutdown(Shutdown::Crash);
        };
    }
}
