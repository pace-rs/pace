use std::sync::{Arc, Mutex};

use chrono::{Local, NaiveDateTime, SubsecRound};
use rayon::prelude::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};

use crate::{
    domain::{
        activity::{Activity, ActivityEndOptions, ActivityGuid},
        activity_log::ActivityLog,
        filter::{ActivityFilter, FilteredActivities},
        time::calculate_duration,
    },
    error::{ActivityLogErrorKind, PaceOptResult, PaceResult},
    storage::{
        ActivityQuerying, ActivityReadOps, ActivityStateManagement, ActivityStorage,
        ActivityWriteOps, SyncStorage,
    },
    ActivityKind, ActivityKindOptions,
};

/// Type for shared `ActivityLog`
type SharedActivityLog = Arc<Mutex<ActivityLog>>;

/// In-memory storage for activities
#[derive(Debug, Clone)]
pub struct InMemoryActivityStorage {
    activities: SharedActivityLog,
}

impl From<ActivityLog> for InMemoryActivityStorage {
    fn from(activities: ActivityLog) -> Self {
        Self::new_with_activity_log(activities)
    }
}

impl InMemoryActivityStorage {
    /// Create a new `InMemoryActivityStorage`
    #[must_use]
    pub fn new() -> Self {
        Self {
            activities: Arc::new(Mutex::new(ActivityLog::default())),
        }
    }

    /// Creates a new `InMemoryActivityStorage` from an `ActivityLog`
    ///
    /// # Arguments
    ///
    /// * `activity_log` - The `ActivityLog` to use
    ///
    /// # Returns
    ///
    /// A new `InMemoryActivityStorage` with the given `ActivityLog`
    pub fn new_with_activity_log(activity_log: ActivityLog) -> Self {
        Self {
            activities: Arc::new(Mutex::new(activity_log)),
        }
    }

    /// Try to convert the `InMemoryActivityStorage` into an `ActivityLog`
    ///
    /// # Errors
    ///
    /// Returns an error if the mutex has been poisoned
    pub fn get_activity_log(&self) -> PaceResult<ActivityLog> {
        let Ok(activity_log) = self.activities.lock() else {
            return Err(ActivityLogErrorKind::MutexHasBeenPoisoned.into());
        };

        Ok(activity_log.clone())
    }
}

impl Default for InMemoryActivityStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivityStorage for InMemoryActivityStorage {
    fn setup_storage(&self) -> PaceResult<()> {
        Ok(())
    }
}

impl SyncStorage for InMemoryActivityStorage {
    fn sync(&self) -> PaceResult<()> {
        Ok(())
    }
}

impl ActivityReadOps for InMemoryActivityStorage {
    fn read_activity(&self, activity_id: ActivityGuid) -> PaceResult<Activity> {
        let Ok(activities) = self.activities.lock() else {
            return Err(ActivityLogErrorKind::MutexHasBeenPoisoned.into());
        };

        let activity = activities
            .activities()
            .par_iter()
            .find_first(|activity| {
                activity
                    .guid()
                    .as_ref()
                    .map_or(false, |orig_activity_id| *orig_activity_id == activity_id)
            })
            .cloned()
            .ok_or(ActivityLogErrorKind::ActivityNotFound(activity_id))?;

        drop(activities);

        Ok(activity)
    }

    fn list_activities(&self, filter: ActivityFilter) -> PaceOptResult<FilteredActivities> {
        let Ok(activities) = self.activities.lock() else {
            return Err(ActivityLogErrorKind::MutexHasBeenPoisoned.into());
        };

        let filtered = activities
            .activities()
            .iter()
            .filter(|activity| match filter {
                ActivityFilter::Active => activity.is_active(),
                ActivityFilter::ActiveIntermission => activity.is_active_intermission(),
                ActivityFilter::Ended => activity.has_ended(),
                ActivityFilter::All => true,
                ActivityFilter::Archived => false, // TODO: Implement archived filter
            })
            .cloned()
            .collect::<ActivityLog>();

        if filtered.activities().is_empty() {
            return Ok(None);
        }

        drop(activities);

        match filter {
            ActivityFilter::All => Ok(Some(FilteredActivities::All(filtered.clone()))),
            ActivityFilter::Active => Ok(Some(FilteredActivities::Active(filtered.clone()))),
            ActivityFilter::ActiveIntermission => Ok(Some(FilteredActivities::ActiveIntermission(
                filtered.clone(),
            ))),
            ActivityFilter::Archived => Ok(Some(FilteredActivities::Archived(filtered.clone()))),
            ActivityFilter::Ended => Ok(Some(FilteredActivities::Ended(filtered.clone()))),
        }
    }
}

