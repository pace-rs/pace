//! `review` subcommand

// use std::path::PathBuf;

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};

use clap::Parser;

use pace_core::ReviewOptions;

use crate::prelude::PACE_APP;

/// `review` subcommand
#[derive(Command, Debug, Parser)]
pub struct ReviewCmd {
    #[clap(flatten)]
    review_opts: ReviewOptions,
}

impl Runnable for ReviewCmd {
    /// Start the application.
    fn run(&self) {
        if let Err(err) = self.review_opts.handle_review(PACE_APP.config()) {
            status_err!("{}", err);
            PACE_APP.shutdown(Shutdown::Crash);
        };
    }
}
