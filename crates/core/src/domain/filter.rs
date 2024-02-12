use crate::domain::activity::{Activity, ActivityLog};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivityFilter {
    #[default]
    All,
    Active,
    Archived,
    Ended,
}

#[derive(Debug, Clone)]
pub enum FilteredActivities {
    All(ActivityLog),
    Active(ActivityLog),
    Archived(ActivityLog),
    Ended(ActivityLog),
}

impl FilteredActivities {
    pub fn into_log(self) -> ActivityLog {
        match self {
            FilteredActivities::All(activities)
            | FilteredActivities::Active(activities)
            | FilteredActivities::Archived(activities)
            | FilteredActivities::Ended(activities) => activities,
        }
    }
}
