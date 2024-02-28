//! `review` subcommand

use std::path::PathBuf;

use abscissa_core::{status_err, Application, Command, Runnable, Shutdown};
use chrono::NaiveDate;
use clap::Parser;
use eyre::Result;

use pace_core::{ActivityKind, PaceDate, PaceTimeFrame, ReviewFormatKind};

// TODO: For unimplemented things
// use pace_core::{
//     get_storage_from_config, ActivityKind, ActivityStore, ActivityTracker, PaceDate, PaceTimeFrame,
//     ReviewFormatKind, ReviewRequest,
// };

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

    /// Show the review for the previous month
    #[clap(long, group = "time-flag")]
    last_month: bool,
}

#[derive(Debug, Parser)]
#[clap(group = clap::ArgGroup::new("date-flag").multiple(true))]
struct DateFlags {
    /// Show the review for a specific date, mutually exclusive with `from` and `to`. Format: YYYY-MM-DD
    #[clap(long, group = "date-flag", name = "Specific Date", exclusive = true)]
    date: Option<NaiveDate>,

    /// Start date for the review period. Format: YYYY-MM-DD
    #[clap(long, group = "date-flag", name = "Start Date")]
    from: Option<NaiveDate>,

    /// End date for the review period. Format: YYYY-MM-DD
    #[clap(long, group = "date-flag", name = "End Date")]
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
    #[clap(short, long, name = "Activity Kind", alias = "kind")]
    activity_kind: Option<ActivityKind>,

    /// Filter by category name, wildcard supported
    #[clap(short, long, name = "Category", alias = "cat")]
    category: Option<String>,

    /// Specify output format (e.g., text, markdown, pdf)
    #[clap(short, long, name = "Output Format", alias = "format")]
    output_format: Option<ReviewFormatKind>,

    /// Export the review report to a specified file
    #[clap(short, long, name = "Export File", alias = "export")]
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
        unimplemented!("The `review` subcommand is not yet implemented. Please check back later.");

        // let activity_store = ActivityStore::new(get_storage_from_config(&PACE_APP.config())?);

        // let activity_tracker = ActivityTracker::with_activity_store(activity_store);

        // let ReviewCmd {
        //     activity_kind,
        //     category,
        //     output_format,
        //     export_file,
        //     time_flags,
        //     date_flags,
        //     expensive_flags,
        // } = self;

        // let time_frame = get_time_frame(time_flags, date_flags);

        // let review_request = ReviewRequest::builder()
        //     .format(output_format.unwrap_or_default())
        //     .build();

        // println!("{:#?}", self);

        // Ok(())
    }
}

/// Convert the time and date flags into a `PaceTimeFrame`
///
/// # Arguments
///
/// * `time_flags` - The time flags
/// * `date_flags` - The date flags
///
/// # Returns
///
/// A `PaceTimeFrame` representing the time frame
// TODO!: Remove this when the actual implementation is done
#[allow(dead_code)]
fn get_time_frame(time_flags: &TimeFlags, date_flags: &DateFlags) -> PaceTimeFrame {
    match (time_flags, date_flags) {
        (TimeFlags { today: true, .. }, _) => PaceTimeFrame::Today,
        (
            TimeFlags {
                yesterday: true, ..
            },
            _,
        ) => PaceTimeFrame::Yesterday,
        (
            TimeFlags {
                current_week: true, ..
            },
            _,
        ) => PaceTimeFrame::CurrentWeek,
        (
            TimeFlags {
                last_week: true, ..
            },
            _,
        ) => PaceTimeFrame::LastWeek,
        (
            TimeFlags {
                current_month: true,
                ..
            },
            _,
        ) => PaceTimeFrame::CurrentMonth,
        (
            TimeFlags {
                last_month: true, ..
            },
            _,
        ) => PaceTimeFrame::LastMonth,
        (
            _,
            DateFlags {
                date: Some(date), ..
            },
        ) => PaceTimeFrame::SpecificDate(PaceDate::from(*date)),
        (_, DateFlags { from, to, .. }) => match (from, to) {
            (Some(from), Some(to)) => {
                PaceTimeFrame::DateRange((PaceDate::from(*from), PaceDate::from(*to)).into())
            }
            (Some(from), None) => {
                PaceTimeFrame::DateRange((PaceDate::from(*from), PaceDate::default()).into())
            }
            (None, Some(to)) => {
                PaceTimeFrame::DateRange((PaceDate::with_start(), PaceDate::from(*to)).into())
            }
            _ => PaceTimeFrame::default(),
        },
    }
}
