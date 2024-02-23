//! `resume` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};

use clap::Parser;
use eyre::Result;

// use pace_core::{get_storage_from_config, ActivityStore, SyncStorage};

use crate::prelude::PACE_APP;

/// `resume` subcommand
#[derive(Command, Debug, Parser)]
pub struct ResumeCmd {
    /// Show a list of all recent activities to continue
    #[clap(short, long)]
    list: bool,
}

impl Runnable for ResumeCmd {
    /// Start the application.
    fn run(&self) {
        if let Err(err) = self.inner_run() {
            status_err!("{}", err);
            PACE_APP.shutdown(Shutdown::Crash);
        };
    }
}

// TODO!: Resume activity should be ending all currently running intermissions.

impl ResumeCmd {
    /// Inner run implementation for the resume command
    pub fn inner_run(&self) -> Result<()> {
        // let ResumeCmd { list } = self;

        // let activity_store = ActivityStore::new(get_storage_from_config(&PACE_APP.config())?);

        // if *list {
        //     let activities = activity_store.get_recent_activities(5)?;

        //     if activities.is_empty() {
        //         println!("No recent activities to continue.");
        //     } else {
        //         todo!("Implement with dialoguer in pace_cli.");
        //     }
        // } else {
        //     let Some(activity) = activity_store.resume_last_unfinished_activity()? else {
        //         println!("No unfinished activities to resume.");
        //     };

        //     println!("Resumed {activity}");
        // }

        Ok(())
    }
}
