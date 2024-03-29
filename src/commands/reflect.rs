//! `review` subcommand

// use std::path::PathBuf;

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};

use clap::Parser;

use pace_cli::commands::reflect::ReflectCommandOptions;
use pace_service::get_storage_from_config;

use crate::prelude::PACE_APP;

/// `review` subcommand
#[derive(Command, Debug, Parser)]
pub struct ReflectCmd {
    #[clap(flatten)]
    review_opts: ReflectCommandOptions,
}

impl Runnable for ReflectCmd {
    fn run(&self) {
        if let Ok(storage) = get_storage_from_config(&PACE_APP.config()) {
            match self.review_opts.handle_reflect(&PACE_APP.config(), storage) {
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
