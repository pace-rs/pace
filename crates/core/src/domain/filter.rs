use crate::domain::activity_log::ActivityLog;

/// Filter for activities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivityFilter {
    /// All activities
    #[default]
    All,

    /// Active, currently running activities
    Active,

    /// Archived activities
    Archived,

    /// Activities that have ended
    Ended,
}

/// Filtered activities
#[derive(Debug, Clone)]
pub enum FilteredActivities {
    /// All activities
    All(ActivityLog),

    /// Active, currently running activities
    Active(ActivityLog),

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
            Self::All(activities)
            | Self::Active(activities)
            | Self::Archived(activities)
            | Self::Ended(activities) => activities,
        }
    }
}
