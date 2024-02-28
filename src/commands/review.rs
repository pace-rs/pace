//! `review` subcommand

// use std::path::PathBuf;

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
// use chrono::NaiveDate;
use clap::Parser;
use eyre::Result;

use pace_core::ReviewOptions;

// TODO: For unimplemented things
// use pace_core::{
//     get_storage_from_config, get_time_frame_from_flags, ActivityKind, ActivityStore,
//     ActivityTracker, PaceDate, PaceTimeFrame, ReviewOptions, ReviewRequest,
// };

use crate::prelude::PACE_APP;

/// `review` subcommand
#[derive(Command, Debug, Parser)]
pub struct ReviewCmd {
    #[clap(flatten)]
    review_options: ReviewOptions,
}

impl Runnable for ReviewCmd {
    /// Start the application.
    fn run(&self) {
        if let Err(err) = self.inner_run() {
            status_err!("{}", err);
            PACE_APP.shutdown(Shutdown::Crash);
        };
    }
}

impl ReviewCmd {
    fn inner_run(&self) -> Result<()> {
        unimplemented!("The `review` subcommand is not yet implemented. Please check back later.");

        // let activity_store = ActivityStore::new(get_storage_from_config(&PACE_APP.config())?);

        // let activity_tracker = ActivityTracker::with_activity_store(activity_store);

        // let time_frame = get_time_frame_from_flags(
        //     self.review_options.time_flags(),
        //     self.review_options.date_flags(),
        // );

        // let review_request = ReviewRequest::builder()
        //     .format(self.review_options.output_format().unwrap_or_default())
        //     .build();

        // println!("{:#?}", self);

        // Ok(())
    }
}
