//! `end` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Parser;

use crate::prelude::PACE_APP;

use pace_core::EndOptions;

/// `end` subcommand
#[derive(Command, Debug, Parser)]
pub struct EndCmd {
    #[clap(flatten)]
    end_opts: EndOptions,
}

impl Runnable for EndCmd {
    /// Start the application.
    fn run(&self) {
        if let Err(err) = self.end_opts.handle_end(&PACE_APP.config()) {
            status_err!("{}", err);
            PACE_APP.shutdown(Shutdown::Crash);
        };
    }
}
