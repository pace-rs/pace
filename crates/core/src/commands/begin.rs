use std::collections::HashSet;

#[cfg(feature = "clap")]
use clap::Parser;

use crate::{
    extract_time_or_now, get_storage_from_config, Activity, ActivityKind, ActivityStateManagement,
    ActivityStore, PaceConfig, PaceResult, SyncStorage,
};

/// `begin` subcommand options
#[derive(Debug)]
#[cfg_attr(feature = "clap", derive(Parser))]
pub struct BeginCommandOptions {
    /// The Category of the activity you want to start
    ///
    /// You can use the separator you setup in the configuration file
    /// to specify a subcategory.
    #[cfg_attr(feature = "clap", clap(short, long, name = "Category"))]
    category: Option<String>,

    /// The time the activity has been started at. Format: HH:MM
    // FIXME: We should directly parse that into PaceTime or PaceDateTime
    #[cfg_attr(
        feature = "clap",
        clap(short, long, name = "Starting Time", alias = "start")
    )]
    at: Option<String>,

    /// The description of the activity you want to start
    #[cfg_attr(feature = "clap", clap(name = "Activity Description"))]
    description: String,

    /// The tags you want to associate with the activity, separated by a comma
    #[cfg_attr(
        feature = "clap",
        clap(short, long, name = "Tags", alias = "tag", value_delimiter = ',')
    )]
    tags: Option<Vec<String>>,

    /// TODO: The project you want to start tracking time for
    /// FIXME: involves parsing the project configuration first
    #[cfg_attr(feature = "clap", clap(skip))]
    _projects: Option<Vec<String>>,
}

impl BeginCommandOptions {
    /// Inner run implementation for the begin command
    pub fn handle_begin(&self, config: &PaceConfig) -> PaceResult<()> {
        let Self {
            category,
            at: start,
            description,
            tags,
            .. // TODO: exclude projects for now
        } = self;

        // parse tags from string or get an empty set
        let tags = tags
            .as_ref()
            .map(|tags| tags.iter().cloned().collect::<HashSet<String>>());

        // parse time from string or get now
        let date_time = extract_time_or_now(start)?;

        // TODO: Parse categories and subcategories from string
        // let (category, subcategory) = if let Some(ref category) = category {
        //     let separator = config.general().category_separator();
        //     extract_categories(category.as_str(), separator.as_str())
        // } else {
        //     // if no category is given, use the default category
        //     // FIXME: This should be the default category from the project configuration
        //     // but for now, we'll just use category defaults
        //     //
        //     // FIXME: We might also want to merge the project configuration with the general configuration first to have precedence
        //     //
        //     // let category = if let Some(category) = PACE_APP.config().general().default_category() {
        //     //     category
        //     // } else {
        //     // &Category::default()
        //     // };

        //     (Category::default(), None)
        // };

        let activity = Activity::builder()
            .description(description.clone())
            .begin(date_time)
            .kind(ActivityKind::default())
            .category(category.clone())
            .tags(tags.clone())
            .build();

        let activity_store = ActivityStore::new(get_storage_from_config(config)?);

        let activity_item = activity_store.begin_activity(activity.clone())?;

        activity_store.sync()?;

        println!("{}", activity_item.activity());

        Ok(())
    }
}
