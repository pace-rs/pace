//! `hold` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};

use clap::Parser;
use eyre::Result;
use pace_core::{
    get_storage_from_config, parse_time_from_user_input, Activity, ActivityKind,
    ActivityKindOptions, ActivityQuerying, ActivityStateManagement, ActivityStorage, ActivityStore,
    SyncStorage,
};

use crate::prelude::PACE_APP;

/// `hold` subcommand>
#[derive(Command, Debug, Parser)]
pub struct HoldCmd {
    /// The time the activity has been holded (defaults to the current time if not provided). Format: HH:MM
    #[clap(long)]
    time: Option<String>,

    /// The Category of the activity you want to start
    ///
    /// You can use the separator you setup in the configuration file
    /// to specify a subcategory.
    #[clap(short, long)]
    category: Option<String>,
}

impl Runnable for HoldCmd {
    /// Start the application.
    fn run(&self) {
        if let Err(err) = self.inner_run() {
            status_err!("{}", err);
            PACE_APP.shutdown(Shutdown::Crash);
        };
    }
}

impl HoldCmd {
    /// Inner run implementation for the hold command
    pub fn inner_run(&self) -> Result<()> {
        let HoldCmd { time, category } = self;

        let time = parse_time_from_user_input(time)?;

        let activity_store = ActivityStore::new(get_storage_from_config(&PACE_APP.config())?);

        // Get id from last activity that is not ended
        let Some(active_activity) = activity_store.latest_active_activity()? else {
            eyre::bail!("No activity to hold.");
        };

        let Some(parent_id) = active_activity.guid() else {
            eyre::bail!(
                "Activity {active_activity} has no valid ID, can't identify uniquely. Stopping."
            );
        };

        let activity_kind_opts = ActivityKindOptions::new(*parent_id);

        let activity = Activity::builder()
            .begin(time.into())
            .kind(ActivityKind::Intermission)
            .description(
                active_activity
                    .description()
                    .clone()
                    .unwrap_or_else(|| format!("Holding {active_activity}")),
            )
            .category(category.clone())
            .activity_kind_options(activity_kind_opts)
            .build();

        activity_store.setup_storage()?;

        let activity_id = activity_store.begin_activity(activity.clone())?;

        if let Some(og_activity_id) = activity.guid() {
            if activity_id == *og_activity_id {
                activity_store.sync()?;
                println!("Held {activity}");
                return Ok(());
            }
        }

        Ok(())
    }
}
