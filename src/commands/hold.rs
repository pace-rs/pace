//! `hold` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};

use clap::Parser;
use eyre::Result;
use pace_core::{
    get_storage_from_config, parse_time_from_user_input, ActivityStateManagement, ActivityStore,
    SyncStorage,
};

use crate::prelude::PACE_APP;

/// `hold` subcommand>
#[derive(Command, Debug, Parser)]
pub struct HoldCmd {
    /// The time the activity has been holded (defaults to the current time if not provided). Format: HH:MM
    #[clap(long)]
    time: Option<String>,
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
        let HoldCmd { time } = self;

        let time = parse_time_from_user_input(time)?;

        let activity_store = ActivityStore::new(get_storage_from_config(&PACE_APP.config())?);

        let Some(activity) = activity_store.hold_last_unfinished_activity(time)? else {
            eyre::bail!("No unfinished activities to hold.");
        };

        activity_store.sync()?;

        println!("Held {activity}");

        Ok(())
    }
}
