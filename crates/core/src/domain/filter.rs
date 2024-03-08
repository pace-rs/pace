use crate::{ActivityGuid, ReviewCommandOptions, TimeRangeOptions};
use getset::{Getters, MutGetters, Setters};
use serde_derive::Serialize;
use strum::EnumIter;
use typed_builder::TypedBuilder;

/// Filter for activities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, EnumIter)]
pub enum ActivityFilterKind {
    /// Everything, activities, intermissions, archived, and ended
    #[default]
    Everything,

    /// Only activities, no intermissions
    OnlyActivities,

    /// Active, currently running activities
    Active,

    /// Active, currently running intermissions
    ActiveIntermission,

    /// Archived activities
    Archived,

    /// Activities that have ended
    Ended,

    /// Activities that are held
    Held,

    /// Intermission
    Intermission,

    /// Time range
    TimeRange(TimeRangeOptions),
}

/// Filtered activities
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilteredActivities {
    /// Everything, activities, intermissions, archived, and ended
    Everything(Vec<ActivityGuid>),

    /// Only activities, no intermissions
    OnlyActivities(Vec<ActivityGuid>),

    /// Active, currently running activities
    Active(Vec<ActivityGuid>),

    /// Active, currently running activities
    ActiveIntermission(Vec<ActivityGuid>),

    /// Archived activities
    Archived(Vec<ActivityGuid>),

    /// Activities that have ended
    Ended(Vec<ActivityGuid>),

    /// Activities that are held
    Held(Vec<ActivityGuid>),

    /// Intermission
    Intermission(Vec<ActivityGuid>),

    /// Time range
    TimeRange(Vec<ActivityGuid>),
}

impl FilteredActivities {
    /// Convert the filtered activities into a vector of activity GUIDs
    #[must_use]
    pub fn into_vec(self) -> Vec<ActivityGuid> {
        match self {
            Self::Everything(activities)
            | Self::OnlyActivities(activities)
            | Self::Active(activities)
            | Self::Archived(activities)
            | Self::Ended(activities)
            | Self::ActiveIntermission(activities)
            | Self::Held(activities)
            | Self::Intermission(activities)
            | Self::TimeRange(activities) => activities,
        }
    }
}

#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
#[getset(get = "pub")]
pub struct FilterOptions {
    category: Option<String>,
    case_sensitive: bool,
}

impl From<ReviewCommandOptions> for FilterOptions {
    fn from(options: ReviewCommandOptions) -> Self {
        Self {
            category: options.category().clone(),
            case_sensitive: *options.case_sensitive(),
        }
    }
}
impl From<&ReviewCommandOptions> for FilterOptions {
    fn from(options: &ReviewCommandOptions) -> Self {
        Self {
            category: options.category().clone(),
            case_sensitive: *options.case_sensitive(),
        }
    }
}
