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
    /// Start the application.
    fn run(&self) {
        if let Err(err) = self.hold_opts.handle_hold(&PACE_APP.config()) {
            status_err!("{}", err);
            PACE_APP.shutdown(Shutdown::Crash);
        };
    }
}
