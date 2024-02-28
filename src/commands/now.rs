//! `now` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Parser;

use crate::prelude::PACE_APP;

use pace_core::NowOptions;

/// `now` subcommand
#[derive(Command, Debug, Parser)]
pub struct NowCmd {
    #[clap(flatten)]
    now_opts: NowOptions,
}

impl Runnable for NowCmd {
    /// Start the application.
    fn run(&self) {
        if let Err(err) = self.now_opts.handle_now(&PACE_APP.config()) {
            status_err!("{}", err);
            PACE_APP.shutdown(Shutdown::Crash);
        };
    }
}
