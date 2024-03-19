#[cfg(feature = "clap")]
use clap::Parser;
use getset::Getters;
use tracing::debug;
use typed_builder::TypedBuilder;

use crate::{
    commands::EndOptions,
    config::PaceConfig,
    domain::time::parse_time_from_user_input,
    error::{PaceResult, UserMessage},
    service::activity_store::ActivityStore,
    storage::{get_storage_from_config, ActivityStateManagement, SyncStorage},
};

/// `end` subcommand options
#[derive(Debug, Clone, PartialEq, TypedBuilder, Eq, Hash, Default, Getters)]
#[getset(get = "pub")]
#[non_exhaustive]
#[cfg_attr(feature = "clap", derive(Parser))]
pub struct EndCommandOptions {
    /// The time the activity has ended (defaults to the current time if not provided). Format: HH:MM
    #[cfg_attr(
        feature = "clap",
        clap(short, long, value_name = "Finishing Time", visible_alias = "end")
    )]
    // FIXME: We should directly parse that into PaceTime or PaceNaiveDateTime
    at: Option<String>,
}

impl EndCommandOptions {
    /// Handles the `end` subcommand
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration for the pace application
    ///
    /// # Errors
    ///
    /// Returns an error if the activity could not be ended
    ///
    /// # Returns
    ///
    /// Returns a `UserMessage` with the information about the ended activity
    /// that can be displayed to the user
    #[tracing::instrument(skip(self))]
    pub fn handle_end(&self, config: &PaceConfig) -> PaceResult<UserMessage> {
        let date_time = parse_time_from_user_input(&self.at)?;

        debug!("Parsed date time: {:?}", date_time);

        let activity_store = ActivityStore::with_storage(get_storage_from_config(config)?)?;

        let end_opts = EndOptions::builder().end_time(date_time).build();

        let user_message = (activity_store.end_all_activities(end_opts)?).map_or_else(
            || "No unfinished activities to end.".to_string(),
            |unfinished_activities| {
                let mut msgs = vec![];
                for activity in &unfinished_activities {
                    debug!("Ended {}", activity.activity());

                    msgs.push(format!("Ended {}", activity.activity()));
                }

                msgs.join("\n")
            },
        );

        activity_store.sync()?;

        Ok(UserMessage::new(user_message))
    }
}
