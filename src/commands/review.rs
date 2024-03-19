//! `review` subcommand

// use std::path::PathBuf;

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};

use clap::Parser;

use pace_core::prelude::ReviewCommandOptions;

use crate::prelude::PACE_APP;

/// `review` subcommand
#[derive(Command, Debug, Parser)]
pub struct ReviewCmd {
    #[clap(flatten)]
    review_opts: ReviewCommandOptions,
}

impl Runnable for ReviewCmd {
    fn run(&self) {
        match self.review_opts.handle_review(&PACE_APP.config()) {
            Ok(user_message) => user_message.display(),
            Err(err) => {
                status_err!("{}", err);
                PACE_APP.shutdown(Shutdown::Crash);
            }
        };
    }
}
