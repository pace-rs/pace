use getset::{Getters, MutGetters, Setters};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use strum_macros::EnumString;
use typed_builder::TypedBuilder;

use crate::{ActivityItem, ActivityKind, PaceDateTime, PaceDuration};

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

/// Represents a category for summarizing activities.
// We use a string to allow for user-defined categories for now,
// but we may want to change this to an enum in the future.
pub type SummaryCategory = String;

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
    pub activities_summary: HashMap<SummaryCategory, ActivitySummary>,

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

    /// Activities within the summary group, with clean durations.
    pub clean_activities: Vec<GroupedByActivityDescriptionWithCleanDuration>,

    /// Activities within the summary group
    activities: Vec<ActivityItem>,
}

#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct GroupedByActivityDescriptionWithCleanDuration {
    description: String,
    clean_duration: PaceDuration,
}

impl GroupedByActivityDescriptionWithCleanDuration {
    pub fn new(description: String, clean_duration: PaceDuration) -> Self {
        Self {
            description,
            clean_duration,
        }
    }
}

impl ActivitySummary {
    pub fn new(
        total_duration: PaceDuration,
        average_duration: PaceDuration,
        activities: Vec<ActivityItem>,
    ) -> Self {
        Self {
            total_duration,
            average_duration,
            clean_activities: vec![], // TODO!: Implement this
            count: activities.len(),
            activities,
        }
    }
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
