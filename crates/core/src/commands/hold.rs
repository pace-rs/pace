#[cfg(feature = "clap")]
use clap::Parser;

use getset::Getters;
use tracing::debug;
use typed_builder::TypedBuilder;

use crate::{
    get_storage_from_config, parse_time_from_user_input, ActivityStateManagement, ActivityStore,
    IntermissionAction, PaceConfig, PaceDateTime, PaceResult, SyncStorage, UserMessage,
};

/// `hold` subcommand options
#[derive(Debug)]
#[cfg_attr(feature = "clap", derive(Parser))]
pub struct HoldCommandOptions {
    /// The time the activity has been holded (defaults to the current time if not provided). Format: HH:MM
    #[cfg_attr(feature = "clap", clap(long, name = "Pause Time", alias = "at"))]
    // FIXME: We should directly parse that into PaceTime or PaceDateTime
    pause_at: Option<String>,

    /// The reason for the intermission, if this is not set, the description of the activity to be held will be used
    #[cfg_attr(feature = "clap", clap(short, long, name = "Reason"))]
    reason: Option<String>,

    /// If there are existing intermissions, they will be finished and a new one is being created
    ///
    /// This is useful, if you want to also track the purpose of an interruption to an activity.
    #[cfg_attr(feature = "clap", clap(long))]
    new_if_exists: bool,
}

impl HoldCommandOptions {
    #[tracing::instrument(skip(self))]
    pub fn handle_hold(&self, config: &PaceConfig) -> PaceResult<UserMessage> {
        let action = if self.new_if_exists {
            IntermissionAction::New
        } else {
            IntermissionAction::Extend
        };

        let date_time = parse_time_from_user_input(&self.pause_at)?;

        debug!("Parsed date time: {:?}", date_time);

        let hold_opts = HoldOptions::builder()
            .action(action)
            .reason(self.reason.clone())
            .begin_time(date_time)
            .build();

        debug!("Hold options: {:?}", hold_opts);

        let activity_store = ActivityStore::with_storage(get_storage_from_config(config)?)?;

        let user_message =
            if let Some(activity) = activity_store.hold_most_recent_active_activity(hold_opts)? {
                debug!("Held {}", activity.activity());

                activity_store.sync()?;

                format!("Held {}", activity.activity())
            } else {
                "No unfinished activities to hold.".to_string()
            };

        Ok(UserMessage::new(user_message))
    }
}

/// Options for holding an activity
#[derive(Debug, Clone, PartialEq, TypedBuilder, Eq, Hash, Default, Getters)]
#[getset(get = "pub")]
#[non_exhaustive]
pub struct HoldOptions {
    /// The action to take on the intermission
    #[builder(default)]
    action: IntermissionAction,

    /// The start time of the intermission
    #[builder(default, setter(into))]
    begin_time: PaceDateTime,

    /// The reason for holding the activity
    #[builder(default, setter(into))]
    reason: Option<String>,
}
