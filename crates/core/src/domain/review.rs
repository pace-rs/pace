use getset::{Getters, MutGetters, Setters};
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strum_macros::EnumString;
use tabled::{
    builder::Builder,
    settings::{object::Columns, Alignment, Modify, Padding, Panel, Settings, Style},
};
use tracing::debug;
use typed_builder::TypedBuilder;

use crate::{ActivityItem, ActivityKind, PaceDateTime, PaceDuration, TimeRangeOptions};

/// The kind of review format
/// Default: `console`
///
/// Options: `console`, `html`, `markdown`, `plain-text`
#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default, EnumString, PartialEq, Eq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ReviewFormatKind {
    #[default]
    Console,
    Json,
    Html,
    Csv,
    #[cfg_attr(feature = "clap", clap(alias("md")))]
    #[serde(rename = "md")]
    Markdown,
    #[cfg_attr(feature = "clap", clap(alias("txt")))]
    #[serde(rename = "txt")]
    PlainText,
}

/// Represents a category for summarizing activities.
// We use a string to allow for user-defined categories for now,
// but we may want to change this to an enum in the future.
pub type SummaryCategory = String;

pub type SummaryGroupByCategory = BTreeMap<SummaryCategory, SummaryGroup>;

/// Represents a summary of activities and insights for a specified review period.
#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct ReviewSummary {
    /// The time range of the review period.
    time_range: TimeRangeOptions,

    /// Total time spent on all activities within the review period.
    total_time_spent: PaceDuration,

    /// Total time spent on intermissions within the review period.
    total_break_duration: PaceDuration,

    /// Summary of activities grouped by a category or another relevant identifier.
    summary_groups_by_category: SummaryGroupByCategory,
    // TODO: Highlights extracted from the review data, offering insights into user productivity.
    // highlights: Highlights,

    // TODO: Suggestions for the user based on the review, aimed at improving productivity or time management.
    // suggestions: Vec<String>,
}

impl ReviewSummary {
    pub fn new(
        time_range: TimeRangeOptions,
        summary_groups_by_category: SummaryGroupByCategory,
    ) -> Self {
        let total_time_spent = PaceDuration::from_seconds(
            summary_groups_by_category
                .values()
                .map(|group| group.total_duration().as_secs())
                .sum(),
        );

        let total_break_duration = PaceDuration::from_seconds(
            summary_groups_by_category
                .values()
                .map(|group| group.total_break_duration().as_secs())
                .sum(),
        );

        Self {
            total_break_duration,
            time_range,
            total_time_spent,
            summary_groups_by_category,
        }
    }
}

// TODO!: Refine the display of the review summary
impl std::fmt::Display for ReviewSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut builder = Builder::new();

        builder.push_record(vec![
            "Category",
            "Description",
            "Duration",
            "Breaks (Amount)",
        ]);

        for (category, summary_group) in self.summary_groups_by_category.iter() {
            builder.push_record(vec![
                category,
                "",
                &summary_group.total_duration().to_string(),
                &summary_group.total_break_duration().to_string(),
            ]);

            for activity_group in summary_group.activity_groups() {
                builder.push_record(vec![
                    "",
                    activity_group.description(),
                    &activity_group.adjusted_duration().to_string(),
                    format!(
                        "{} ({})",
                        &activity_group.intermission_duration().to_string(),
                        &activity_group.intermissions().len().to_string()
                    )
                    .as_str(),
                ]);
            }
        }

        builder.push_record(vec![
            "Total",
            "",
            &self.total_time_spent.to_string(),
            &self.total_break_duration.to_string(),
        ]);

        let table_config = Settings::default()
            .with(Panel::header(format!(
                "Your activity insights for the period:\n\n{}",
                self.time_range
            )))
            .with(Padding::new(1, 1, 0, 0))
            .with(Style::modern_rounded())
            .with(Modify::new(Columns::new(2..)).with(Alignment::right()))
            .with(Modify::new(Columns::new(0..=1)).with(Alignment::center()));

        let table = builder.build().with(table_config).to_string();
        write!(f, "{table}")?;

        Ok(())
    }
}

/// A group of activities for a summary category.
#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
#[getset(get = "pub")]
pub struct SummaryGroup {
    /// The total time spent on all activities within the group.
    total_duration: PaceDuration,

    /// The total time spent on breaks within the group.
    total_break_duration: PaceDuration,

    /// The groups of activities for a summary category
    activity_groups: Vec<ActivityGroup>,
}

impl SummaryGroup {
    /// Create a new summary group with the given activity groups.
    pub fn new(activity_groups: Vec<ActivityGroup>) -> Self {
        let total_duration = PaceDuration::from_seconds(
            activity_groups
                .iter()
                .map(|group| group.adjusted_duration().as_secs())
                .sum(),
        );

        let total_break_duration = PaceDuration::from_seconds(
            activity_groups
                .iter()
                .map(|group| group.intermission_duration().as_secs())
                .sum(),
        );

        Self {
            total_break_duration,
            total_duration,
            activity_groups,
        }
    }

    pub fn with_activity_group(activity_group: ActivityGroup) -> Self {
        Self {
            total_break_duration: *activity_group.intermission_duration(),
            total_duration: *activity_group.adjusted_duration(),
            activity_groups: vec![activity_group],
        }
    }

    /// Add an activity group to the summary group.
    pub fn add_activity_group(&mut self, activity_group: ActivityGroup) {
        self.total_duration += *activity_group.adjusted_duration();
        self.total_break_duration += *activity_group.intermission_duration();
        self.activity_groups.push(activity_group);
    }

    pub fn len(&self) -> usize {
        self.activity_groups.len()
    }

    pub fn is_empty(&self) -> bool {
        self.activity_groups.is_empty()
    }
}

/// A group of activities, the root activity and its intermissions.
#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
#[getset(get = "pub")]
pub struct ActivityGroup {
    /// A description of the activity group
    description: String,

    /// Root Activity within the activity group
    root_activity: ActivityItem,

    /// Duration spent on the grouped activities, essentially the sum of all durations
    /// of the activities within the group and their children. Intermissions are counting
    /// negatively towards the duration.
    adjusted_duration: PaceDuration,

    /// Intermissions within the activity group
    intermissions: Vec<ActivityItem>,

    /// The total duration of intermissions within the activity group
    intermission_duration: PaceDuration,
}

// TODO: Essentially a root activity and all intermissions should always have a duration, but we should
// TODO: handle the case where it doesn't.
impl ActivityGroup {
    pub fn new(root_activity: ActivityItem) -> Self {
        debug!("Creating new activity group");

        debug!("Root Activity: {:#?}", root_activity.activity());

        Self {
            description: root_activity.activity().description().to_owned(),
            adjusted_duration: root_activity.activity().duration().unwrap_or_default(),
            root_activity,
            ..Default::default()
        }
    }

    pub fn add_intermission(&mut self, intermission: ActivityItem) {
        debug!("Adding intermission to activity group");

        debug!("Intermission: {:#?}", intermission.activity());

        self.intermission_duration += intermission.activity().duration().unwrap_or_default();
        self.adjusted_duration -= intermission.activity().duration().unwrap_or_default();
        self.intermissions.push(intermission);
    }

    pub fn add_multiple_intermissions(&mut self, intermissions: Vec<ActivityItem>) {
        debug!("Adding multiple intermissions to activity group");

        for intermission in intermissions {
            self.add_intermission(intermission);
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
