//! `now` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Parser;
use eyre::Result;

use crate::prelude::PACE_APP;

use pace_core::{
    get_storage_from_config, ActivityFilter, ActivityItem, ActivityQuerying, ActivityReadOps,
    ActivityStorage, ActivityStore,
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
    /// Inner run implementation for the now command
    pub fn inner_run(&self) -> Result<()> {
        let activity_store = ActivityStore::new(get_storage_from_config(&PACE_APP.config())?);

        activity_store.setup_storage()?;

        match activity_store.list_current_activities(ActivityFilter::Active)? {
            Some(activities) => {
                let activity_items = activities
                    .iter()
                    .flat_map(|activity_id| activity_store.read_activity(*activity_id))
                    .collect::<Vec<ActivityItem>>();

                activity_items.iter().for_each(|activity| {
                    println!("{}", activity.activity());
                });
            }
            None => {
                println!("No activities are currently running.");
            }
        }

        Ok(())
    }
}
