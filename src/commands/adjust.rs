//! `adjust` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Parser;

use crate::prelude::PACE_APP;

use pace_core::prelude::AdjustCommandOptions;

/// `adjust` subcommand
#[derive(Command, Debug, Parser)]
pub struct AdjustCmd {
    #[clap(flatten)]
    adjust_opts: AdjustCommandOptions,
}

impl Runnable for AdjustCmd {
    fn run(&self) {
        match self.adjust_opts.handle_adjust(&PACE_APP.config()) {
            Ok(user_message) => user_message.display(),
            Err(err) => {
                status_err!("{}", err);
                PACE_APP.shutdown(Shutdown::Crash);
            }
        };
    }
}
