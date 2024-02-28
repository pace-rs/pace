//! `docs` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Args;
use pace_core::DocsCommandOptions;

use crate::application::PACE_APP;

/// Opens the documentation.
#[derive(Command, Debug, Args, Clone)]
pub struct DocsCmd {
    /// Open the development documentation
    #[clap(flatten)]
    docs_opts: DocsCommandOptions,
}

impl Runnable for DocsCmd {
    fn run(&self) {
        match self.docs_opts.handle_docs() {
            Ok(_) => {}
            Err(err) => {
                status_err!("{}", err);
                PACE_APP.shutdown(Shutdown::Crash);
            }
        };
    }
}
