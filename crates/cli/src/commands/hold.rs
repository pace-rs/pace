use std::sync::Arc;

use chrono::{FixedOffset, NaiveTime};
use chrono_tz::Tz;
#[cfg(feature = "clap")]
use clap::Parser;
use tracing::debug;

use pace_core::{
    config::PaceConfig,
    domain::{description::PaceDescription, intermission::IntermissionAction},
    options::HoldOptions,
    storage::{ActivityStateManagement, ActivityStorage, SyncStorage},
};
use pace_error::{PaceResult, UserMessage};
use pace_service::activity_store::ActivityStore;
use pace_time::{date_time::PaceDateTime, time_zone::PaceTimeZoneKind, Validate};

/// `hold` subcommand options
#[derive(Debug)]
#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(
        feature = "clap", clap(group = clap::ArgGroup::new("tz").multiple(false).required(false)))]
pub struct HoldCommandOptions {
    /// The time the activity has been holded (defaults to the current time if not provided). Format: HH:MM
    #[cfg_attr(
        feature = "clap",
        clap(long, value_name = "Pause Time", visible_alias = "at")
    )]
    pause_at: Option<NaiveTime>,

    /// The reason for the intermission, if this is not set, the description of the activity to be held will be used
    #[cfg_attr(feature = "clap", clap(short, long, value_name = "Reason"))]
    reason: Option<PaceDescription>,

    /// If there are existing intermissions, they will be finished and a new one is being created
    ///
    /// This is useful, if you want to also track the purpose of an interruption to an activity.
    #[cfg_attr(feature = "clap", clap(long, visible_alias = "new"))]
    new_if_exists: bool,

    /// Time zone to use for the activity, e.g., "Europe/Amsterdam"
    #[cfg_attr(
        feature = "clap",
        clap(
            short = 'z',
            long,
            value_name = "Time Zone",
            group = "tz",
            visible_alias = "tz"
        )
    )]
    time_zone: Option<Tz>,

    /// Time zone offset to use for the activity, e.g., "+0200" or "-0500". Format: ±HHMM
    #[cfg_attr(
        feature = "clap",
        clap(
            short = 'Z',
            long,
            value_name = "Time Zone Offset",
            group = "tz",
            visible_alias = "tzo"
        )
    )]
    time_zone_offset: Option<FixedOffset>,
}

impl HoldCommandOptions {
    /// Handles the `hold` subcommand
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration for the pace application
    ///
    /// # Errors
    ///
    /// Returns an error if the activity could not be held
    ///
    /// # Returns
    ///
    /// A `UserMessage` with the information about the held activity that can be displayed to the user
    #[tracing::instrument(skip(self))]
    pub fn handle_hold(
        &self,
        config: &PaceConfig,
        storage: Arc<dyn ActivityStorage>,
    ) -> PaceResult<UserMessage> {
        let Self {
            pause_at,
            reason,
            new_if_exists,
            time_zone,
            time_zone_offset,
        } = self;

        // Validate the time and time zone as early as possible
        let date_time = PaceDateTime::try_from((
            pause_at.as_ref(),
            PaceTimeZoneKind::try_from((time_zone.as_ref(), time_zone_offset.as_ref()))?,
            PaceTimeZoneKind::from(config.general().default_time_zone().as_ref()),
        ))?
        .validate()?;

        debug!("Parsed date time: {date_time:?}");

        let action = IntermissionAction::from(*new_if_exists);

        debug!("Intermission action: {action}");

        let hold_opts = HoldOptions::builder()
            .action(action)
            .reason(reason.clone())
            .begin_time(date_time)
            .build();

        debug!("Hold options: {hold_opts:?}");

        let activity_store = ActivityStore::with_storage(storage)?;

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
