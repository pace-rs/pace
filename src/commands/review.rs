//! `review` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};

use clap::Parser;
use eyre::Result;

use pace_core::{get_storage_from_config, ActivityStore, ActivityTracker};

use crate::prelude::PACE_APP;
/// `review` subcommand
#[derive(Command, Debug, Parser)]
pub struct ReviewCmd {}

impl Runnable for ReviewCmd {
    /// Start the application.
    fn run(&self) {
        if let Err(err) = self.inner_run() {
            status_err!("{}", err);
            PACE_APP.shutdown(Shutdown::Crash);
        };
    }
}

impl ReviewCmd {
    fn inner_run(&self) -> Result<()> {
        let activity_store = ActivityStore::new(get_storage_from_config(&PACE_APP.config())?);

        let _activity_tracker = ActivityTracker::with_activity_store(activity_store);

        Ok(())
    }
}
