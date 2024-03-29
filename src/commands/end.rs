//! `end` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Parser;
use pace_cli::commands::end::EndCommandOptions;
use pace_service::get_storage_from_config;

use crate::prelude::PACE_APP;

/// `end` subcommand
#[derive(Command, Debug, Parser)]
pub struct EndCmd {
    #[clap(flatten)]
    end_opts: EndCommandOptions,
}

impl Runnable for EndCmd {
    fn run(&self) {
        if let Ok(storage) = get_storage_from_config(&PACE_APP.config()) {
            match self.end_opts.handle_end(&PACE_APP.config(), storage) {
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
