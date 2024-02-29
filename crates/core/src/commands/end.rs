#[cfg(feature = "clap")]
use clap::Parser;
use getset::Getters;
use typed_builder::TypedBuilder;

use crate::{
    get_storage_from_config, parse_time_from_user_input, ActivityStateManagement, ActivityStore,
    EndOptions, PaceConfig, PaceResult, SyncStorage,
};

/// `end` subcommand options
#[derive(Debug, Clone, PartialEq, TypedBuilder, Eq, Hash, Default, Getters)]
#[getset(get = "pub")]
#[non_exhaustive]
#[cfg_attr(feature = "clap", derive(Parser))]
pub struct EndCommandOptions {
    /// The time the activity has ended (defaults to the current time if not provided). Format: HH:MM
    #[cfg_attr(feature = "clap", clap(long, name = "Finishing Time", alias = "at"))]
    // FIXME: We should directly parse that into PaceTime or PaceDateTime
    end: Option<String>,

    /// End only the last unfinished activity
    #[cfg_attr(feature = "clap", clap(long))]
    only_last: bool,
}

impl EndCommandOptions {
    pub fn handle_end(&self, config: &PaceConfig) -> PaceResult<()> {
        let time = parse_time_from_user_input(&self.end)?;

        let activity_store = ActivityStore::new(get_storage_from_config(config)?);

        let end_opts = EndOptions::builder().end_time(time).build();

        if self.only_last {
            if let Some(last_activity) = activity_store.end_last_unfinished_activity(end_opts)? {
                println!("Ended {}", last_activity.activity());
            }
        } else if let Some(unfinished_activities) =
            activity_store.end_all_unfinished_activities(end_opts)?
        {
            for activity in &unfinished_activities {
                println!("Ended {}", activity.activity());
            }
        } else {
            println!("No unfinished activities to end.");
        }

        activity_store.sync()?;

        Ok(())
    }
}
