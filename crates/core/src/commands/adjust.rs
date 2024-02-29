use std::collections::HashSet;

use chrono::{NaiveDateTime, NaiveTime};
#[cfg(feature = "clap")]
use clap::Parser;
use getset::Getters;
use typed_builder::TypedBuilder;

use crate::{
    error::ActivityLogErrorKind, get_storage_from_config, ActivityQuerying, ActivityStore,
    ActivityWriteOps, PaceConfig, PaceDateTime, PaceResult, SyncStorage, UpdateOptions,
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
    pub fn handle_adjust(&self, config: &PaceConfig) -> PaceResult<()> {
        let activity_store = ActivityStore::new(get_storage_from_config(config)?);

        let activity_item = activity_store
            .most_recent_active_activity()?
            .ok_or_else(|| ActivityLogErrorKind::NoActiveActivityToAdjust)?;

        let guid = *activity_item.guid();
        let mut activity = activity_item.activity().clone();

        if let Some(category) = &self.category {
            _ = activity.set_category(category.clone().into());
        }

        if let Some(description) = &self.description {
            _ = activity.set_description(description.clone());
        }

        if let Some(start) = &self.start {
            let start_time =
                PaceDateTime::new(NaiveDateTime::new(*activity.begin().date(), *start));
            _ = activity.set_begin(start_time);
        }

        if let Some(tags) = &self.tags {
            let tags = tags.iter().cloned().collect::<HashSet<String>>();

            if self.override_tags {
                _ = activity.set_tags(Some(tags.clone()));
            } else {
                let merged_tags = activity.tags_mut().as_mut().map_or_else(
                    || tags.clone(),
                    |existing_tags| existing_tags.union(&tags).cloned().collect(),
                );
                _ = activity.set_tags(Some(merged_tags));
            }
        }

        _ = activity_store.update_activity(guid, activity.clone(), UpdateOptions::default())?;

        if activity_item.activity() != &activity {
            activity_store.sync()?;
            println!("{} has been adjusted.", activity);
        } else {
            println!("No changes were made.");
        }

        Ok(())
    }
}
