#[cfg(feature = "clap")]
use clap::Parser;
use tracing::debug;

use crate::{
    get_storage_from_config, ActivityItem, ActivityQuerying, ActivityReadOps, ActivityStatusFilter,
    ActivityStore, PaceConfig, PaceResult, UserMessage,
};

/// `now` subcommand options
#[derive(Debug)]
#[cfg_attr(feature = "clap", derive(Parser))]
pub struct NowCommandOptions {}

impl NowCommandOptions {
    #[tracing::instrument(skip(self))]
    pub fn handle_now(&self, config: &PaceConfig) -> PaceResult<UserMessage> {
        let activity_store = ActivityStore::with_storage(get_storage_from_config(config)?)?;

        let user_message =
            match activity_store.list_current_activities(ActivityStatusFilter::Active)? {
                Some(activities) => {
                    debug!("Current Activities: {:?}", activities);

                    // Get the activity items
                    let activity_items = activities
                        .iter()
                        .flat_map(|activity_id| activity_store.read_activity(*activity_id))
                        .collect::<Vec<ActivityItem>>();

                    let mut msgs = vec![];
                    for activity in &activity_items {
                        msgs.push(format!("{}", activity.activity()));
                    }

                    msgs.join("\n")
                }
                None => "No activities are currently running.".to_string(),
            };

        Ok(UserMessage::new(user_message))
    }
}
