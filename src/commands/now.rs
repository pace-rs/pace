//! `now` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Parser;

use crate::prelude::PACE_APP;

use pace_core::prelude::NowCommandOptions;

/// `now` subcommand
#[derive(Command, Debug, Parser)]
pub struct NowCmd {
    #[clap(flatten)]
    now_opts: NowCommandOptions,
}

impl Runnable for NowCmd {
    fn run(&self) {
        match self.now_opts.handle_now(&PACE_APP.config()) {
            Ok(user_message) => user_message.display(),
            Err(err) => {
                status_err!("{}", err);
                PACE_APP.shutdown(Shutdown::Crash);
            }
        };
    }
}
