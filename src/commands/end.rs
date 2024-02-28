//! `end` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use clap::Parser;
use eyre::Result;

use crate::prelude::PACE_APP;

use pace_core::{
    get_storage_from_config, parse_time_from_user_input, ActivityStateManagement, ActivityStore,
    EndOptions, SyncStorage,
};
/// `end` subcommand
#[derive(Command, Debug, Parser)]
pub struct EndCmd {
    /// The time the activity has ended (defaults to the current time if not provided). Format: HH:MM
    #[clap(long, name = "Finishing Time", alias = "at")]
    end: Option<String>,

    /// End only the last unfinished activity
    #[clap(long)]
    only_last: bool,
}

impl Runnable for EndCmd {
    /// Start the application.
    fn run(&self) {
        if let Err(err) = self.inner_run() {
            status_err!("{}", err);
            PACE_APP.shutdown(Shutdown::Crash);
        };
    }
}

// TODO!: End command needs to end all activities that are currently running
// including intermissions etc.
impl EndCmd {
    fn inner_run(&self) -> Result<()> {
        let Self {
            end: time,
            only_last,
            ..
        } = self;

        let time = parse_time_from_user_input(time)?;

        let activity_store = ActivityStore::new(get_storage_from_config(&PACE_APP.config())?);

        let end_opts = EndOptions::builder().end_time(time).build();

        if *only_last {
            if let Some(last_activity) = activity_store.end_last_unfinished_activity(end_opts)? {
                println!("Ended {}", last_activity.activity());
            }
        } else if let Some(unfinished_activities) =
            activity_store.end_all_unfinished_activities(end_opts)?
        {
            for activity in &unfinished_activities {
                println!("Ended {}", activity.activity());
            }
        } else {
            println!("No unfinished activities to end.");
        }

        activity_store.sync()?;

        Ok(())
    }
}
