use getset::{Getters, MutGetters, Setters};
use pace_time::{date_time::PaceDateTime, duration::PaceDuration, time_range::TimeRangeOptions};
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strum_macros::EnumString;
use tabled::{
    builder::Builder,
    settings::{object::Columns, Alignment, Modify, Padding, Panel, Settings, Style},
};

use typed_builder::TypedBuilder;

use crate::domain::activity::{ActivityGroup, ActivityItem, ActivityKind};

/// The kind of review format
/// Default: `console`
///
/// Options: `console`, `html`, `markdown`, `plain-text`
#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default, EnumString, PartialEq, Eq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ReflectionsFormatKind {
    #[default]
    Console,
    Template,
    Json,
    Csv,
}

/// Represents a category for summarizing activities.
// We use a string to allow for user-defined categories for now,
// but we may want to change this to an enum in the future.
pub type SummaryCategories = (String, String);

pub type SummaryGroupByCategory = BTreeMap<SummaryCategories, SummaryActivityGroup>;

/// Represents a summary of activities and insights for a specified review period.
#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct ReflectionSummary {
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

impl ReflectionSummary {
    #[must_use]
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
            time_range,
            total_time_spent,
            total_break_duration,
            summary_groups_by_category,
        }
    }
}

// TODO!: Refine the display of the review summary
impl std::fmt::Display for ReflectionSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut builder = Builder::new();

        builder.push_record(vec![
            "Category",
            "Description",
            "Duration (Sessions)",
            "Breaks (Amount)",
        ]);

        for ((category, subcategory), summary_group) in &self.summary_groups_by_category {
            builder.push_record(vec![
                category,
                "",
                &summary_group.total_duration().to_string(),
                &summary_group.total_break_duration().to_string(),
            ]);

            for (description, activity_group) in summary_group.activity_groups_by_description() {
                builder.push_record(vec![
                    subcategory,
                    description,
                    format!(
                        "{} ({})",
                        &activity_group.adjusted_duration().to_string(),
                        &activity_group.activity_sessions().len()
                    )
                    .as_str(),
                    format!(
                        "{} ({})",
                        &activity_group.intermission_duration().to_string(),
                        &activity_group.intermission_count().to_string()
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
pub struct SummaryActivityGroup {
    /// The total time spent on all activities within the group.
    total_duration: PaceDuration,

    /// The total time spent on breaks within the group.
    total_break_duration: PaceDuration,

    /// The total amount of breaks within the group.
    total_break_count: usize,

    /// The groups of activities for a summary category
    activity_groups_by_description: BTreeMap<String, ActivityGroup>,
}

impl SummaryActivityGroup {
    #[must_use]
    pub fn with_activity_group(activity_group: ActivityGroup) -> Self {
        Self {
            total_break_count: *activity_group.intermission_count(),
            total_break_duration: *activity_group.intermission_duration(),
            total_duration: *activity_group.adjusted_duration(),
            activity_groups_by_description: BTreeMap::from([(
                activity_group.description().to_owned(),
                activity_group,
            )]),
        }
    }

    /// Add an activity group to the summary group.
    pub fn add_activity_group(&mut self, activity_group: ActivityGroup) {
        self.total_duration += *activity_group.adjusted_duration();

        self.total_break_duration += *activity_group.intermission_duration();

        _ = self
            .activity_groups_by_description
            .entry(activity_group.description().to_owned())
            .or_insert(activity_group);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.activity_groups_by_description.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.activity_groups_by_description.is_empty()
    }
}

/// Highlights from the review period, providing quick insights into key metrics.
#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
#[allow(clippy::struct_field_names)]
pub struct Highlights {
    /// The day with the highest productive hours.
    pub most_productive_day: PaceDateTime,

    /// The kind of activity most frequently logged.
    pub most_frequent_activity_kind: ActivityKind,

    /// The category or activity where the most time was spent.
    pub most_time_spent_on: ActivityItem,
}
