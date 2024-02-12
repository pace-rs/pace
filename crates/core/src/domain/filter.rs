use std::collections::VecDeque;

use crate::domain::activity::Activity;

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
    All(VecDeque<Activity>),
    Active(VecDeque<Activity>),
    Archived(VecDeque<Activity>),
    Ended(VecDeque<Activity>),
}

impl FilteredActivities {
    pub fn into_activities(self) -> VecDeque<Activity> {
        match self {
            FilteredActivities::All(activities)
            | FilteredActivities::Active(activities)
            | FilteredActivities::Archived(activities)
            | FilteredActivities::Ended(activities) => activities,
        }
    }
}
