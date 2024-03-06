use chrono::NaiveDate;
use getset::{Getters, MutGetters, Setters};
use serde_derive::Serialize;
use std::path::PathBuf;
use tracing::debug;
use typed_builder::TypedBuilder;

#[cfg(feature = "clap")]
use clap::Parser;

use crate::{
    domain::review::ReviewFormatKind, get_storage_from_config, get_time_frame_from_flags,
    ActivityKind, ActivityStore, ActivityTracker, PaceConfig, PaceResult, UserMessage,
};

/// `review` subcommand options
#[derive(Debug, Getters)]
#[getset(get = "pub")]
#[cfg_attr(feature = "clap", derive(Parser))]
pub struct ReviewCommandOptions {
    /// Filter by activity kind (e.g., activity, task)
    #[cfg_attr(
        feature = "clap",
        clap(short, long, name = "Activity Kind", alias = "kind")
    )]
    activity_kind: Option<ActivityKind>,

    /// Filter by category name, wildcard supported
    #[cfg_attr(feature = "clap", clap(short, long, name = "Category", alias = "cat"))]
    category: Option<String>,

    /// Specify output format (e.g., text, markdown, pdf)
    #[cfg_attr(
        feature = "clap",
        clap(short, long, name = "Output Format", alias = "format")
    )]
    output_format: Option<ReviewFormatKind>,

    /// Export the review report to a specified file
    #[cfg_attr(
        feature = "clap",
        clap(short, long, name = "Export File", alias = "export")
    )]
    export_file: Option<PathBuf>,

    /// Time flags
    #[cfg_attr(
        feature = "clap",
        clap(flatten, next_help_heading = "Flags for specifying time periods")
    )]
    time_flags: TimeFlags,

    /// Date flags
    #[cfg_attr(
        feature = "clap",
        clap(
            flatten,
            next_help_heading = "Date flags for specifying custom date ranges or specific dates"
        )
    )]
    date_flags: DateFlags,

    /// Expensive flags
    /// These flags are expensive to compute and may take longer to generate
    #[cfg_attr(
        feature = "clap",
        clap(flatten, next_help_heading = "Expensive flags for detailed insights")
    )]
    expensive_flags: ExpensiveFlags,
}

impl ReviewCommandOptions {
    #[tracing::instrument(skip(self))]
    pub fn handle_review(&self, config: &PaceConfig) -> PaceResult<UserMessage> {
        let activity_store = ActivityStore::with_storage(get_storage_from_config(config)?)?;

        let activity_tracker = ActivityTracker::with_activity_store(activity_store);

        let time_frame: crate::PaceTimeFrame =
            get_time_frame_from_flags(self.time_flags(), self.date_flags())?;

        debug!("Displaying review for time frame: {}", time_frame);

        let _review_summary = activity_tracker.generate_review_summary(time_frame)?;

        debug!("{:#?}", self);

        Ok(UserMessage::new("Review report generated"))
    }
}

#[derive(Debug, Getters)]
#[getset(get = "pub")]
#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(
        feature = "clap", clap(group = clap::ArgGroup::new("date-flag").multiple(true)))]
pub struct DateFlags {
    /// Show the review for a specific date, mutually exclusive with `from` and `to`. Format: YYYY-MM-DD
    #[cfg_attr(
        feature = "clap",
        clap(long, group = "date-flag", name = "Specific Date", exclusive = true)
    )]
    date: Option<NaiveDate>,

    /// Start date for the review period. Format: YYYY-MM-DD
    #[cfg_attr(feature = "clap", clap(long, group = "date-flag", name = "Start Date"))]
    from: Option<NaiveDate>,

    /// End date for the review period. Format: YYYY-MM-DD
    #[cfg_attr(feature = "clap", clap(long, group = "date-flag", name = "End Date"))]
    to: Option<NaiveDate>,
}

#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
#[cfg_attr(feature = "clap", derive(Parser))]
pub struct ExpensiveFlags {
    /// Include detailed time logs in the review
    #[cfg_attr(feature = "clap", clap(long))]
    detailed: bool,

    /// Enable comparative insights against a previous period
    #[cfg_attr(feature = "clap", clap(long))]
    comparative: bool,

    /// Enable personalized recommendations based on review data
    #[cfg_attr(feature = "clap", clap(long))]
    recommendations: bool,
}

#[derive(Debug, Getters)]
#[getset(get = "pub")]
#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(feature = "clap", clap(group = clap::ArgGroup::new("time-flag").multiple(false)))]
pub struct TimeFlags {
    /// Show the review for the current day
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    today: bool,

    /// Show the review for the previous day
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    yesterday: bool,

    /// Show the review for the current week
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    current_week: bool,

    /// Show the review for the previous week
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    last_week: bool,

    /// Show the review for the current month
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    current_month: bool,

    /// Show the review for the previous month
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    last_month: bool,
}
