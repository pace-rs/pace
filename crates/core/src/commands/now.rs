#[cfg(feature = "clap")]
use clap::Parser;

use crate::{
    get_storage_from_config, ActivityItem, ActivityQuerying, ActivityReadOps, ActivityStatusFilter,
    ActivityStore, PaceConfig, PaceResult,
};

/// `now` subcommand options
#[derive(Debug)]
#[cfg_attr(feature = "clap", derive(Parser))]
pub struct NowCommandOptions {}

impl NowCommandOptions {
    pub fn handle_now(&self, config: &PaceConfig) -> PaceResult<()> {
        let activity_store = ActivityStore::with_storage(get_storage_from_config(config)?)?;

        match activity_store.list_current_activities(ActivityStatusFilter::Active)? {
            Some(activities) => {
                let activity_items = activities
                    .iter()
                    .flat_map(|activity_id| activity_store.read_activity(*activity_id))
                    .collect::<Vec<ActivityItem>>();

                for activity in &activity_items {
                    println!("{}", activity.activity());
                }
            }
            None => {
                println!("No activities are currently running.");
            }
        }

        Ok(())
    }
}
