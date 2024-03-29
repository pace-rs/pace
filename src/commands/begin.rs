//! `begin` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Parser;
use pace_cli::commands::begin::BeginCommandOptions;
use pace_service::get_storage_from_config;

use crate::prelude::PACE_APP;

/// `begin` subcommand
#[derive(Command, Debug, Parser)]
pub struct BeginCmd {
    #[clap(flatten)]
    begin_opts: BeginCommandOptions,
}

impl Runnable for BeginCmd {
    fn run(&self) {
        if let Ok(storage) = get_storage_from_config(&PACE_APP.config()) {
            match self.begin_opts.handle_begin(&PACE_APP.config(), storage) {
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
