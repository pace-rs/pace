use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use merge::Merge;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    commands::{resume::ResumeOptions, DeleteOptions, UpdateOptions},
    domain::{
        activity::{Activity, ActivityEndOptions, ActivityGuid, ActivityItem},
        activity_log::ActivityLog,
        filter::{ActivityFilter, FilteredActivities},
        time::calculate_duration,
    },
    error::{ActivityLogErrorKind, PaceOptResult, PaceResult},
    storage::{
        ActivityQuerying, ActivityReadOps, ActivityStateManagement, ActivityStorage,
        ActivityWriteOps, SyncStorage,
    },
    ActivityKind, ActivityKindOptions, ActivityStatus, EndOptions, HoldOptions, PaceDateTime,
};

/// Type for shared `ActivityLog`
type SharedActivityLog = Arc<RwLock<ActivityLog>>;

/// In-memory storage for activities
#[derive(Debug, Clone)]
pub struct InMemoryActivityStorage {
    log: SharedActivityLog,
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
            log: Arc::new(RwLock::new(ActivityLog::default())),
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
            log: Arc::new(RwLock::new(activity_log)),
        }
    }

    /// Try to convert the `InMemoryActivityStorage` into an `ActivityLog`
    ///
    /// # Errors
    ///
    /// Returns an error if the mutex has been poisoned
    pub fn get_activity_log(&self) -> PaceResult<ActivityLog> {
        let Ok(activity_log) = self.log.read() else {
            return Err(ActivityLogErrorKind::RwLockHasBeenPoisoned.into());
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
    fn read_activity(&self, activity_id: ActivityGuid) -> PaceResult<ActivityItem> {
        let Ok(activities) = self.log.read() else {
            return Err(ActivityLogErrorKind::RwLockHasBeenPoisoned.into());
        };

        let activity = activities
            .get(&activity_id)
            .cloned()
            .ok_or(ActivityLogErrorKind::ActivityNotFound(activity_id))?;

        drop(activities);

        Ok((activity_id, activity).into())
    }

    fn list_activities(&self, filter: ActivityFilter) -> PaceOptResult<FilteredActivities> {
        let Ok(activity_log) = self.log.read() else {
            return Err(ActivityLogErrorKind::RwLockHasBeenPoisoned.into());
        };

        let filtered = activity_log
            .par_iter()
            .filter(|(_, activity)| match filter {
                ActivityFilter::Everything => true,
                ActivityFilter::OnlyActivities => activity.kind().is_activity(),
                ActivityFilter::Active => activity.is_active(),
                ActivityFilter::ActiveIntermission => activity.is_active_intermission(),
                ActivityFilter::Ended => activity.has_ended(),
                ActivityFilter::Archived => activity.is_archived(),
                ActivityFilter::Held => activity.is_held(),
            })
            .map(|(activity_id, _)| activity_id)
            .cloned()
            .collect::<Vec<ActivityGuid>>();

        drop(activity_log);

        if filtered.is_empty() {
            return Ok(None);
        }

        match filter {
            ActivityFilter::Everything => Ok(Some(FilteredActivities::Everything(filtered))),
            ActivityFilter::OnlyActivities => {
                Ok(Some(FilteredActivities::OnlyActivities(filtered)))
            }
            ActivityFilter::Active => Ok(Some(FilteredActivities::Active(filtered))),
            ActivityFilter::ActiveIntermission => {
                Ok(Some(FilteredActivities::ActiveIntermission(filtered)))
            }
            ActivityFilter::Archived => Ok(Some(FilteredActivities::Archived(filtered))),
            ActivityFilter::Ended => Ok(Some(FilteredActivities::Ended(filtered))),
            ActivityFilter::Held => Ok(Some(FilteredActivities::Held(filtered))),
        }
    }
}

impl ActivityWriteOps for InMemoryActivityStorage {
    fn create_activity(&self, activity: Activity) -> PaceResult<ActivityItem> {
        let Ok(activities) = self.log.read() else {
            return Err(ActivityLogErrorKind::RwLockHasBeenPoisoned.into());
        };

        let activity_item = ActivityItem::from(activity.clone());

        // Search for the activity in the list of activities to see if the ID is already in use.
        // We use a ULID as the ID for the activity, so it should be unique and not collide with
        // other activities. But still, let's check if the ID is already in use. If so, let's return
        // an error.
        // FIXME: We could essentially handle the case where the ID is already in use by creating a
        // new ID and trying to insert the activity again. But for now, let's just return an error as
        // it's not expected to happen.
        if activities.contains_key(activity_item.guid()) {
            return Err(ActivityLogErrorKind::ActivityIdAlreadyInUse(*activity_item.guid()).into());
        }

        drop(activities);

        let Ok(mut activities) = self.log.write() else {
            return Err(ActivityLogErrorKind::RwLockHasBeenPoisoned.into());
        };

        // We don't check for None here, because we know that the ID was not existing in the list of
        // activities.
        _ = activities
            .activities_mut()
            .insert(*activity_item.guid(), activity_item.activity().clone());

        drop(activities);

        Ok(activity_item)
    }

    fn update_activity(
        &self,
        activity_id: ActivityGuid,
        updated_activity: Activity,
        _update_opts: UpdateOptions,
    ) -> PaceResult<ActivityItem> {
        let Ok(activities) = self.log.read() else {
            return Err(ActivityLogErrorKind::RwLockHasBeenPoisoned.into());
        };

        let original_activity = activities
            .get(&activity_id)
            .cloned()
            .ok_or(ActivityLogErrorKind::ActivityNotFound(activity_id))?;

        drop(activities);

        let Ok(mut activities) = self.log.write() else {
            return Err(ActivityLogErrorKind::RwLockHasBeenPoisoned.into());
        };

        let _ = activities.entry(activity_id).and_modify(|activity| {
            activity.merge(updated_activity);
        });

        drop(activities);

        Ok((activity_id, original_activity).into())
    }

    fn delete_activity(
        &self,
        activity_id: ActivityGuid,
        _delete_opts: DeleteOptions,
    ) -> PaceResult<ActivityItem> {
        let Ok(mut activities) = self.log.write() else {
            return Err(ActivityLogErrorKind::RwLockHasBeenPoisoned.into());
        };

        let activity = activities
            .remove(&activity_id)
            .ok_or(ActivityLogErrorKind::ActivityNotFound(activity_id))?;

        drop(activities);

        Ok((activity_id, activity).into())
    }
}

impl ActivityStateManagement for InMemoryActivityStorage {
    fn end_activity(
        &self,
        activity_id: ActivityGuid,
        end_opts: EndOptions,
    ) -> PaceResult<ActivityItem> {
        let Ok(mut activities) = self.log.write() else {
            return Err(ActivityLogErrorKind::RwLockHasBeenPoisoned.into());
        };

        let _ = activities.entry(activity_id).and_modify(|activity| {
            match calculate_duration(activity.begin(), *end_opts.end_time()) {
                Ok(duration) => {
                    let end_opts = ActivityEndOptions::new(*end_opts.end_time(), duration);
                    activity.end_activity(end_opts);
                }
                Err(_) => {
                    log::warn!(
                        "Activity {} ends before it began. That's impossible. Skipping \
                                 activity. Please fix manually and run the command again.",
                        activity
                    );
                }
            }
        });

        drop(activities);

        self.read_activity(activity_id)
    }

    fn end_last_unfinished_activity(&self, end_opts: EndOptions) -> PaceOptResult<ActivityItem> {
        let Some(most_recent) = self.most_recent_active_activity()? else {
            return Ok(None);
        };

        let activity = self.end_activity(*most_recent.guid(), end_opts)?;

        Ok(Some(activity))
    }

    fn end_all_unfinished_activities(
        &self,
        end_opts: EndOptions,
    ) -> PaceOptResult<Vec<ActivityItem>> {
        let Ok(activities) = self.log.read() else {
            return Err(ActivityLogErrorKind::RwLockHasBeenPoisoned.into());
        };

        let active_activities = activities
            .par_iter()
            .filter_map(|(activity_id, activity)| {
                if activity.is_active() {
                    Some(*activity_id)
                } else {
                    None
                }
            })
            .collect::<Vec<ActivityGuid>>();

        drop(activities);

        // There are no active activities
        if active_activities.is_empty() {
            return Ok(None);
        }

        let ended_activities = active_activities
            .par_iter()
            .map(|activity_id| -> PaceResult<ActivityItem> {
                self.end_activity(*activity_id, end_opts.clone())
            })
            .collect::<PaceResult<Vec<ActivityItem>>>()?;

        if ended_activities.len() != active_activities.len() {
            // This is weird, we should return an error about it
            return Err(ActivityLogErrorKind::ActivityNotEnded.into());
        }

        Ok(Some(ended_activities))
    }

    fn hold_most_recent_active_activity(
        &self,
        hold_opts: HoldOptions,
    ) -> PaceOptResult<ActivityItem> {
        // Get id from last activity that is not ended
        let Some(active_activity) = self.most_recent_active_activity()? else {
            // There are no active activities
            return Ok(None);
        };

        Some(self.hold_activity(*active_activity.guid(), hold_opts)).transpose()
    }

    fn end_all_active_intermissions(
        &self,
        end_opts: EndOptions,
    ) -> PaceOptResult<Vec<ActivityGuid>> {
        let Some(active_intermissions) = self.list_active_intermissions()? else {
            // There are no active intermissions
            return Ok(None);
        };

        let ended_intermissions = active_intermissions
            .par_iter()
            .map(|activity_id| -> PaceResult<ActivityGuid> {
                let _ = self.end_activity(*activity_id, end_opts.clone())?;
                Ok(*activity_id)
            })
            .collect::<PaceResult<Vec<ActivityGuid>>>()?;

        if ended_intermissions.len() != active_intermissions.len() {
            // This is weird, we should return an error about it
            return Err(ActivityLogErrorKind::ActivityNotEnded.into());
        }

        Ok(Some(ended_intermissions))
    }

    fn resume_activity(
        &self,
        activity_id: ActivityGuid,
        resume_opts: ResumeOptions,
    ) -> PaceResult<ActivityItem> {
        let resumable_activity = self.read_activity(activity_id)?;

        // If the activity is not active, return early with an error
        if !resumable_activity.activity().is_held() {
            return Err(ActivityLogErrorKind::NoHeldActivityFound(activity_id).into());
        } else if resumable_activity.activity().is_active() {
            return Err(ActivityLogErrorKind::ActiveActivityFound(activity_id).into());
        } else if resumable_activity.activity().has_ended() {
            return Err(ActivityLogErrorKind::ActivityAlreadyEnded(activity_id).into());
        } else if resumable_activity.activity().is_archived() {
            return Err(ActivityLogErrorKind::ActivityAlreadyArchived(activity_id).into());
        };

        // If there are active intermissions for any activity, end the intermissions
        // because the user wants to resume from an intermission and time is limited,
        // so you can't have multiple intermissions at once, only one at a time.
        let _ = self.end_all_active_intermissions(resume_opts.into())?;

        // Update the activity to be active again
        let mut editable_activity = resumable_activity.clone();

        let updated_activity = editable_activity
            .activity_mut()
            .set_status(ActivityStatus::Active)
            .clone();

        let _ = self.update_activity(
            *resumable_activity.guid(),
            updated_activity.clone(),
            UpdateOptions::default(),
        )?;

        Ok(resumable_activity)
    }

    fn hold_activity(
        &self,
        activity_id: ActivityGuid,
        hold_opts: HoldOptions,
    ) -> PaceResult<ActivityItem> {
        // Get ActivityItem for activity that
        let active_activity = self.read_activity(activity_id)?;

        // make sure, the activity is not already ended or archived
        if !active_activity.activity().is_active() {
            return Err(ActivityLogErrorKind::NoActiveActivityFound(activity_id).into());
        } else if active_activity.activity().has_ended() {
            return Err(ActivityLogErrorKind::ActivityAlreadyEnded(activity_id).into());
        } else if active_activity.activity().is_archived() {
            return Err(ActivityLogErrorKind::ActivityAlreadyArchived(activity_id).into());
        };

        // Check if the latest active activity is already having an intermission
        if let Some(intermissions) =
            self.list_active_intermissions_for_activity_id(*active_activity.guid())?
        {
            // TODO!: What if there are any other intermissions ongoing for other activities?
            // TODO!: Should we end them as well? Or should we just end the intermission for the active activity?

            // If there are active intermissions and we want to extend return early with the active activity
            //
            // Handles the case, if someone wants to create an intermission for an
            // activity that already has an intermission, but hasn't set that we should
            // create a new intermission. In this case we don't want to create
            // another intermission, but return with the active activity.
            if !intermissions.is_empty() && hold_opts.action().is_extend() {
                return Ok(active_activity);
            }
        };

        // If there are active intermissions for any activity, end the intermissions
        // because the user wants to create a new intermission and time is limited,
        // so you can't have multiple intermissions at once, only one at a time.
        let _ = self.end_all_active_intermissions(hold_opts.clone().into())?;

        // Create a new intermission for the active activity
        let activity_kind_opts = ActivityKindOptions::with_parent_id(*active_activity.guid());

        let description = hold_opts.reason().clone().unwrap_or_else(|| {
            active_activity
                .activity()
                .description()
                .clone()
                .unwrap_or_else(|| format!("Holding {}", active_activity.activity()))
        });

        let intermission = Activity::builder()
            .begin(*hold_opts.begin_time())
            .kind(ActivityKind::Intermission)
            .status(ActivityStatus::Active)
            .description(description)
            .category(active_activity.activity().category().clone())
            .activity_kind_options(activity_kind_opts)
            .build();

        let _created_intermission_item = self.begin_activity(intermission.clone())?;

        // Update the active activity to be held
        let mut editable_activity = active_activity.clone();
        let updated_activity = editable_activity
            .activity_mut()
            .set_status(ActivityStatus::Held)
            .clone();

        let _ = self.update_activity(
            *active_activity.guid(),
            updated_activity.clone(),
            UpdateOptions::default(),
        )?;

        Ok((*active_activity.guid(), updated_activity).into())
    }

    fn resume_most_recent_activity(
        &self,
        resume_opts: ResumeOptions,
    ) -> PaceOptResult<ActivityItem> {
        // Get id from last activity that is not ended
        let Some(active_activity) = self.most_recent_held_activity()? else {
            // There are no active activities
            return Ok(None);
        };

        // TODO!: Check how applicable that is!
        // - If there are active intermissions for any activity, end the intermissions
        //   and resume the activity with the same id as the most recent intermission's parent_id
        // - If there are no active intermissions, but there are active activities, return the last active activity
        // - If there are no active intermissions, resume the activity with the given id or the last active activity

        Some(self.resume_activity(*active_activity.guid(), resume_opts)).transpose()
    }
}

impl ActivityQuerying for InMemoryActivityStorage {
    fn find_activities_in_date_range(
        &self,
        _start: PaceDateTime,
        _end: PaceDateTime,
    ) -> PaceResult<ActivityLog> {
        todo!("Implement find_activities_in_date_range for InMemoryActivityStorage")
    }

    fn list_activities_by_id(&self) -> PaceOptResult<BTreeMap<ActivityGuid, Activity>> {
        let Ok(activities) = self.log.read() else {
            return Err(ActivityLogErrorKind::RwLockHasBeenPoisoned.into());
        };

        let activities_by_id = activities.activities().clone();

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

        let item = storage.create_activity(activity.clone()).unwrap();

        assert_eq!(
            storage.get_activity_log().unwrap().activities().len(),
            1,
            "Activity was not created."
        );

        let stored_activity = storage.read_activity(*item.guid()).unwrap();

        assert_eq!(
            activity,
            *stored_activity.activity(),
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

        let _activity_item = storage.create_activity(activity.clone()).unwrap();

        let filtered_activities = storage
            .list_activities(ActivityFilter::Everything)
            .unwrap()
            .unwrap()
            .into_vec();

        assert_eq!(
            filtered_activities.len(),
            1,
            "Amount of activities is not the same as the amount of created activities."
        );

        let stored_activity = storage.read_activity(filtered_activities[0]).unwrap();

        assert_eq!(
            activity,
            *stored_activity.activity(),
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

        let activity_item = storage.create_activity(og_activity.clone()).unwrap();

        let read_activity = storage.read_activity(*activity_item.guid()).unwrap();

        assert_eq!(
            og_activity,
            *read_activity.activity(),
            "Stored activity is not the same as the original activity."
        );

        let new_description = "Updated description";

        let updated_activity = Activity::builder()
            .begin(begin + chrono::Duration::seconds(30))
            .kind(ActivityKind::PomodoroWork)
            .status(ActivityStatus::Active)
            .description(new_description)
            .build();

        let old_activity = storage
            .update_activity(
                *activity_item.guid(),
                updated_activity.clone(),
                UpdateOptions::default(),
            )
            .unwrap();

        assert_eq!(
            og_activity,
            *old_activity.activity(),
            "Stored activity is not the same as the original activity."
        );

        let new_stored_activity = storage.read_activity(*activity_item.guid()).unwrap();

        assert_eq!(
            old_activity.guid(),
            new_stored_activity.guid(),
            "ID was updated, but shouldn't."
        );

        assert_eq!(
            new_stored_activity.activity().description().as_deref(),
            Some(new_description),
            "Description was not updated."
        );

        assert_eq!(
            old_activity.activity().kind(),
            new_stored_activity.activity().kind(),
            "Kind was updated, but shouldn't."
        );

        assert_eq!(
            og_activity.begin(),
            new_stored_activity.activity().begin(),
            "Begin time was updated, but shouldn't."
        );

        assert!(
            new_stored_activity.activity().is_active(),
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

        let mut activity = Activity::builder()
            .begin(begin)
            .kind(kind)
            .description(description)
            .build();

        assert_eq!(
            storage.get_activity_log().unwrap().activities().len(),
            0,
            "Activity log is not empty."
        );

        let activity_item = storage.begin_activity(activity.clone()).unwrap();

        assert_eq!(
            storage.get_activity_log().unwrap().activities().len(),
            1,
            "Activity was not created."
        );

        // Read activity
        let stored_activity = storage.read_activity(*activity_item.guid()).unwrap();

        // Make sure the activity is active now, as begin_activity should make it active automatically
        activity.make_active();

        assert_eq!(
            activity,
            *stored_activity.activity(),
            "Stored activity is not the same as the original activity."
        );

        assert_eq!(
            *stored_activity.activity().status(),
            ActivityStatus::Active,
            "Activity is not active."
        );

        // Update activity
        let new_description = "Updated description";

        let updated_activity = Activity::builder()
            .begin(begin + chrono::Duration::seconds(30))
            .kind(ActivityKind::PomodoroWork)
            .status(ActivityStatus::Inactive)
            .description(new_description)
            .build();

        let _ = storage
            .update_activity(
                *activity_item.guid(),
                updated_activity.clone(),
                UpdateOptions::default(),
            )
            .unwrap();

        let new_stored_activity = storage.read_activity(*activity_item.guid()).unwrap();

        assert_eq!(
            new_stored_activity.activity().description().as_deref(),
            Some(new_description),
            "Description was not updated."
        );

        assert_eq!(
            stored_activity.activity().kind(),
            new_stored_activity.activity().kind(),
            "Kind was updated, but shouldn't."
        );

        assert_eq!(
            stored_activity.activity().begin(),
            new_stored_activity.activity().begin(),
            "Begin time was updated, but shouldn't."
        );

        assert!(
            new_stored_activity.activity().is_inactive(),
            "Activity should be active now, but was not updated."
        );

        // Delete activity
        let deleted_activity = storage
            .delete_activity(*activity_item.guid(), DeleteOptions::default())
            .unwrap();

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

        let read_deleted_activity_result = storage.read_activity(*activity_item.guid());

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

        let activity_item = storage.begin_activity(activity.clone()).unwrap();

        let end_opts = EndOptions::builder().end_time(end_time).build();

        let ended_activity = storage
            .end_activity(*activity_item.guid(), end_opts)
            .unwrap();

        assert_ne!(
            activity_item, ended_activity,
            "Activities do match, although they should be different."
        );

        assert!(ended_activity.activity().activity_end_options().is_some());

        let ended_activity = storage.read_activity(*activity_item.guid()).unwrap();

        assert!(
            ended_activity.activity().has_ended(),
            "Activity has not ended, but should have."
        );

        assert_eq!(
            ended_activity
                .activity()
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

        let activity_item = storage.begin_activity(activity.clone()).unwrap();

        let ended_activity = storage
            .end_last_unfinished_activity(EndOptions::builder().end_time(now).build())
            .unwrap()
            .unwrap();

        assert_eq!(
            ended_activity.guid(),
            activity_item.guid(),
            "Activity IDs do not match."
        );

        assert!(
            ended_activity.activity().has_ended(),
            "Activity has not ended, but should have."
        );

        assert_eq!(
            ended_activity
                .activity()
                .activity_end_options()
                .as_ref()
                .unwrap()
                .end(),
            &PaceDateTime::new(now),
            "End time was not set."
        );
    }

    #[test]
    fn test_begin_and_auto_end_for_multiple_activities_passes() {
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

        // Begin the first activity
        let activity_item = storage.begin_activity(activity.clone()).unwrap();

        let begin_time = now - chrono::Duration::seconds(60);
        let kind = ActivityKind::Activity;
        let description = "Test activity 2";

        let activity2 = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .build();

        // Begin the second activity, the first one should be ended automatically now
        let activity_item2 = storage.begin_activity(activity2.clone()).unwrap();

        let ended_activity = storage.read_activity(*activity_item.guid()).unwrap();

        assert!(
            ended_activity.activity().has_ended(),
            "Activity has not ended, but should have."
        );

        assert_eq!(
            ended_activity
                .activity()
                .activity_end_options()
                .as_ref()
                .unwrap()
                .end(),
            &PaceDateTime::new(now),
            "End time was not set."
        );

        let ended_activity2 = storage.read_activity(*activity_item2.guid()).unwrap();

        assert!(
            ended_activity2.activity().is_active(),
            "Activity has not ended, but should have."
        );

        assert!(
            ended_activity2.activity().activity_end_options().is_none(),
            "End time should not be set."
        );
    }

    #[test]
    fn test_hold_most_recent_active_activity_passes() {
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

        let activity_item = storage.begin_activity(activity.clone()).unwrap();

        let hold_time = now + chrono::Duration::seconds(30);

        let hold_opts = HoldOptions::builder().begin_time(hold_time).build();

        let held_activity = storage
            .hold_most_recent_active_activity(hold_opts)
            .unwrap()
            .unwrap();

        assert_eq!(
            held_activity.guid(),
            activity_item.guid(),
            "Activity IDs do not match."
        );

        let intermission_guids = storage
            .list_active_intermissions_for_activity_id(*activity_item.guid())
            .unwrap()
            .unwrap();

        assert_eq!(intermission_guids.len(), 1, "Intermission was not created.");

        let intermission_item = storage.read_activity(intermission_guids[0]).unwrap();

        assert_eq!(
            *intermission_item.activity().kind(),
            ActivityKind::Intermission,
            "Intermission was not created."
        );

        assert_eq!(
            intermission_item
                .activity()
                .activity_kind_options()
                .as_ref()
                .unwrap()
                .parent_id()
                .unwrap(),
            *activity_item.guid(),
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

        let active_activity_item = storage.begin_activity(activity.clone()).unwrap();

        let hold_opts = HoldOptions::builder()
            .begin_time(now + chrono::Duration::seconds(30))
            .build();

        let _held_item = storage
            .hold_most_recent_active_activity(hold_opts)
            .unwrap()
            .unwrap();

        let held_activity = storage.read_activity(*active_activity_item.guid()).unwrap();

        assert_eq!(
            *held_activity.activity().status(),
            ActivityStatus::Held,
            "Activity was not held."
        );

        let intermission_guids = storage
            .list_active_intermissions_for_activity_id(*active_activity_item.guid())
            .unwrap();

        assert_eq!(
            intermission_guids.as_ref().unwrap().len(),
            1,
            "Intermission was not created."
        );

        let hold_opts = HoldOptions::builder()
            .begin_time(now + chrono::Duration::seconds(60))
            .build();

        assert!(
            storage
                .hold_most_recent_active_activity(hold_opts)
                .unwrap()
                .is_none(),
            "Activity was held again."
        );

        let intermission_guids = storage
            .list_active_intermissions_for_activity_id(*active_activity_item.guid())
            .unwrap()
            .unwrap();

        assert_eq!(
            intermission_guids.len(),
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

        let active_activity_item = storage.begin_activity(activity.clone()).unwrap();

        let hold_opts = HoldOptions::builder()
            .begin_time(now + chrono::Duration::seconds(30))
            .build();

        let _ = storage.hold_most_recent_active_activity(hold_opts).unwrap();

        let intermission_guids = storage
            .list_active_intermissions_for_activity_id(*active_activity_item.guid())
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
            ended_intermission.activity().has_ended(),
            "Intermission has not ended, but should have."
        );

        assert_eq!(
            ended_intermission
                .activity()
                .activity_end_options()
                .as_ref()
                .unwrap()
                .end(),
            &PaceDateTime::new(end_time),
            "End time was not set."
        );
    }

    #[test]
    fn test_important_pace_flow_for_activities_passes() {
        let storage = InMemoryActivityStorage::new();
        let _now = PaceDateTime::now();

        let first_og_activity = Activity::builder().description("Test activity").build();

        let first_begin_activity = storage.begin_activity(first_og_activity.clone()).unwrap();

        let first_stored_activity = storage.read_activity(*first_begin_activity.guid()).unwrap();

        assert_eq!(
            first_og_activity.begin(),
            first_stored_activity.activity().begin(),
            "Stored activity has not the same begin time as the original activity."
        );

        assert_eq!(
            first_og_activity.description().as_deref(),
            first_stored_activity.activity().description().as_deref(),
            "Stored activity has not the same description as the original activity."
        );

        assert_eq!(
            first_og_activity.kind(),
            first_stored_activity.activity().kind(),
            "Stored activity has not the same kind as the original activity."
        );

        assert_ne!(
            first_og_activity.status(),
            first_stored_activity.activity().status(),
            "Stored activity has the same status as the original activity. Which can't be, because it should be active."
        );

        assert!(
            first_stored_activity.activity().status().is_active(),
            "Stored activity is not active."
        );

        assert!(
            first_og_activity.status().is_inactive(),
            "Original activity is not inactive."
        );

        // Now we create another activity, which should end the first one automatically

        let second_og_activity = Activity::builder().description("Our new activity").build();

        let second_begin_activity = storage.begin_activity(second_og_activity.clone()).unwrap();

        let second_stored_activity = storage
            .read_activity(*second_begin_activity.guid())
            .unwrap();

        let first_stored_activity = storage.read_activity(*first_begin_activity.guid()).unwrap();

        assert!(
            first_stored_activity.activity().status().is_ended(),
            "First activity is not ended."
        );

        assert_eq!(
            second_og_activity.begin(),
            second_stored_activity.activity().begin(),
            "Stored activity has not the same begin time as the original activity."
        );

        assert_eq!(
            second_og_activity.description().as_deref(),
            second_stored_activity.activity().description().as_deref(),
            "Stored activity has not the same description as the original activity."
        );

        assert_eq!(
            second_og_activity.kind(),
            second_stored_activity.activity().kind(),
            "Stored activity has not the same kind as the original activity."
        );

        assert_ne!(
            second_og_activity.status(),
            second_stored_activity.activity().status(),
            "Stored activity has the same status as the original activity. Which can't be, because it should be active."
        );

        assert!(
            second_stored_activity.activity().status().is_active(),
            "Stored activity is not active."
        );

        assert!(
            second_og_activity.status().is_inactive(),
            "Original activity is not inactive."
        );

        // Now we create an intermission for the second activity

        let _ = storage
            .hold_most_recent_active_activity(HoldOptions::default())
            .unwrap()
            .unwrap();

        let second_stored_activity = storage
            .read_activity(*second_begin_activity.guid())
            .unwrap();

        assert!(
            second_stored_activity.activity().status().is_held(),
            "Second activity is not held."
        );

        // This is more complicated, but maybe also on purpose, as directly dealing with the intermission
        // is not the most common use case and should be discouraged as messing with it could lead to
        // inconsistencies in the data.
        let second_activity_intermission_id = storage
            .list_active_intermissions_for_activity_id(*second_begin_activity.guid())
            .unwrap()
            .unwrap();
        let second_activity_intermission_id = second_activity_intermission_id.first().unwrap();

        let second_stored_intermission = storage
            .read_activity(*second_activity_intermission_id)
            .unwrap();

        assert_eq!(
            second_stored_intermission
                .activity()
                .activity_kind_options()
                .as_ref()
                .unwrap()
                .parent_id()
                .unwrap(),
            *second_begin_activity.guid(),
            "Parent IDs of intermission and parent activity do not match."
        );

        // Now we want to continue the activity, which should end the intermission automatically
        // and set the activity from held to active again

        let resumed_activity = storage
            .resume_most_recent_activity(ResumeOptions::default())
            .unwrap()
            .unwrap();

        let resumed_stored_activity = storage.read_activity(*resumed_activity.guid()).unwrap();

        let second_stored_intermission = storage
            .read_activity(*second_activity_intermission_id)
            .unwrap();

        assert!(
            resumed_stored_activity.activity().status().is_active(),
            "Resumed activity is not active."
        );

        assert!(
            second_stored_intermission.activity().status().is_ended(),
            "Intermission has not ended."
        );

        assert!(
            second_stored_intermission.activity().has_ended(),
            "Intermission has not ended."
        );

        assert!(
            resumed_stored_activity.activity().status().is_active(),
            "Resumed activity is not active."
        );

        assert_eq!(
            resumed_stored_activity.guid(),
            second_stored_activity.guid(),
            "Resumed activity is not the same as the second stored activity."
        );

        assert_eq!(
            resumed_stored_activity.activity().begin(),
            second_stored_activity.activity().begin(),
            "Resumed activity has not the same begin time as the second stored activity."
        );

        assert_eq!(
            resumed_stored_activity.activity().description().as_deref(),
            second_stored_activity.activity().description().as_deref(),
            "Resumed activity has not the same description as the second stored activity."
        );

        assert_eq!(
            resumed_stored_activity.activity().kind(),
            second_stored_activity.activity().kind(),
            "Resumed activity has not the same kind as the second stored activity."
        );

        assert!(!resumed_stored_activity.activity().has_ended());
    }
}
