//! `now` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Parser;
use eyre::Result;

use crate::prelude::PACE_APP;

use pace_core::{
    domain::filter::ActivityFilter,
    service::activity_store::ActivityStore,
    storage::{file::TomlActivityStorage, ActivityReadOps, ActivityStorage},
};

/// `now` subcommand
#[derive(Command, Debug, Parser)]
pub struct NowCmd {}

impl Runnable for NowCmd {
    /// Start the application.
    fn run(&self) {
        if let Err(err) = self.inner_run() {
            status_err!("{}", err);
            PACE_APP.shutdown(Shutdown::Crash);
        };
    }
}

impl NowCmd {
    pub fn inner_run(&self) -> Result<()> {
        let activity_store = ActivityStore::new(Box::new(TomlActivityStorage::new(
            PACE_APP.config().general().activity_log_file_path(),
        )));

        activity_store.setup_storage()?;

        match activity_store.list_activities(ActivityFilter::Active)? {
            Some(activities) => {
                activities
                    .into_log()
                    .activities()
                    .iter()
                    .for_each(|activity| println!("{}", activity));
            }
            None => {
                println!("No activities are currently running.");
            }
        }

        Ok(())
    }
}
