//! `config` subcommand

use std::path::PathBuf;

use abscissa_core::{status_warn, Application, Command, Runnable, Shutdown};
use clap::Parser;

use dialoguer::console::Term;
use pace_cli::{setup_config, PathOptions};

use crate::prelude::PACE_APP;

/// `config` subcommand
#[derive(Command, Debug, Parser)]
pub struct ConfigSubCmd {
    /// Specify a custom directory for the activity log file
    #[clap(long, short, name = "Activity Log Directory", value_hint = clap::ValueHint::DirPath)]
    activity_log_dir: Option<PathBuf>,
}

impl Runnable for ConfigSubCmd {
    /// Start the application.
    fn run(&self) {
        let term = Term::stdout();

        let ConfigSubCmd {
            activity_log_dir: activity_log,
        } = self;

        let path_opts = PathOptions::builder()
            .activity_log(activity_log.clone())
            .build();

        if let Err(err) = setup_config(&term, path_opts) {
            // Do nothing, and let the error be, we are already panicking anyway
            _ = term.clear_screen();

            status_warn!("{}", err);
            PACE_APP.shutdown(Shutdown::Graceful);
        };
    }
}
