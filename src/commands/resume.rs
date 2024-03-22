//! `resume` subcommand

use abscissa_core::{status_err, tracing::debug, Application, Command, Runnable, Shutdown};

use clap::Parser;
use eyre::Result;

use pace_cli::{confirmation_or_break, prompt_resume_activity};
use pace_core::prelude::{
    get_storage_from_config, ActivityQuerying, ActivityReadOps, ActivityStateManagement,
    ActivityStore, ResumeCommandOptions, ResumeOptions, SyncStorage, UserMessage,
};
use pace_time::{date_time::PaceDateTime, time_zone::PaceTimeZoneKind, Validate};

use crate::prelude::PACE_APP;

/// `resume` subcommand
#[derive(Command, Debug, Parser)]
pub struct ResumeCmd {
    #[clap(flatten)]
    resume_opts: ResumeCommandOptions,
}

impl Runnable for ResumeCmd {
    fn run(&self) {
        match self.inner_run() {
            Ok(user_message) => user_message.display(),
            Err(err) => {
                status_err!("{}", err);
                PACE_APP.shutdown(Shutdown::Crash);
            }
        };
    }
}

// TODO!: Move the inner_run implementation to the pace-core crate
// TODO: Factor out cli related stuff to pace-cli
impl ResumeCmd {
    /// Inner run implementation for the resume command
    pub fn inner_run(&self) -> Result<UserMessage> {
        let config = &PACE_APP.config();

        // Validate the time and time zone as early as possible
        let date_time = PaceDateTime::try_from((
            self.resume_opts.at().as_ref(),
            PaceTimeZoneKind::try_from((
                self.resume_opts.time_zone().as_ref(),
                self.resume_opts.time_zone_offset().as_ref(),
            ))?,
            PaceTimeZoneKind::from(config.general().default_time_zone().as_ref()),
        ))?
        .validate()?;

        debug!("Parsed time: {date_time:?}");

        let activity_store =
            ActivityStore::with_storage(get_storage_from_config(&PACE_APP.config())?)?;

        let (msg, resumed) = if let Some(resumed_activity) = activity_store
            .resume_most_recent_activity(ResumeOptions::builder().resume_time(date_time).build())?
        {
            (format!("Resumed {}", resumed_activity.activity()), true)
        } else {
            ("".to_string(), false)
        };

        // If there is no activity to resume or the user wants to list activities to resume
        let user_message = if *self.resume_opts.list() || !resumed {
            // List activities to resume with fuzzy search and select
            let Some(activity_ids) = activity_store.list_most_recent_activities(usize::from(
                PACE_APP
                    .config()
                    .general()
                    .most_recent_count()
                    // Default to '9' if the most recent count is not set
                    .unwrap_or_else(|| 9u8),
            ))?
            else {
                return Ok(UserMessage::new("No recent activities to continue."));
            };

            let activity_items = activity_ids
                .iter()
                // TODO: With pomodoro, we might want to filter for activities that are not intermissions
                .flat_map(|activity_id| activity_store.read_activity(*activity_id))
                .filter_map(|activity| activity.activity().is_resumable().then_some(activity))
                .collect::<Vec<_>>();

            if activity_items.is_empty() {
                return Ok(UserMessage::new("No activities to continue."));
            }

            let string_repr = activity_items
                .iter()
                .map(|activity| activity.activity().to_string())
                .collect::<Vec<_>>();

            let selection = prompt_resume_activity(&string_repr)?;

            let Some(activity_item) = activity_items.get(selection) else {
                return Ok(UserMessage::new("No activity selected to resume."));
            };

            let result =
                activity_store.resume_activity(*activity_item.guid(), ResumeOptions::default());

            let user_message = match result {
                Ok(_) => format!("Resumed {}", activity_item.activity()),
                // Handle the case where we can't resume the activity and ask the user if they want to create a new activity
                // with the same contents
                Err(recoverable_err) if recoverable_err.possible_new_activity_from_resume() => {
                    debug!("Activity to resume: {:?}", activity_item.activity());

                    confirmation_or_break(
                            "We can't resume this already ended activity. Do you want to begin one with the same contents?",
                        )?;

                    debug!("Creating new activity from the same contents");

                    let new_activity = activity_item.activity().new_from_self();

                    debug!("New Activity: {:?}", new_activity);

                    let new_stored_activity = activity_store.begin_activity(new_activity)?;

                    debug!("Started Activity: {:?}", new_stored_activity);

                    format!("Resumed {}", new_stored_activity.activity())
                }
                Err(err) => return Err(err.into()),
            };

            user_message
        } else {
            // If we have resumed an activity, we don't need to do anything else
            // and can just return the message from the resume
            msg
        };

        activity_store.sync()?;

        Ok(UserMessage::new(user_message))
    }
}
