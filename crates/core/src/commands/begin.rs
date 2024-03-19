use std::collections::HashSet;

use chrono_tz::Tz;
#[cfg(feature = "clap")]
use clap::Parser;
use tracing::debug;

use crate::{
    config::PaceConfig,
    domain::{
        activity::{Activity, ActivityKind},
        time::extract_time_or_now,
    },
    error::{PaceResult, UserMessage},
    service::activity_store::ActivityStore,
    storage::{get_storage_from_config, ActivityStateManagement, SyncStorage},
};

/// `begin` subcommand options
#[derive(Debug)]
#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(
        feature = "clap", clap(group = clap::ArgGroup::new("tz").multiple(false).required(false)))]
pub struct BeginCommandOptions {
    /// The Category of the activity you want to start
    ///
    /// You can use the separator you setup in the configuration file
    /// to specify a subcategory.
    #[cfg_attr(feature = "clap", clap(short, long, name = "Category"))]
    category: Option<String>,

    /// The time the activity has been started at. Format: HH:MM
    // FIXME: We should directly parse that into PaceTime or PaceNaiveDateTime
    #[cfg_attr(
        feature = "clap",
        clap(short, long, value_name = "Starting Time", visible_alias = "start")
    )]
    at: Option<String>,

    /// The description of the activity you want to start
    #[cfg_attr(feature = "clap", clap(value_name = "Activity Description"))]
    description: String,

    /// The tags you want to associate with the activity, separated by a comma
    #[cfg_attr(
        feature = "clap",
        clap(
            short,
            long,
            value_name = "Tags",
            visible_alias = "tag",
            value_delimiter = ','
        )
    )]
    tags: Option<Vec<String>>,

    /// TODO: The project you want to start tracking time for
    /// FIXME: involves parsing the project configuration first
    #[cfg_attr(feature = "clap", clap(skip))]
    _projects: Option<Vec<String>>,

    /// Time zone to use for the activity, e.g., "Europe/Amsterdam"
    #[cfg_attr(
        feature = "clap",
        clap(long, value_name = "Time Zone", group = "tz", visible_alias = "tz")
    )]
    time_zone: Option<Tz>,

    /// Time zone offset to use for the activity, e.g., "+0200" or "-0500". Format: ±HHMM
    #[cfg_attr(
        feature = "clap",
        clap(
            long,
            value_name = "Time Zone Offset",
            group = "tz",
            visible_alias = "tzo"
        )
    )]
    time_zone_offset: Option<String>,
}

impl BeginCommandOptions {
    /// Handles the `begin` subcommand
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration for the pace application
    ///
    /// # Errors
    ///
    /// Returns an error if the activity could not be started
    ///
    /// # Returns
    ///
    /// Returns a `UserMessage` with the information about the started activity
    /// that can be displayed to the user
    #[tracing::instrument(skip(self))]
    pub fn handle_begin(&self, config: &PaceConfig) -> PaceResult<UserMessage> {
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

        debug!("Parsed tags: {:?}", tags);

        // parse time from string or get now
        let date_time = extract_time_or_now(start)?.validate_future()?;

        debug!("Parsed date time: {:?}", date_time);

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
            .tags(tags)
            .build();

        let activity_store = ActivityStore::with_storage(get_storage_from_config(config)?)?;

        let activity_item = activity_store.begin_activity(activity)?;

        debug!("Started Activity: {:?}", activity_item);

        activity_store.sync()?;

        Ok(UserMessage::new(format!("{}", activity_item.activity())))
    }
}
