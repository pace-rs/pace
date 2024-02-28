//! `docs` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Args;

use crate::application::PACE_APP;

/// Opens the documentation.
#[derive(Command, Debug, Args, Clone)]
pub struct DocsCmd {
    /// Open the development documentation
    #[clap(short, long)]
    dev: bool,
}

impl Runnable for DocsCmd {
    fn run(&self) {
        let DocsCmd { dev } = self;

        match dev {
            true => match open::that("https://pace.cli.rs/dev-docs") {
                Ok(_) => {}
                Err(err) => {
                    status_err!("{}", err);
                    PACE_APP.shutdown(Shutdown::Crash);
                }
            },
            false => match open::that("https://pace.cli.rs/docs") {
                Ok(_) => {}
                Err(err) => {
                    status_err!("{}", err);
                    PACE_APP.shutdown(Shutdown::Crash);
                }
            },
        }
    }
}
