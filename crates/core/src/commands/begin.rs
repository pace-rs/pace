use std::collections::HashSet;

use chrono::{FixedOffset, NaiveTime};
use chrono_tz::Tz;
#[cfg(feature = "clap")]
use clap::Parser;
use getset::Getters;
use pace_time::{date_time::PaceDateTime, time_zone::PaceTimeZoneKind, Validate};
use tracing::debug;

use crate::{
    config::PaceConfig,
    domain::activity::{Activity, ActivityKind},
    error::{PaceResult, UserMessage},
    service::activity_store::ActivityStore,
    storage::{get_storage_from_config, ActivityStateManagement, SyncStorage},
};

/// `begin` subcommand options
#[derive(Debug, Clone, PartialEq, Eq, Getters)]
#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(
        feature = "clap", clap(group = clap::ArgGroup::new("tz").multiple(false).required(false)))]
#[getset(get = "pub")]
pub struct BeginCommandOptions {
    /// The Category of the activity you want to start
    ///
    /// You can use the separator you setup in the configuration file
    /// to specify a subcategory.
    #[cfg_attr(feature = "clap", clap(short, long, name = "Category"))]
    category: Option<String>,

    /// The time the activity has been started at. Format: HH:MM
    #[cfg_attr(
        feature = "clap",
        clap(short, long, value_name = "Starting Time", visible_alias = "start")
    )]
    at: Option<NaiveTime>,

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
            at,
            description,
            tags,
            time_zone,
            time_zone_offset,
            .. // TODO: exclude projects for now
        } = self;

        let date_time = PaceDateTime::try_from((
            at.as_ref(),
            PaceTimeZoneKind::try_from((time_zone.as_ref(), time_zone_offset.as_ref()))?,
            PaceTimeZoneKind::from(config.general().default_time_zone().as_ref()),
        ))?
        .validate()?;

        debug!("Parsed time: {date_time:?}");

        // parse tags from string or get an empty set
        let tags = tags
            .as_ref()
            .map(|tags| tags.iter().cloned().collect::<HashSet<String>>());

        debug!("Parsed tags: {tags:?}");

        // TODO: Parse categories and subcategories from string

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
