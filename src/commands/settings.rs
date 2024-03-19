//! `settings` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Parser;
use pace_core::prelude::SettingsCommandOptions;

use crate::prelude::PACE_APP;

/// `settings` subcommand
#[derive(Command, Debug, Parser)]
pub struct SettingsCmd {
    #[clap(flatten)]
    settings_opts: SettingsCommandOptions,
}

impl Runnable for SettingsCmd {
    fn run(&self) {
        match self.settings_opts.handle_settings(&PACE_APP.config()) {
            Ok(user_message) => user_message.display(),
            Err(err) => {
                status_err!("{}", err);
                PACE_APP.shutdown(Shutdown::Crash);
            }
        };
    }
}
