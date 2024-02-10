//! `begin` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use chrono::{Local, NaiveTime, SubsecRound};
use clap::Parser;
use eyre::Result;

use crate::prelude::PACE_APP;

use pace_core::{
    domain::activity::{Activity, ActivityKind},
    service::activity_store::ActivityStore,
    storage::{file::TomlActivityStorage, ActivityStorage},
};

/// `begin` subcommand
#[derive(Command, Debug, Parser)]
pub struct BeginCmd {
    /// The Category of the activity you want to start
    ///
    /// You can use the separator you setup in the configuration file
    /// to specify a subcategory.
    #[clap(short, long)]
    category: Option<String>,

    /// The time the activity has been started at
    #[clap(long)]
    time: Option<String>,

    /// The description of the activity you want to start
    description: String,

    /// The tags you want to associate with the activity
    #[clap(short, long)]
    tags: Option<Vec<String>>,

    /// TODO: The project you want to start tracking time for
    /// FIXME: involves parsing the project configuration first
    #[clap(skip)]
    _projects: Option<Vec<String>>,
}

impl Runnable for BeginCmd {
    /// Start the application.
    fn run(&self) {
        if let Err(err) = self.inner_run() {
            status_err!("{}", err);
            PACE_APP.shutdown(Shutdown::Crash);
        };
    }
}

impl BeginCmd {
    pub fn inner_run(&self) -> Result<()> {
        let BeginCmd {
            category,
            time,
            description,
            ..
        } = self;

        // parse time from string or set now
        let (time, date) = if let Some(ref time) = time {
            (
                NaiveTime::parse_from_str(time, "%H:%M")?,
                Local::now().date_naive(),
            )
        } else {
            // if no time is given, use the current time
            (
                Local::now().time().round_subsecs(0),
                Local::now().date_naive(),
            )
        };

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
            .start_time(time)
            .start_date(date)
            .kind(ActivityKind::default())
            .category(category.clone())
            .build();

        let activity_store = ActivityStore::new(Box::new(TomlActivityStorage::new(
            PACE_APP.config().general().activity_log_file_path(),
        )));

        activity_store.setup_storage()?;
        activity_store.save_activity(&activity)?;

        Ok(())
    }
}