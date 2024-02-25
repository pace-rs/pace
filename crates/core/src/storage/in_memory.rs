use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use chrono::NaiveDateTime;
use merge::Merge;
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
    ActivityKind, ActivityKindOptions, EndOptions, HoldOptions,
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
                ActivityFilter::Everything => true,
                ActivityFilter::OnlyActivities => activity.kind().is_activity(),
                ActivityFilter::Active => activity.is_active(),
                ActivityFilter::ActiveIntermission => activity.is_active_intermission(),
                ActivityFilter::Ended => activity.has_ended(),
                ActivityFilter::Archived => activity.is_archived(),
            })
            .cloned()
            .collect::<ActivityLog>();

        drop(activities);

        if filtered.activities().is_empty() {
            return Ok(None);
        }

        match filter {
            ActivityFilter::Everything => {
                Ok(Some(FilteredActivities::Everything(filtered.clone())))
            }
            ActivityFilter::OnlyActivities => {
                Ok(Some(FilteredActivities::OnlyActivities(filtered.clone())))
            }
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
        updated_activity: Activity,
    ) -> PaceResult<Activity> {
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

        og_activity.merge(updated_activity);

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
        end_opts: EndOptions,
    ) -> PaceResult<ActivityGuid> {
        let Ok(mut activities) = self.activities.lock() else {
            return Err(ActivityLogErrorKind::MutexHasBeenPoisoned.into());
        };

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

        let duration = calculate_duration(activity.begin(), *end_opts.end_time())?;

        let end_opts = ActivityEndOptions::new(*end_opts.end_time(), duration);

        activity.end_activity(end_opts);

        drop(activities);

        Ok(activity_id)
    }

    fn end_last_unfinished_activity(&self, end_opts: EndOptions) -> PaceOptResult<Activity> {
        let Ok(mut activities) = self.activities.lock() else {
            return Err(ActivityLogErrorKind::MutexHasBeenPoisoned.into());
        };

        let Some(last_unfinished_activity) = activities
            .activities_mut()
            .par_iter_mut()
            .find_first(|activity| activity.is_active())
        else {
            return Ok(None);
        };

        let duration = calculate_duration(last_unfinished_activity.begin(), *end_opts.end_time())?;

        let end_opts = ActivityEndOptions::new(*end_opts.end_time(), duration);
        last_unfinished_activity.end_activity(end_opts);

        Ok(Some(last_unfinished_activity.clone()))
    }

    fn end_all_unfinished_activities(&self, end_opts: EndOptions) -> PaceOptResult<Vec<Activity>> {
        let mut ended_activities = vec![];

        let Ok(mut activities) = self.activities.lock() else {
            return Err(ActivityLogErrorKind::MutexHasBeenPoisoned.into());
        };

        activities
            .activities_mut()
            .iter_mut()
            .filter(|activity| activity.is_active())
            .for_each(|activity| {
                match calculate_duration(activity.begin(), *end_opts.end_time()) {
                    Ok(duration) => {
                        let end_opts = ActivityEndOptions::new(*end_opts.end_time(), duration);
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

    fn hold_last_unfinished_activity(&self, hold_opts: HoldOptions) -> PaceOptResult<Activity> {
        // Get id from last activity that is not ended
        let Some(active_activity) = self.most_recent_active_activity()? else {
            // There are no active activities
            return Ok(None);
        };

        let Some(active_guid) = active_activity.guid().as_ref() else {
            return Err(ActivityLogErrorKind::ActivityIdNotSet.into());
        };

        // Check if the latest active activity is already having an intermission
        if let Some(intermissions) = self.list_active_intermissions_for_activity_id(*active_guid)? {
            // If there are active intermissions and we want to extend return early with the active activity
            //
            // Handles the case, if someone wants to create an intermission for an
            // activity that already has an intermission, but hasn't set that we should
            // create a new intermission. In this case we don't want to create
            // another intermission, but return with the active activity.
            if !intermissions.is_empty() && hold_opts.action().is_extend() {
                return Ok(Some(active_activity));
            }
        };

        // If there are active intermissions for any activity, end the intermissions
        // because the user wants to create a new intermission
        let _ = self.end_all_active_intermissions(hold_opts.clone().into())?;

        // Create a new intermission for the active activity
        let activity_kind_opts = ActivityKindOptions::with_parent_id(*active_guid);

        let description = hold_opts.reason().clone().unwrap_or_else(|| {
            active_activity
                .description()
                .clone()
                .unwrap_or_else(|| format!("Holding {active_activity}"))
        });

        let intermission = Activity::builder()
            .begin(*hold_opts.begin_time())
            .kind(ActivityKind::Intermission)
            .description(description)
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

    fn end_all_active_intermissions(&self, end_opts: EndOptions) -> PaceOptResult<Vec<Activity>> {
        let mut ended_intermissions = vec![];

        let Ok(mut activities) = self.activities.lock() else {
            return Err(ActivityLogErrorKind::MutexHasBeenPoisoned.into());
        };

        activities
            .activities_mut()
            .iter_mut()
            .filter(|activity| activity.is_active_intermission())
            .for_each(|activity| {
                match calculate_duration(activity.begin(), *end_opts.end_time()) {
                    Ok(duration) => {
                        let end_opts = ActivityEndOptions::new(*end_opts.end_time(), duration);
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

    fn resume_activity(
        &self,
        _activity_id: Option<ActivityGuid>,
        _resume_time: Option<NaiveDateTime>,
    ) -> PaceOptResult<Activity> {
        // What do we need to do here?
        // - Find the activity by id, if it's not given, find the last active activity
        // - If there are active intermissions for any activity, end the intermissions
        //   and resume the activity with the same id as the most recent intermission's parent_id
        // - If there are no active intermissions, but there are active activities, return the last active activity
        // - If there are no active intermissions, resume the activity with the given id or the last active activity
        // - If there are no active activities, return an error

        todo!("Implement resume_activity for InMemoryActivityStorage")
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

    fn list_activities_by_id(&self) -> PaceOptResult<BTreeMap<ActivityGuid, Activity>> {
        let Ok(activities) = self.activities.lock() else {
            return Err(ActivityLogErrorKind::MutexHasBeenPoisoned.into());
        };

        let activities_by_id = activities
            .activities()
            .par_iter()
            .map(|activity| {
                let id = activity
                    .guid()
                    .ok_or_else(|| ActivityLogErrorKind::ActivityIdNotSet)?;

                Ok((id, activity.clone()))
            })
            .collect::<PaceResult<BTreeMap<ActivityGuid, Activity>>>()?;

        drop(activities);

        if activities_by_id.is_empty() {
            return Ok(None);
        }

        Ok(Some(activities_by_id))
    }
}

#[cfg(test)]
mod tests {

    use chrono::Local;

    use crate::PaceDateTime;

    use super::*;

    #[test]
    fn test_in_memory_activity_storage_passes() {
        let storage = InMemoryActivityStorage::new();

        assert_eq!(
            storage.get_activity_log().unwrap().activities().len(),
            0,
            "Activity log is not empty."
        );
    }

    #[test]
    fn test_in_memory_activity_storage_from_activity_log_passes() {
        let activity_log = ActivityLog::default();
        let storage = InMemoryActivityStorage::from(activity_log);

        assert_eq!(
            storage.get_activity_log().unwrap().activities().len(),
            0,
            "Activity log is not empty."
        );
    }

    #[test]
    fn test_create_same_activity_twice_fails() {
        let storage = InMemoryActivityStorage::new();

        let begin = Local::now().naive_local();
        let kind = ActivityKind::Activity;
        let description = "Test activity";

        let activity = Activity::builder()
            .begin(begin)
            .kind(kind)
            .description(description)
            .build();

        let id = storage.create_activity(activity.clone()).unwrap();

        assert_eq!(id, activity.guid().unwrap());
        assert_eq!(
            storage.get_activity_log().unwrap().activities().len(),
            1,
            "Activity was not created."
        );

        let create_same_activity_result = storage.create_activity(activity);

        assert!(
            create_same_activity_result.is_err(),
            "Activity was created twice."
        );
    }

    #[test]
    fn test_create_read_activity_passes() {
        let storage = InMemoryActivityStorage::new();

        let begin = Local::now().naive_local();
        let kind = ActivityKind::Activity;
        let description = "Test activity";

        let activity = Activity::builder()
            .begin(begin)
            .kind(kind)
            .description(description)
            .build();

        let id = storage.create_activity(activity.clone()).unwrap();

        assert_eq!(id, activity.guid().unwrap());
        assert_eq!(
            storage.get_activity_log().unwrap().activities().len(),
            1,
            "Activity was not created."
        );

        let stored_activity = storage.read_activity(id).unwrap();

        assert_eq!(
            activity, stored_activity,
            "Stored activity is not the same as the original activity."
        );
    }

    #[test]
    fn test_list_activities_passes() {
        let storage = InMemoryActivityStorage::new();

        let begin = Local::now().naive_local();
        let kind = ActivityKind::Activity;
        let description = "Test activity";

        let activity = Activity::builder()
            .begin(begin)
            .kind(kind)
            .description(description)
            .build();

        let _ = storage.create_activity(activity.clone()).unwrap();

        let filtered_activities = storage.list_activities(ActivityFilter::Everything).unwrap();

        assert_eq!(
            filtered_activities,
            Some(FilteredActivities::Everything(ActivityLog::from_iter(
                vec![activity.clone()]
            ))),
            "Filtered activities are not the same as the original activity."
        );
    }

    #[test]
    fn test_update_activity_passes() {
        let storage = InMemoryActivityStorage::new();

        let begin = Local::now().naive_local();
        let kind = ActivityKind::Activity;
        let description = "Test activity";

        let og_activity = Activity::builder()
            .begin(begin)
            .kind(kind)
            .description(description)
            .build();

        let id = storage.create_activity(og_activity.clone()).unwrap();

        let read_activity = storage.read_activity(id).unwrap();

        assert_eq!(
            og_activity, read_activity,
            "Stored activity is not the same as the original activity."
        );

        let new_description = "Updated description";

        let updated_activity = Activity::builder()
            .begin(begin + chrono::Duration::seconds(30))
            .kind(ActivityKind::PomodoroWork)
            .description(new_description)
            .build();

        let old_activity = storage
            .update_activity(id, updated_activity.clone())
            .unwrap();

        assert_eq!(
            old_activity, og_activity,
            "Stored activity is not the same as the original activity."
        );

        let new_stored_activity = storage.read_activity(id).unwrap();

        assert_eq!(
            og_activity.guid().unwrap(),
            new_stored_activity.guid().unwrap(),
            "ID was updated, but shouldn't."
        );

        assert_eq!(
            new_stored_activity.description().as_deref(),
            Some(new_description),
            "Description was not updated."
        );

        assert_eq!(
            og_activity.kind(),
            new_stored_activity.kind(),
            "Kind was updated, but shouldn't."
        );

        assert_eq!(
            og_activity.begin(),
            new_stored_activity.begin(),
            "Begin time was updated, but shouldn't."
        );

        assert!(
            new_stored_activity.active(),
            "Activity should be active now, but was not updated."
        );
    }

    #[test]
    fn test_crud_activity_passes() {
        let storage = InMemoryActivityStorage::new();

        // Create activity
        let begin = Local::now().naive_local();
        let kind = ActivityKind::Activity;
        let description = "Test activity";

        let activity = Activity::builder()
            .begin(begin)
            .kind(kind)
            .description(description)
            .build();

        assert_eq!(
            storage.get_activity_log().unwrap().activities().len(),
            0,
            "Activity log is not empty."
        );

        let id = storage.create_activity(activity.clone()).unwrap();

        assert_eq!(id, activity.guid().unwrap());
        assert_eq!(
            storage.get_activity_log().unwrap().activities().len(),
            1,
            "Activity was not created."
        );

        // Read activity
        let stored_activity = storage.read_activity(id).unwrap();

        assert_eq!(
            activity, stored_activity,
            "Stored activity is not the same as the original activity."
        );

        // Update activity
        let new_description = "Updated description";

        let updated_activity = Activity::builder()
            .begin(begin + chrono::Duration::seconds(30))
            .kind(ActivityKind::PomodoroWork)
            .description(new_description)
            .build();

        let _ = storage
            .update_activity(id, updated_activity.clone())
            .unwrap();

        let new_stored_activity = storage.read_activity(id).unwrap();

        assert_eq!(
            new_stored_activity.description().as_deref(),
            Some(new_description),
            "Description was not updated."
        );

        assert_eq!(
            stored_activity.kind(),
            new_stored_activity.kind(),
            "Kind was updated, but shouldn't."
        );

        assert_eq!(
            stored_activity.begin(),
            new_stored_activity.begin(),
            "Begin time was updated, but shouldn't."
        );

        assert!(
            new_stored_activity.active(),
            "Activity should be active now, but was not updated."
        );

        // Delete activity
        let deleted_activity = storage.delete_activity(id).unwrap();

        assert_eq!(
            storage.get_activity_log().unwrap().activities().len(),
            0,
            "Activity was not deleted."
        );

        assert_eq!(
            deleted_activity, new_stored_activity,
            "Deleted activity is not the same as the updated activity."
        );

        // Try to read the deleted activity

        let read_deleted_activity_result = storage.read_activity(id);

        assert!(
            read_deleted_activity_result.is_err(),
            "Deleted activity was read."
        );
    }

    #[test]
    fn test_end_single_activity_passes() {
        let storage = InMemoryActivityStorage::new();
        let now = Local::now().naive_local();
        let begin_time = now - chrono::Duration::seconds(30);
        let end_time = now + chrono::Duration::seconds(30);
        let kind = ActivityKind::Activity;
        let description = "Test activity";

        let activity = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .build();

        let id = storage.create_activity(activity.clone()).unwrap();

        let end_opts = EndOptions::builder().end_time(end_time).build();

        let ended_activity_id = storage.end_single_activity(id, end_opts).unwrap();

        assert_eq!(ended_activity_id, id, "Activity IDs do not match.");

        let ended_activity = storage.read_activity(id).unwrap();

        assert!(
            ended_activity.has_ended(),
            "Activity has not ended, but should have."
        );

        assert_eq!(
            ended_activity
                .activity_end_options()
                .as_ref()
                .unwrap()
                .end(),
            &PaceDateTime::new(end_time),
            "End time was not set."
        );
    }

    #[test]
    fn test_end_last_unfinished_activity_passes() {
        let storage = InMemoryActivityStorage::new();
        let now = Local::now().naive_local();
        let begin_time = now - chrono::Duration::seconds(30);
        let kind = ActivityKind::Activity;
        let description = "Test activity";

        let activity = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .build();

        let id = storage.create_activity(activity.clone()).unwrap();

        let ended_activity = storage
            .end_last_unfinished_activity(EndOptions::builder().end_time(now).build())
            .unwrap();

        assert_eq!(
            ended_activity.as_ref().unwrap().guid().unwrap(),
            id,
            "Activity IDs do not match."
        );

        assert!(
            ended_activity.as_ref().unwrap().has_ended(),
            "Activity has not ended, but should have."
        );

        assert_eq!(
            ended_activity
                .as_ref()
                .unwrap()
                .activity_end_options()
                .as_ref()
                .unwrap()
                .end(),
            &PaceDateTime::new(now),
            "End time was not set."
        );
    }

    #[test]
    fn test_end_all_unfinished_activities_for_multiple_activities_passes() {
        let storage = InMemoryActivityStorage::new();
        let now = Local::now().naive_local();
        let begin_time = now - chrono::Duration::seconds(30);
        let kind = ActivityKind::Activity;
        let description = "Test activity";

        let activity = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .build();

        let id = storage.create_activity(activity.clone()).unwrap();

        let begin_time = now - chrono::Duration::seconds(60);
        let kind = ActivityKind::Activity;
        let description = "Test activity 2";

        let activity2 = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .build();

        let id2 = storage.create_activity(activity2.clone()).unwrap();

        let ended_activities = storage
            .end_all_unfinished_activities(EndOptions::builder().end_time(now).build())
            .unwrap();

        assert_eq!(
            ended_activities.as_ref().unwrap().len(),
            2,
            "Not all activities were ended."
        );

        assert!(
            ended_activities
                .as_ref()
                .unwrap()
                .iter()
                .all(|activity| activity.has_ended()),
            "Not all activities have ended."
        );

        let ended_activity = storage.read_activity(id).unwrap();

        assert!(
            ended_activity.has_ended(),
            "Activity has not ended, but should have."
        );

        assert_eq!(
            ended_activity
                .activity_end_options()
                .as_ref()
                .unwrap()
                .end(),
            &PaceDateTime::new(now),
            "End time was not set."
        );

        let ended_activity2 = storage.read_activity(id2).unwrap();

        assert!(
            ended_activity2.has_ended(),
            "Activity has not ended, but should have."
        );

        assert_eq!(
            ended_activity2
                .activity_end_options()
                .as_ref()
                .unwrap()
                .end(),
            &PaceDateTime::new(now),
            "End time was not set."
        );
    }

    #[test]
    fn test_hold_last_unfinished_activity_passes() {
        let storage = InMemoryActivityStorage::new();
        let now = Local::now().naive_local();
        let begin_time = now - chrono::Duration::seconds(30);
        let kind = ActivityKind::Activity;
        let description = "Test activity";

        let activity = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .build();

        let id = storage.create_activity(activity.clone()).unwrap();

        let hold_time = now + chrono::Duration::seconds(30);

        let hold_opts = HoldOptions::builder().begin_time(hold_time).build();

        let held_activity = storage.hold_last_unfinished_activity(hold_opts).unwrap();

        assert_eq!(
            held_activity.as_ref().unwrap().guid().unwrap(),
            id,
            "Activity IDs do not match."
        );

        let intermission_guids = storage
            .list_active_intermissions_for_activity_id(id)
            .unwrap();

        assert_eq!(
            intermission_guids.as_ref().unwrap().len(),
            1,
            "Intermission was not created."
        );

        let intermission = storage
            .read_activity(intermission_guids.as_ref().unwrap()[0])
            .unwrap();

        assert_eq!(
            intermission
                .activity_kind_options()
                .as_ref()
                .unwrap()
                .parent_id()
                .unwrap(),
            id,
            "Parent ID is not set."
        );
    }

    #[test]
    fn test_hold_last_unfinished_activity_with_existing_intermission_does_nothing_passes() {
        let storage = InMemoryActivityStorage::new();
        let now = Local::now().naive_local();
        let begin_time = now - chrono::Duration::seconds(30);
        let kind = ActivityKind::Activity;
        let description = "Test activity";

        let activity = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .build();

        let id = storage.create_activity(activity.clone()).unwrap();

        let hold_opts = HoldOptions::builder()
            .begin_time(now + chrono::Duration::seconds(30))
            .build();

        let _ = storage.hold_last_unfinished_activity(hold_opts).unwrap();

        let intermission_guids = storage
            .list_active_intermissions_for_activity_id(id)
            .unwrap();

        assert_eq!(
            intermission_guids.as_ref().unwrap().len(),
            1,
            "Intermission was not created."
        );

        let hold_opts = HoldOptions::builder()
            .begin_time(now + chrono::Duration::seconds(60))
            .build();

        let held_activity = storage.hold_last_unfinished_activity(hold_opts).unwrap();

        assert_eq!(
            held_activity.as_ref().unwrap().guid().unwrap(),
            id,
            "Activity IDs do not match."
        );

        let intermission_guids = storage
            .list_active_intermissions_for_activity_id(id)
            .unwrap();

        assert_eq!(
            intermission_guids.as_ref().unwrap().len(),
            1,
            "Intermission was created again."
        );
    }

    #[test]
    fn test_end_all_active_intermissions_passes() {
        let storage = InMemoryActivityStorage::new();
        let now = Local::now().naive_local();
        let begin_time = now - chrono::Duration::seconds(30);
        let end_time = now + chrono::Duration::seconds(60);
        let kind = ActivityKind::Activity;
        let description = "Test activity";

        let activity = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .build();

        let id = storage.create_activity(activity.clone()).unwrap();

        let hold_opts = HoldOptions::builder()
            .begin_time(now + chrono::Duration::seconds(30))
            .build();

        let _ = storage.hold_last_unfinished_activity(hold_opts).unwrap();

        let intermission_guids = storage
            .list_active_intermissions_for_activity_id(id)
            .unwrap();

        assert_eq!(
            intermission_guids.as_ref().unwrap().len(),
            1,
            "Intermission was not created."
        );

        let end_opts = EndOptions::builder().end_time(end_time).build();

        let ended_intermissions = storage.end_all_active_intermissions(end_opts).unwrap();

        assert_eq!(
            ended_intermissions.as_ref().unwrap().len(),
            1,
            "Not all intermissions were ended."
        );

        let ended_intermission = storage
            .read_activity(intermission_guids.as_ref().unwrap()[0])
            .unwrap();

        assert!(
            ended_intermission.has_ended(),
            "Intermission has not ended, but should have."
        );

        assert_eq!(
            ended_intermission
                .activity_end_options()
                .as_ref()
                .unwrap()
                .end(),
            &PaceDateTime::new(end_time),
            "End time was not set."
        );
    }
}
