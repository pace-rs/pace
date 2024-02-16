//! `hold` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};

use clap::Parser;
use eyre::Result;

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
    pub fn inner_run(&self) -> Result<()> {
        // TODO!: Implement hold command
        //
        // let HoldCmd { time } = self;

        // let time = parse_time_from_user_input(time)?;

        // let activity_store = ActivityStore::new(get_storage_from_config(&PACE_APP.config())?);

        // activity_store.setup_storage()?;

        // if let Some(held_activity) = activity_store
        //     .end_or_hold_activities(ActivityEndKind::Hold, time)?
        //     .try_into_hold()?
        // {
        //     println!("Held {held_activity}");
        // } else {
        //     println!("No unfinished activities to hold.");
        // }

        Ok(())
    }
}
