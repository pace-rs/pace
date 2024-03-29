//! `hold` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};

use clap::Parser;
use pace_cli::commands::hold::HoldCommandOptions;
use pace_storage::get_storage_from_config;

use crate::prelude::PACE_APP;

/// `hold` subcommand>
#[derive(Command, Debug, Parser)]
pub struct HoldCmd {
    #[clap(flatten)]
    hold_opts: HoldCommandOptions,
}

impl Runnable for HoldCmd {
    fn run(&self) {
        if let Ok(storage) = get_storage_from_config(&PACE_APP.config()) {
            match self.hold_opts.handle_hold(&PACE_APP.config(), storage) {
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
