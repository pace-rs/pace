//! `resume` subcommand

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};

use clap::Parser;
use eyre::Result;

use pace_cli::{confirmation_or_break, prompt_resume_activity};
use pace_core::{
    get_storage_from_config, ActivityQuerying, ActivityReadOps, ActivityStateManagement,
    ActivityStore, ResumeCommandOptions, ResumeOptions, SyncStorage,
};

use crate::prelude::PACE_APP;

/// `resume` subcommand
#[derive(Command, Debug, Parser)]
pub struct ResumeCmd {
    #[clap(flatten)]
    resume_opts: ResumeCommandOptions,
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

// TODO!: Move the inner_run implementation to the pace-core crate
// TODO: Factor out cli related stuff to pace-cli
impl ResumeCmd {
    /// Inner run implementation for the resume command
    pub fn inner_run(&self) -> Result<()> {
        let activity_store = ActivityStore::new(get_storage_from_config(&PACE_APP.config())?);

        if *self.resume_opts.list() {
            // List activities to resume with fuzzy search and select
            let Some(activity_ids) = activity_store.list_most_recent_activities(usize::from(
                PACE_APP
                    .config()
                    .general()
                    .most_recent_count()
                    .unwrap_or_else(|| 9u8),
            ))?
            else {
                println!("No recent activities to continue.");
                return Ok(());
            };

            let activity_items = activity_ids
                .iter()
                // TODO: With pomodoro, we might want to filter for activities that are not intermissions
                .flat_map(|activity_id| activity_store.read_activity(*activity_id))
                .collect::<Vec<_>>();

            let string_repr = activity_items
                .iter()
                .map(|activity| activity.activity().to_string())
                .collect::<Vec<_>>();

            let selection = prompt_resume_activity(string_repr)?;

            let Some(activity_item) = activity_items.get(selection) else {
                println!("No activity selected to resume.");
                return Ok(());
            };

            let result =
                activity_store.resume_activity(*activity_item.guid(), ResumeOptions::default());

            match result {
                Ok(_) => println!("Resumed {}", activity_item.activity()),
                // Handle the case where we can't resume the activity and ask the user if they want to create a new activity
                // with the same contents
                Err(recoverable_err) if recoverable_err.possible_new_activity_from_resume() => {
                    confirmation_or_break(
                            "We can't resume this activity, do you want to begin one with the same contents?",
                        )?;

                    let new_activity = activity_item.activity().new_from_self();

                    let new_stored_activity = activity_store.begin_activity(new_activity)?;

                    println!("Resumed {}", new_stored_activity.activity());
                }
                Err(err) => return Err(err.into()),
            }
        } else if let Ok(Some(resumed_activity)) =
            activity_store.resume_most_recent_activity(ResumeOptions::default())
        {
            println!("Resumed {}", resumed_activity.activity());
        } else {
            println!("No activities to resume.");
        }

        activity_store.sync()?;

        Ok(())
    }
}
