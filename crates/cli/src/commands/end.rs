use std::sync::Arc;

use chrono::{FixedOffset, NaiveTime};
use chrono_tz::Tz;
#[cfg(feature = "clap")]
use clap::Parser;
use getset::Getters;
use tracing::debug;
use typed_builder::TypedBuilder;

use pace_core::{
    config::PaceConfig,
    options::EndOptions,
    storage::{ActivityStateManagement, ActivityStorage, SyncStorage},
};
use pace_error::{PaceResult, UserMessage};
use pace_service::activity_store::ActivityStore;
use pace_time::{date_time::PaceDateTime, time_zone::PaceTimeZoneKind, Validate};

/// `end` subcommand options
#[derive(Debug, Clone, PartialEq, TypedBuilder, Eq, Hash, Default, Getters)]
#[getset(get = "pub")]
#[non_exhaustive]
#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(
        feature = "clap", clap(group = clap::ArgGroup::new("tz").multiple(false).required(false)))]
pub struct EndCommandOptions {
    /// The time the activity has ended (defaults to the current time if not provided). Format: HH:MM
    #[cfg_attr(
        feature = "clap",
        clap(short, long, value_name = "Finishing Time", visible_alias = "end")
    )]
    at: Option<NaiveTime>,

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
    pub fn handle_end(
        &self,
        config: &PaceConfig,
        storage: Arc<dyn ActivityStorage>,
    ) -> PaceResult<UserMessage> {
        let Self {
            at,
            time_zone,
            time_zone_offset,
            ..
        } = self;

        // Validate the time and time zone as early as possible
        let date_time = PaceDateTime::try_from((
            at.as_ref(),
            PaceTimeZoneKind::try_from((time_zone.as_ref(), time_zone_offset.as_ref()))?,
            PaceTimeZoneKind::from(config.general().default_time_zone().as_ref()),
        ))?
        .validate()?;

        debug!("Parsed date time: {:?}", date_time);

        let activity_store = ActivityStore::with_storage(storage)?;

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