impl ActivityWriteOps for InMemoryActivityStorage {
    fn create_activity(&self, activity: Activity) -> PaceResult<ActivityGuid> {
        let Ok(mut activities) = self.activities.lock() else {
            return Err(ActivityLogErrorKind::MutexHasBeenPoisoned.into());
        };

        let Some(activity_id) = activity.guid() else {
            return Err(ActivityLogErrorKind::ActivityIdNotSet.into());
        };

        // Search for the activity in the list of activities to see if the ID is already in use.
        if activities.activities().par_iter().any(|activity| {
            activity
                .guid()
                .as_ref()
                .map_or(false, |id| id == activity_id)
        }) {
            return Err(ActivityLogErrorKind::ActivityIdAlreadyInUse(*activity_id).into());
        }

        activities.activities_mut().push_front(activity.clone());

        drop(activities);

        Ok(*activity_id)
    }

    fn update_activity(
        &self,
        activity_id: ActivityGuid,
        mut activity: Activity,
    ) -> PaceResult<Activity> {
        // First things, first. Replace new activity's id with the original ID we are looking for.
        // To make sure we are not accidentally changing the ID.
        let _ = activity.guid_mut().replace(activity_id);

        let Ok(mut activities) = self.activities.lock() else {
            return Err(ActivityLogErrorKind::MutexHasBeenPoisoned.into());
        };

        let og_activity = activities
            .activities_mut()
            .par_iter_mut()
            .find_first(|activity| {
                activity
                    .guid()
                    .as_ref()
                    .map_or(false, |orig_activity_id| *orig_activity_id == activity_id)
            })
            .ok_or(ActivityLogErrorKind::ActivityNotFound(activity_id))?;

        let original_activity = og_activity.clone();

        *og_activity = activity;

        drop(activities);

        Ok(original_activity)
    }

    fn delete_activity(&self, activity_id: ActivityGuid) -> PaceResult<Activity> {
        let Ok(mut activities) = self.activities.lock() else {
            return Err(ActivityLogErrorKind::MutexHasBeenPoisoned.into());
        };

        let activity_index = activities
            .activities_mut()
            .par_iter()
            .position_first(|activity| {
                activity
                    .guid()
                    .as_ref()
                    .map_or(false, |orig_activity_id| *orig_activity_id == activity_id)
            })
            .ok_or(ActivityLogErrorKind::ActivityNotFound(activity_id))?;

        let activity = activities
            .activities_mut()
            .remove(activity_index)
            .ok_or(ActivityLogErrorKind::ActivityCantBeRemoved(activity_index))?;

        drop(activities);

        Ok(activity)
    }
}

impl ActivityStateManagement for InMemoryActivityStorage {
    fn end_single_activity(
        &self,
        activity_id: ActivityGuid,
        end_time: Option<NaiveDateTime>,
    ) -> PaceResult<ActivityGuid> {
        let Ok(mut activities) = self.activities.lock() else {
            return Err(ActivityLogErrorKind::MutexHasBeenPoisoned.into());
        };

        let end_time = end_time.unwrap_or_else(|| Local::now().naive_local());

        let activity = activities
            .activities_mut()
            .par_iter_mut()
            .find_first(|activity| {
                activity
                    .guid()
                    .as_ref()
                    .map_or(false, |orig_activity_id| *orig_activity_id == activity_id)
            })
            .ok_or(ActivityLogErrorKind::ActivityNotFound(activity_id))?;

        let duration = calculate_duration(activity.begin(), end_time)?;

        let end_opts = ActivityEndOptions::new(end_time, duration);

        activity.end_activity(end_opts);

        drop(activities);

        Ok(activity_id)
    }

    fn end_last_unfinished_activity(
        &self,
        end_time: Option<NaiveDateTime>,
    ) -> PaceOptResult<Activity> {
        let Ok(mut activities) = self.activities.lock() else {
            return Err(ActivityLogErrorKind::MutexHasBeenPoisoned.into());
        };

        let end_time = end_time.unwrap_or_else(|| Local::now().naive_local().round_subsecs(0));

        let Some(last_unfinished_activity) = activities
            .activities_mut()
            .par_iter_mut()
            .find_first(|activity| activity.is_active())
        else {
            return Ok(None);
        };

        let duration = calculate_duration(last_unfinished_activity.begin(), end_time)?;

        let end_opts = ActivityEndOptions::new(end_time, duration);
        last_unfinished_activity.end_activity(end_opts);

        Ok(Some(last_unfinished_activity.clone()))
    }

