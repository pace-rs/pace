//! `resume` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};

use clap::Parser;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use eyre::Result;

use pace_core::{get_storage_from_config, ActivityQuerying, ActivityStore, SyncStorage};

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

// TODO! Implement the resume functionality
//
// Possible branches for resume:
//
// [ ] Resume last unfinished activity that has no active/open intermissions (Default without options)
// [ ] Resume from an intermission into an unfinished activity => end the intermission
// [ ] Resume a specific, ended activity => prompt _create new activity with the same contents_ (should be made clear for user), but changes the begin time, remove end time
// [ ] Resume from an active activity to another activity => prompt/warn if this is really, what someone wants, ends the currently active activity, adds flexibility, it might also encourage a less focused work approach
// [ ] Resume a specific archived activity => prompt, _create new activity with the same contents_ (should be made clear for user), but changes the begin time, remove the end time, and remove the archived status

impl ResumeCmd {
    /// Inner run implementation for the resume command
    pub fn inner_run(&self) -> Result<()> {
        let ResumeCmd { list } = self;

        let activity_store = ActivityStore::new(get_storage_from_config(&PACE_APP.config())?);

        // C: List activities to resume with fuzzy search and select
        // TODO: add symbols for intermissions, ended or archived activities
        if *list {
            if let Some(activity_log) = activity_store.list_most_recent_activities(9)? {
                let items: Vec<String> = activity_log
                    .iter()
                    .map(|activity| activity.to_string())
                    .collect();

                let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
                    .with_prompt("Which activity do you want to continue?")
                    .items(&items)
                    .interact()
                    .unwrap();

                let activity = activity_log.get(selection).unwrap();
                println!("Resumed {activity}");
            } else {
                println!("No recent activities to continue.");
            };
        // TODO! } else if let Some(activity) = activity_store.resume_last_unfinished_activity()? {
        //     println!("Resumed {activity}");
        } else {
            println!("No unfinished activities to resume.");
        }

        Ok(())
    }
}
