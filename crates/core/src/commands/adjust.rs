use std::collections::HashSet;

use chrono::{NaiveDateTime, NaiveTime};
#[cfg(feature = "clap")]
use clap::Parser;
use getset::Getters;
use tracing::debug;
use typed_builder::TypedBuilder;

use crate::{
    error::{ActivityLogErrorKind, PaceTimeErrorKind},
    get_storage_from_config, ActivityQuerying, ActivityStore, ActivityWriteOps, PaceConfig,
    PaceDateTime, PaceResult, SyncStorage, UpdateOptions, UserMessage,
};

/// `adjust` subcommand options
#[derive(Debug, Clone, PartialEq, TypedBuilder, Eq, Hash, Default, Getters)]
#[getset(get = "pub")]
#[non_exhaustive]
#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(
        feature = "clap", clap(group = clap::ArgGroup::new("adjust").multiple(true).required(true)))]
pub struct AdjustCommandOptions {
    /// The category for the activity
    #[cfg_attr(
        feature = "clap",
        clap(short, long, group = "adjust", name = "Category", alias = "at")
    )]
    category: Option<String>,

    /// The description of the activity
    #[cfg_attr(
        feature = "clap",
        clap(short, long, group = "adjust", name = "Description", alias = "desc")
    )]
    description: Option<String>,

    /// The start time of the activity. Format: HH:MM
    #[cfg_attr(
        feature = "clap",
        clap(short, long, group = "adjust", name = "Start Time", alias = "begin")
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
            alias = "tag",
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
            name = "Override Tags",
            alias = "otag"
        )
    )]
    override_tags: bool,
}

impl AdjustCommandOptions {
    #[tracing::instrument(skip(self))]
    pub fn handle_adjust(&self, config: &PaceConfig) -> PaceResult<UserMessage> {
        let activity_store = ActivityStore::with_storage(get_storage_from_config(config)?)?;

        let activity_item = activity_store
            .most_recent_active_activity()?
            .ok_or_else(|| ActivityLogErrorKind::NoActiveActivityToAdjust)?;

        debug!("Most recent active activity item: {:?}", activity_item);

        let guid = *activity_item.guid();
        let mut activity = activity_item.activity().clone();

        if let Some(category) = &self.category {
            debug!("Setting category to: {:?}", category);

            _ = activity.set_category(category.clone().into());
        }

        if let Some(description) = &self.description {
            debug!("Setting description to: {:?}", description);

            _ = activity.set_description(description.clone());
        }

        if let Some(start) = &self.start {
            // Test if PaceDateTime actually lies in the future
            let start_time =
                PaceDateTime::new(NaiveDateTime::new(*activity.begin().date(), *start));

            // Test if PaceDateTime actually lies in the future
            if start_time > PaceDateTime::now() {
                return Err(PaceTimeErrorKind::StartTimeInFuture(start_time).into());
            };

            debug!("Setting start time to: {:?}", start_time);

            _ = activity.set_begin(start_time);
        }

        if let Some(tags) = &self.tags {
            let tags = tags.iter().cloned().collect::<HashSet<String>>();

            if self.override_tags {
                debug!("Overriding tags with: {:?}", tags);
                _ = activity.set_tags(Some(tags.clone()));
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