    fn end_all_unfinished_activities(
        &self,
        end_time: Option<NaiveDateTime>,
    ) -> PaceOptResult<Vec<Activity>> {
        let mut ended_activities = vec![];

        let end_time = end_time.unwrap_or_else(|| Local::now().naive_local().round_subsecs(0));

        let Ok(mut activities) = self.activities.lock() else {
            return Err(ActivityLogErrorKind::MutexHasBeenPoisoned.into());
        };

        activities
            .activities_mut()
            .iter_mut()
            .filter(|activity| activity.is_active())
            .for_each(|activity| {
                match calculate_duration(activity.begin(), end_time) {
                    Ok(duration) => {
                        let end_opts = ActivityEndOptions::new(end_time, duration);
                        activity.end_activity(end_opts);

                        ended_activities.push(activity.clone());
                    }
                    Err(_) => {
                        log::warn!(
                            "Activity {} ends before it began. That's impossible. Skipping \
                             activity.",
                            activity
                        );
                    }
                };
            });

        drop(activities);

        if ended_activities.is_empty() {
            return Ok(None);
        }

        Ok(Some(ended_activities))
    }

    fn hold_last_unfinished_activity(
        &self,
        hold_time: Option<NaiveDateTime>,
    ) -> PaceOptResult<Activity> {
        let time = hold_time.unwrap_or_else(|| Local::now().naive_local().round_subsecs(0));

        // Get id from last activity that is not ended
        let Some(active_activity) = self.latest_active_activity()? else {
            // There are no active activities
            return Ok(None);
        };

        // Check if the latest active activity is already having an intermission
        // TODO: Refactor, that should be way easier to do
        if let Some(intermissions) = self.list_active_intermissions()? {
            if intermissions.iter().any(|intermission| {
                intermission
                    .activity_kind_options()
                    .as_ref()
                    .map(|kind| {
                        let Some(parent_id) = kind.parent_id() else {
                            return false;
                        };

                        let Some(activity_id) = active_activity.guid() else {
                            return false;
                        };

                        parent_id == activity_id
                    })
                    .unwrap_or(false)
            }) {
                // There is already an intermission for an active activity
                return Ok(None);
            }
        };

        let Some(parent_id) = active_activity.guid() else {
            return Err(ActivityLogErrorKind::ActivityIdNotSet.into());
        };

        let activity_kind_opts = ActivityKindOptions::new(*parent_id);

        let intermission = Activity::builder()
            .begin(time)
            .kind(ActivityKind::Intermission)
            .description(
                active_activity
                    .description()
                    .clone()
                    .unwrap_or_else(|| format!("Holding {active_activity}")),
            )
            .category(active_activity.category().clone())
            .activity_kind_options(activity_kind_opts)
            .build();

        let id = self.create_activity(intermission.clone())?;

        if id
            != intermission
                .guid()
                .ok_or_else(|| ActivityLogErrorKind::ActivityIdNotSet)?
        {
            return Err(ActivityLogErrorKind::ActivityIdMismatch(
                id,
                intermission
                    .guid()
                    .expect("ID for activity should be existing at this point."),
            )
            .into());
        }

        Ok(Some(active_activity))
    }

    fn end_all_active_intermissions(
        &self,
        end_time: Option<NaiveDateTime>,
    ) -> PaceOptResult<Vec<Activity>> {
        let mut ended_intermissions = vec![];

        let end_time = end_time.unwrap_or_else(|| Local::now().naive_local().round_subsecs(0));

        let Ok(mut activities) = self.activities.lock() else {
            return Err(ActivityLogErrorKind::MutexHasBeenPoisoned.into());
        };

        activities
            .activities_mut()
            .iter_mut()
            .filter(|activity| activity.is_active_intermission())
            .for_each(|activity| {
                match calculate_duration(activity.begin(), end_time) {
                    Ok(duration) => {
                        let end_opts = ActivityEndOptions::new(end_time, duration);
                        activity.end_activity(end_opts);

                        ended_intermissions.push(activity.clone());
                    }
                    Err(_) => {
                        log::warn!(
                            "Activity {} ends before it began. That's impossible. Skipping \
                                 activity.",
                            activity
                        );
                    }
                };
            });

        drop(activities);

        if ended_intermissions.is_empty() {
            return Ok(None);
        }

        Ok(Some(ended_intermissions))
    }
}

impl ActivityQuerying for InMemoryActivityStorage {
    fn find_activities_in_date_range(
        &self,
        _start_date: chrono::prelude::NaiveDate,
        _end_date: chrono::prelude::NaiveDate,
    ) -> PaceResult<ActivityLog> {
        todo!("Implement find_activities_in_date_range for InMemoryActivityStorage")
    }

    fn list_activities_by_id(
        &self,
    ) -> PaceOptResult<std::collections::BTreeMap<ActivityGuid, Activity>> {
        todo!("Implement list_activities_by_id for InMemoryActivityStorage")
    }
}
