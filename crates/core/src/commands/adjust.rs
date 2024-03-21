use std::collections::HashSet;

use chrono::NaiveTime;
use chrono_tz::Tz;
#[cfg(feature = "clap")]
use clap::Parser;
use getset::Getters;
use pace_time::{date_time::PaceDateTime, Validate};
use tracing::debug;
use typed_builder::TypedBuilder;

use crate::{
    commands::UpdateOptions,
    config::PaceConfig,
    error::{ActivityLogErrorKind, PaceResult, UserMessage},
    service::activity_store::ActivityStore,
    storage::{get_storage_from_config, ActivityQuerying, ActivityWriteOps, SyncStorage},
};

/// `adjust` subcommand options
#[derive(Debug, Clone, PartialEq, TypedBuilder, Eq, Hash, Default, Getters)]
#[getset(get = "pub")]
#[non_exhaustive]
#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(
        feature = "clap", clap(group = clap::ArgGroup::new("adjust").multiple(true).required(true)))]
#[cfg_attr(
        feature = "clap", clap(group = clap::ArgGroup::new("tz").multiple(false).required(false)))]
pub struct AdjustCommandOptions {
    /// The category for the activity
    #[cfg_attr(
        feature = "clap",
        clap(
            short,
            long,
            group = "adjust",
            value_name = "Category",
            visible_alias = "cat"
        )
    )]
    category: Option<String>,

    /// The description of the activity
    #[cfg_attr(
        feature = "clap",
        clap(
            short,
            long,
            group = "adjust",
            value_name = "Description",
            visible_alias = "desc"
        )
    )]
    description: Option<String>,

    /// The start time of the activity. Format: HH:MM
    #[cfg_attr(
        feature = "clap",
        clap(
            short,
            long,
            group = "adjust",
            value_name = "Starting Time",
            visible_alias = "begin",
            visible_alias = "at"
        )
    )]
    start: Option<NaiveTime>,

    /// Tags for the activity
    #[cfg_attr(
        feature = "clap",
        clap(
            short,
            long,
            group = "adjust",
            name = "Tags",
            value_name = "Tags",
            visible_alias = "tag",
            value_delimiter = ','
        )
    )]
    tags: Option<Vec<String>>,

    /// Do not extend the current list of tags, but override them
    #[cfg_attr(
        feature = "clap",
        clap(
            short,
            long,
            group = "adjust",
            requires = "Tags",
            value_name = "Override Tags",
        )
    )]
    override_tags: bool,

    /// Time zone to use for the activity, e.g., "Europe/Amsterdam"
    #[cfg_attr(
        feature = "clap",
        clap(long, group = "tz", value_name = "Time Zone", visible_alias = "tz")
    )]
    time_zone: Option<Tz>,

    /// Time zone offset to use for the activity, e.g., "+0200" or "-0500". Format: ±HHMM
    #[cfg_attr(
        feature = "clap",
        clap(
            long,
            group = "tz",
            value_name = "Time Zone Offset",
            visible_alias = "tzo"
        )
    )]
    time_zone_offset: Option<String>,
}

impl AdjustCommandOptions {
    /// Handle the `adjust` subcommand
    ///
    /// # Arguments
    ///
    /// * `config` - The pace configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the activity store cannot be created or if the most recent active activity cannot be found
    ///
    /// # Returns
    ///
    /// A `UserMessage` to be printed to the user indicating the result of the operation and
    /// some additional information
    #[tracing::instrument(skip(self))]
    pub fn handle_adjust(&self, config: &PaceConfig) -> PaceResult<UserMessage> {
        let Self {
            category,
            description,
            start,
            tags,
            override_tags,
            time_zone,
            time_zone_offset,
            ..
        } = self;

        // Validate the time and time zone as early as possible
        let date_time = PaceDateTime::try_from((
            start.as_ref(),
            time_zone
                .as_ref()
                .or_else(|| config.general().default_time_zone().as_ref()),
            time_zone_offset.as_ref(),
        ))?
        .validate()?;

        debug!("Parsed time: {date_time:?}");

        let activity_store = ActivityStore::with_storage(get_storage_from_config(config)?)?;

        let activity_item = activity_store
            .most_recent_active_activity()?
            .ok_or_else(|| ActivityLogErrorKind::NoActiveActivityToAdjust)?;

        debug!("Most recent active activity item: {:?}", activity_item);

        let guid = *activity_item.guid();
        let mut activity = activity_item.activity().clone();

        if let Some(category) = category {
            debug!("Setting category to: {:?}", category);

            _ = activity.set_category(category.clone().into());
        }

        if let Some(description) = description {
            debug!("Setting description to: {:?}", description);

            _ = activity.set_description(description.clone());
        }

        if start.is_some() {
            debug!("Setting start time to: {:?}", date_time);

            _ = activity.set_begin(date_time);
        }

        if let Some(tags) = tags {
            let tags = tags.iter().cloned().collect::<HashSet<String>>();

            if *override_tags {
                debug!("Overriding tags with: {:?}", tags);
                _ = activity.set_tags(Some(tags));
            } else {
                let merged_tags = activity.tags_mut().as_mut().map_or_else(
                    || tags.clone(),
                    |existing_tags| existing_tags.union(&tags).cloned().collect(),
                );

                debug!("Setting merged tags: {:?}", merged_tags);

                _ = activity.set_tags(Some(merged_tags));
            }
        }

        _ = activity_store.update_activity(guid, activity.clone(), UpdateOptions::default())?;

        if activity_item.activity() != &activity {
            activity_store.sync()?;
            return Ok(UserMessage::new(format!(
                "{} has been adjusted.",
                activity_item.activity()
            )));
        }

        Ok(UserMessage::new("No changes were made."))
    }
}
