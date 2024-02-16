//! `begin` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Parser;
use eyre::Result;

use crate::prelude::PACE_APP;

use pace_core::{
    domain::{
        activity::{Activity, ActivityKind},
        time::extract_time_or_now,
    },
    service::activity_store::ActivityStore,
    storage::{get_storage_from_config, ActivityStateManagement, ActivityStorage, SyncStorage},
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
        let Self {
            category,
            time,
            description,
            ..
        } = self;

        // parse time from string or get now
        let date_time = extract_time_or_now(time)?;

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
            .begin(date_time.into())
            .kind(ActivityKind::default())
            .category(category.clone())
            .build();

        let activity_store = ActivityStore::new(get_storage_from_config(&PACE_APP.config())?);

        activity_store.setup_storage()?;
        let _activity = activity_store.begin_activity(activity.clone())?;
        activity_store.sync()?;

        println!("{activity}");

        Ok(())
    }
}
