//! `resume` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};

use clap::Parser;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use eyre::Result;

use pace_core::{get_storage_from_config, ActivityQuerying, ActivityStore};

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

// [ ] Resume a specific, ended activity => prompt _create new activity with the same contents_ (should be made clear for user), but changes the begin time, remove end time
// [ ] Resume from an active activity to another activity => prompt/warn if this is really, what someone wants, ends the currently active activity, adds flexibility, it might also encourage a less focused work approach
// [ ] Resume a specific archived activity => prompt, _create new activity with the same contents_ (should be made clear for user), but changes the begin time, remove the end time, and remove the archived status

// [ ] Resume from an active intermission, but there are no unfinished activities available (someone forgot something?) => end the intermission, check parent_id resume that??? or what we do?

impl ResumeCmd {
    /// Inner run implementation for the resume command
    pub fn inner_run(&self) -> Result<()> {
        let ResumeCmd { list } = self;

        let activity_store = ActivityStore::new(get_storage_from_config(&PACE_APP.config())?);

        if *list {
            // List activities to resume with fuzzy search and select
            // TODO: add symbols for intermissions, ended or archived activities
            if let Some(activity_log) = activity_store.list_most_recent_activities(usize::from(
                PACE_APP
                    .config()
                    .general()
                    .most_recent_count()
                    .unwrap_or_else(|| 9u8),
            ))? {
                let items: Vec<String> = activity_log
                    .iter()
                    // TODO: With pomodoro, we might want to filter for activities that are not intermissions
                    .filter(|activity| activity.kind().is_activity())
                    .map(|activity| activity.to_string())
                    .collect();

                let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
                    .with_prompt("Which activity do you want to continue?")
                    .items(&items)
                    .interact()
                    .unwrap();

                let activity = activity_log.get(selection).unwrap();

                // TODO: check what other things are needed to resume this activity
                // TODO: Anything to sync to storage?
                println!("Resumed {activity}");
            } else {
                println!("No recent activities to continue.");
            };
        } else if let Ok(active_intermissions) = activity_store.list_active_intermissions() {
            if let Some(activity_log) = active_intermissions {
                // TODO: check for open intermissions
                // TODO: if there is no open intermission, resume the last unfinished activity
                if activity_log.len() == 0 {
                    // TODO: Resume last unfinished activity that has no active/open intermissions (Default without options)
                    println!("Resume last unfinished activity");
                } else {
                    // TODO: Resume from an active intermission into an unfinished activity => end all intermissions, use the most_recent intermissions parent_id
                    // TODO: Sync the updated intermission to storage
                }
            } else {
                println!("No active intermissions to continue.");
            };
            // TODO: if there is an open intermission, get the parent activity, end the intermission and resume the parent activity
        } else {
            println!("No activities to resume.");
        }

        Ok(())
    }
}
