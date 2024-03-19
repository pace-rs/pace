//! `end` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Parser;

use crate::prelude::PACE_APP;

use pace_core::prelude::EndCommandOptions;

/// `end` subcommand
#[derive(Command, Debug, Parser)]
pub struct EndCmd {
    #[clap(flatten)]
    end_opts: EndCommandOptions,
}

impl Runnable for EndCmd {
    fn run(&self) {
        match self.end_opts.handle_end(&PACE_APP.config()) {
            Ok(user_message) => user_message.display(),
            Err(err) => {
                status_err!("{}", err);
                PACE_APP.shutdown(Shutdown::Crash);
            }
        };
    }
}
