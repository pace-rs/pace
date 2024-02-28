//! `hold` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};

use clap::Parser;
use eyre::Result;
use pace_core::{
    get_storage_from_config, parse_time_from_user_input, ActivityStateManagement, ActivityStore,
    HoldOptions, IntermissionAction, SyncStorage,
};

use crate::prelude::PACE_APP;

/// `hold` subcommand>
#[derive(Command, Debug, Parser)]
pub struct HoldCmd {
    /// The time the activity has been holded (defaults to the current time if not provided). Format: HH:MM
    #[clap(long, name = "Pause Time", alias = "at")]
    pause_at: Option<String>,

    /// The reason for the intermission, if this is not set, the description of the activity to be held will be used
    #[clap(short, long, name = "Reason")]
    reason: Option<String>,

    /// If there are existing intermissions, they will be finished and a new one is being created
    ///
    /// This is useful, if you want to also track the purpose of an interruption to an activity.
    #[clap(long)]
    new_if_exists: bool,
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
        let HoldCmd {
            pause_at: time,
            new_if_exists,
            reason,
        } = self;

        let action = if *new_if_exists {
            IntermissionAction::New
        } else {
            IntermissionAction::Extend
        };

        let time = parse_time_from_user_input(time)?;

        let hold_opts = HoldOptions::builder()
            .action(action)
            .reason(reason.clone())
            .begin_time(time)
            .build();

        let activity_store = ActivityStore::new(get_storage_from_config(&PACE_APP.config())?);

        if let Some(activity) = activity_store.hold_most_recent_active_activity(hold_opts)? {
            activity_store.sync()?;
            println!("Held {}", activity.activity());
        } else {
            println!("No unfinished activities to hold.");
        };

        Ok(())
    }
}
