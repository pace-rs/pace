//! `review` subcommand

// use std::path::PathBuf;

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};

use clap::Parser;

use pace_core::prelude::ReflectCommandOptions;

use crate::prelude::PACE_APP;

/// `review` subcommand
#[derive(Command, Debug, Parser)]
pub struct ReflectCmd {
    #[clap(flatten)]
    review_opts: ReflectCommandOptions,
}

impl Runnable for ReflectCmd {
    fn run(&self) {
        match self.review_opts.handle_reflect(&PACE_APP.config()) {
            Ok(user_message) => user_message.display(),
            Err(err) => {
                status_err!("{}", err);
                PACE_APP.shutdown(Shutdown::Crash);
            }
        };
    }
}
