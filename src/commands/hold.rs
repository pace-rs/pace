//! `hold` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};

use clap::Parser;
use pace_core::HoldCommandOptions;

use crate::prelude::PACE_APP;

/// `hold` subcommand>
#[derive(Command, Debug, Parser)]
pub struct HoldCmd {
    #[clap(flatten)]
    hold_opts: HoldCommandOptions,
}

impl Runnable for HoldCmd {
    fn run(&self) {
        match self.hold_opts.handle_hold(&PACE_APP.config()) {
            Ok(user_message) => user_message.display(),
            Err(err) => {
                status_err!("{}", err);
                PACE_APP.shutdown(Shutdown::Crash);
            }
        };
    }
}
