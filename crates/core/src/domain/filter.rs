use crate::domain::activity_log::ActivityLog;
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
    Everything(ActivityLog),

    /// Only activities, no intermissions
    OnlyActivities(ActivityLog),

    /// Active, currently running activities
    Active(ActivityLog),

    /// Active, currently running activities
    ActiveIntermission(ActivityLog),

    /// Archived activities
    Archived(ActivityLog),

    /// Activities that have ended
    Ended(ActivityLog),
}

impl FilteredActivities {
    /// Convert the filtered activities into an activity log
    #[must_use]
    pub fn into_log(self) -> ActivityLog {
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
