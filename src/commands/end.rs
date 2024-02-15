//! `end` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use chrono::{Local, NaiveDateTime, NaiveTime};
use clap::Parser;
use eyre::Result;

use crate::prelude::PACE_APP;

use pace_core::{
    service::activity_store::ActivityStore,
    storage::{file::TomlActivityStorage, ActivityStateManagement, ActivityStorage},
};

/// `end` subcommand
#[derive(Command, Debug, Parser)]
pub struct EndCmd {
    /// The time the activity has ended (defaults to the current time if not provided). Format: HH:MM
    #[clap(long)]
    time: Option<String>,

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

impl EndCmd {
    pub fn inner_run(&self) -> Result<()> {
        let EndCmd {
            time, only_last, ..
        } = self;

        let time = time
            .as_ref()
            .map(|time| -> Result<NaiveDateTime> {
                let time = NaiveTime::parse_from_str(time, "%H:%M")
                    .map_err(|err| eyre::eyre!("Invalid time format: {}", err))?;
                Ok(NaiveDateTime::new(Local::now().date_naive(), time))
            })
            .transpose()?;

        // TODO!: Make storage configurable via Config file
        let activity_store = ActivityStore::new(Box::new(TomlActivityStorage::new(
            PACE_APP.config().general().activity_log_file_path(),
        )));

        activity_store.setup_storage()?;

        if *only_last {
            if let Some(last_activity) = activity_store.end_last_unfinished_activity(time)? {
                println!("Ended {last_activity}");
            }
        } else if let Some(unfinished_activities) =
            activity_store.end_all_unfinished_activities(time)?
        {
            for activity in unfinished_activities {
                println!("Ended {activity}");
            }
        } else {
            println!("No unfinished activities to end.");
        }

        Ok(())
    }
}
