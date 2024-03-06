#[cfg(feature = "clap")]
use clap::Parser;
use getset::Getters;
use tracing::debug;
use typed_builder::TypedBuilder;

use crate::{
    get_storage_from_config, parse_time_from_user_input, ActivityStateManagement, ActivityStore,
    EndOptions, PaceConfig, PaceResult, SyncStorage, UserMessage,
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
        clap(short, long, name = "Finishing Time", alias = "end")
    )]
    // FIXME: We should directly parse that into PaceTime or PaceDateTime
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

        let user_message =
            if let Some(unfinished_activities) = activity_store.end_all_activities(end_opts)? {
                let mut msgs = vec![];
                for activity in &unfinished_activities {
                    debug!("Ended {}", activity.activity());

                    msgs.push(format!("Ended {}", activity.activity()));
                }

                msgs.join("\n")
            } else {
                "No unfinished activities to end.".to_string()
            };

        activity_store.sync()?;

        Ok(UserMessage::new(user_message))
    }
}
