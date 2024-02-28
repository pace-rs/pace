//! `config` subcommand

use std::path::PathBuf;

use abscissa_core::{status_warn, Application, Command, Runnable, Shutdown};
use clap::Parser;

use dialoguer::console::Term;
use pace_cli::setup_config;

use crate::prelude::PACE_APP;

/// `config` subcommand
#[derive(Command, Debug, Parser)]
pub struct ConfigSubCmd {
    /// Path to the configuration file
    config_path: Option<PathBuf>,

    /// Path to the activity log file
    activity_log: Option<PathBuf>,
}

impl Runnable for ConfigSubCmd {
    /// Start the application.
    fn run(&self) {
        let term = Term::stdout();
        if let Err(err) = setup_config(&term) {
            // Do nothing, and let the error be, we are already panicking anyway
            _ = term.clear_screen();

            status_warn!("{}", err);
            PACE_APP.shutdown(Shutdown::Graceful);
        };
    }
}
