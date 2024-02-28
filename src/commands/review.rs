//! `review` subcommand

use std::path::PathBuf;

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use chrono::NaiveDate;
use clap::Parser;
use eyre::Result;

use pace_core::{
    get_storage_from_config, ActivityKind, ActivityStore, ActivityTracker, ReviewFormatKind,
};

use crate::prelude::PACE_APP;

#[derive(Debug, Parser)]
#[clap(group = clap::ArgGroup::new("time-flag").multiple(false))]
struct TimeFlags {
    /// Show the review for the current day
    #[clap(long, group = "time-flag")]
    today: bool,

    /// Show the review for the previous day
    #[clap(long, group = "time-flag")]
    yesterday: bool,

    /// Show the review for the current week
    #[clap(long, group = "time-flag")]
    current_week: bool,

    /// Show the review for the previous week
    #[clap(long, group = "time-flag")]
    last_week: bool,

    /// Show the review for the current month
    #[clap(long, group = "time-flag")]
    current_month: bool,
}

#[derive(Debug, Parser)]
#[clap(group = clap::ArgGroup::new("date-flag").multiple(true))]
struct DateFlags {
    /// Show the review for a specific date, mutually exclusive with `from` and `to`
    #[clap(long, group = "date-flag", exclusive = true)]
    date: Option<NaiveDate>,

    /// Start date for the review period in YYYY-MM-DD format
    #[clap(long, group = "date-flag")]
    from: Option<NaiveDate>,

    /// End date for the review period in YYYY-MM-DD format
    #[clap(long, group = "date-flag")]
    to: Option<NaiveDate>,
}

#[derive(Debug, Parser)]
struct ExpensiveFlags {
    /// Include detailed time logs in the review
    #[clap(long)]
    detailed: bool,

    /// Enable comparative insights against a previous period
    #[clap(long)]
    comparative: bool,

    /// Enable personalized recommendations based on review data
    #[clap(long)]
    recommendations: bool,
}

/// `review` subcommand
#[derive(Command, Debug, Parser)]
pub struct ReviewCmd {
    /// Filter by activity kind (e.g., activity, task)
    #[clap(long)]
    activity_kind: Option<ActivityKind>,

    /// Filter by category name, wildcard supported
    #[clap(long)]
    category: Option<String>,

    /// Specify output format (e.g., text, markdown, pdf)
    #[clap(long)]
    output_format: Option<ReviewFormatKind>,

    /// Export the review report to a specified file
    #[clap(long)]
    export_file: Option<PathBuf>,

    /// Time flags
    #[clap(flatten, next_help_heading = "Flags for specifying time periods")]
    time_flags: TimeFlags,

    /// Date flags
    #[clap(
        flatten,
        next_help_heading = "Date flags for specifying custom date ranges or specific dates"
    )]
    date_flags: DateFlags,

    /// Expensive flags
    /// These flags are expensive to compute and may take longer to generate
    #[clap(flatten, next_help_heading = "Expensive flags for detailed insights")]
    expensive_flags: ExpensiveFlags,
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
        let activity_store = ActivityStore::new(get_storage_from_config(&PACE_APP.config())?);

        let _activity_tracker = ActivityTracker::with_activity_store(activity_store);

        println!("{:#?}", self);

        Ok(())
    }
}
