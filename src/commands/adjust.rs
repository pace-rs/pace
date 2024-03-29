//! `adjust` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Parser;
use pace_cli::commands::adjust::AdjustCommandOptions;
use pace_storage::get_storage_from_config;

use crate::prelude::PACE_APP;

/// `adjust` subcommand
#[derive(Command, Debug, Parser)]
pub struct AdjustCmd {
    #[clap(flatten)]
    adjust_opts: AdjustCommandOptions,
}

impl Runnable for AdjustCmd {
    fn run(&self) {
        if let Ok(storage) = get_storage_from_config(&PACE_APP.config()) {
            match self.adjust_opts.handle_adjust(&PACE_APP.config(), storage) {
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
