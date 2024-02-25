use crate::ActivityGuid;
use strum::EnumIter;

/// Filter for activities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, EnumIter)]
pub enum ActivityFilter {
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
            | Self::ActiveIntermission(activities) => activities,
        }
    }
}
