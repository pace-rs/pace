//! `now` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Parser;
use pace_storage::get_storage_from_config;

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
        if let Ok(storage) = get_storage_from_config(&PACE_APP.config()) {
            match self.now_opts.handle_now(&PACE_APP.config(), storage) {
                Ok(user_message) => user_message.display(),
                Err(err) => {
                    status_err!("{}", err);
                    PACE_APP.shutdown(Shutdown::Crash);
                }
            };
        } else {
            status_err!("Failed to get storage from config");
            PACE_APP.shutdown(Shutdown::Crash);
        }
    }
}
