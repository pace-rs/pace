#[cfg(feature = "clap")]
use clap::Parser;
use tracing::debug;

use crate::{
    config::PaceConfig,
    domain::{activity::ActivityItem, filter::ActivityFilterKind},
    error::{PaceResult, UserMessage},
    service::activity_store::ActivityStore,
    storage::{get_storage_from_config, ActivityQuerying, ActivityReadOps},
};

/// `now` subcommand options
#[derive(Debug)]
#[cfg_attr(feature = "clap", derive(Parser))]
pub struct NowCommandOptions {}

impl NowCommandOptions {
    /// Handles the `now` subcommand
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration for the pace application
    ///
    /// # Errors
    ///
    /// Returns an error if the current activities could not be listed
    ///
    /// # Returns
    ///
    /// Returns a `UserMessage` with the information about the current activities that can be displayed to the user
    #[tracing::instrument(skip(self))]
    pub fn handle_now(&self, config: &PaceConfig) -> PaceResult<UserMessage> {
        let activity_store = ActivityStore::with_storage(get_storage_from_config(config)?)?;

        let user_message = (activity_store.list_current_activities(ActivityFilterKind::Active)?)
            .map_or_else(
                || "No activities are currently running.".to_string(),
                |activities| {
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
                },
            );

        Ok(UserMessage::new(user_message))
    }
}
