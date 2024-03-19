use chrono::NaiveDate;
use getset::{Getters, MutGetters, Setters};
use serde_derive::Serialize;
use std::path::PathBuf;
use tracing::debug;
use typed_builder::TypedBuilder;

#[cfg(feature = "clap")]
use clap::Parser;

use crate::{
    config::PaceConfig,
    domain::{
        activity::ActivityKind, filter::FilterOptions, review::ReviewFormatKind,
        time::get_time_frame_from_flags,
    },
    error::{PaceResult, UserMessage},
    service::{activity_store::ActivityStore, activity_tracker::ActivityTracker},
    storage::get_storage_from_config,
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

    /// Case sensitive category filter
    #[cfg_attr(feature = "clap", clap(long, name = "Case Sensitive"))]
    case_sensitive: bool,

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

        let time_frame = get_time_frame_from_flags(self.time_flags(), self.date_flags())?;

        debug!("Displaying review for time frame: {}", time_frame);

        let Some(review_summary) =
            activity_tracker.generate_review_summary(FilterOptions::from(self), time_frame)?
        else {
            return Ok(UserMessage::new(
                "No activities found for the specified time frame",
            ));
        };

        match self.output_format() {
            Some(ReviewFormatKind::Console) | None => {
                return Ok(UserMessage::new(review_summary.to_string()));
            }
            Some(ReviewFormatKind::Json) => {
                let json = serde_json::to_string_pretty(&review_summary)?;

                debug!("Review summary: {}", json);

                // write to file if export file is specified
                if let Some(export_file) = self.export_file() {
                    std::fs::write(export_file, json)?;

                    return Ok(UserMessage::new(format!(
                        "Review report generated: {}",
                        export_file.display()
                    )));
                }

                return Ok(UserMessage::new(json));
            }

            Some(ReviewFormatKind::Html) => unimplemented!("HTML format not yet supported"),
            Some(ReviewFormatKind::Csv) => unimplemented!("CSV format not yet supported"),
            Some(ReviewFormatKind::Markdown) => unimplemented!("Markdown format not yet supported"),
            Some(ReviewFormatKind::PlainText) => {
                unimplemented!("Plain text format not yet supported")
            }
        }
    }
}

#[derive(Debug, Getters, Default, TypedBuilder, Setters, MutGetters, Clone, Eq, PartialEq)]
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
    #[builder(setter(strip_option))]
    date: Option<NaiveDate>,

    /// Start date for the review period. Format: YYYY-MM-DD
    #[cfg_attr(feature = "clap", clap(long, group = "date-flag", name = "Start Date"))]
    #[builder(setter(strip_option))]
    from: Option<NaiveDate>,

    /// End date for the review period. Format: YYYY-MM-DD
    #[cfg_attr(feature = "clap", clap(long, group = "date-flag", name = "End Date"))]
    #[builder(setter(strip_option))]
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

#[derive(Debug, Getters, TypedBuilder, Setters, MutGetters, Clone, Eq, PartialEq, Default)]
#[getset(get = "pub")]
#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(feature = "clap", clap(group = clap::ArgGroup::new("time-flag").multiple(false)))]
// We allow this here, because it's convenient to have all the flags in one place for the cli
// and because it's easier to deal with clap in this way.
#[allow(clippy::struct_excessive_bools)]
pub struct TimeFlags {
    /// Show the review for the current day
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    #[builder(setter(strip_bool))]
    today: bool,

    /// Show the review for the previous day
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    #[builder(setter(strip_bool))]
    yesterday: bool,

    /// Show the review for the current week
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    #[builder(setter(strip_bool))]
    current_week: bool,

    /// Show the review for the previous week
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    #[builder(setter(strip_bool))]
    last_week: bool,

    /// Show the review for the current month
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    #[builder(setter(strip_bool))]
    current_month: bool,

    /// Show the review for the previous month
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    #[builder(setter(strip_bool))]
    last_month: bool,
}
