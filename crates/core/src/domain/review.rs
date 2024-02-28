use chrono::NaiveDate;
use getset::{Getters, MutGetters, Setters};
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use strum_macros::EnumString;
use typed_builder::TypedBuilder;

#[cfg(feature = "clap")]
use clap::Parser;

use crate::{ActivityItem, ActivityKind, PaceDateTime, PaceDuration, PaceTimeFrame};

#[derive(Debug, Getters)]
#[getset(get = "pub")]
#[cfg_attr(feature = "clap", derive(Parser))]
pub struct ReviewOptions {
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

/// Represents a request to generate a review of activities within a specified period and other criteria.
#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct ReviewRequest {
    /// The kind of review format
    #[builder(default = ReviewFormatKind::Console)]
    format: ReviewFormatKind,

    /// The criteria for the review
    #[builder(default)]
    criteria: SummaryCriteria,
}

/// The kind of review format
/// Default: `console`
///
/// Options: `console`, `html`, `markdown`, `plain-text`
#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default, EnumString, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ReviewFormatKind {
    #[default]
    Console,
    Html,
    Csv,
    #[serde(rename = "md")]
    Markdown,
    #[serde(rename = "txt")]
    PlainText,
}

/// Criteria for the review
#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct SummaryCriteria {
    time_frame: PaceTimeFrame,
    expensive_flags: ExpensiveFlags,
}

/// Represents a summary of activities and insights for a specified review period.
#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct ReviewSummary {
    /// The start date and time of the review period.
    pub period_start: PaceDateTime,

    /// The end date and time of the review period.
    pub period_end: PaceDateTime,

    /// Total time spent on all activities within the review period.
    pub total_time_spent: PaceDuration,

    /// Summary of activities grouped by a category or another relevant identifier.
    pub activities_summary: HashMap<String, ActivitySummary>,

    /// Highlights extracted from the review data, offering insights into user productivity.
    pub highlights: Highlights,

    /// Suggestions for the user based on the review, aimed at improving productivity or time management.
    pub suggestions: Vec<String>,
}

// TODO!: Maybe add this later
// pub struct ActivityStats {}

/// Detailed summary of activities, potentially within a specific category or type.
#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct ActivitySummary {
    /// Total duration spent on the grouped activities.
    pub total_duration: PaceDuration,

    /// Count of activities within the group.
    pub count: usize,

    /// Average duration of an activity within the group.
    pub average_duration: PaceDuration,
}

/// Highlights from the review period, providing quick insights into key metrics.
#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct Highlights {
    /// The day with the highest productive hours.
    pub most_productive_day: PaceDateTime,

    /// The kind of activity most frequently logged.
    pub most_frequent_activity_kind: ActivityKind,

    /// The category or activity where the most time was spent.
    pub most_time_spent_on: ActivityItem,
}
