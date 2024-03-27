use std::{collections::BTreeMap, sync::Arc};

use pace_time::{
    date::PaceDate,
    duration::{calculate_duration, PaceDurationRange},
    time_range::TimeRangeOptions,
};
use parking_lot::RwLock;

use merge::Merge;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use tracing::debug;

use pace_core::prelude::{
    Activity, ActivityEndOptions, ActivityFilterKind, ActivityGuid, ActivityItem, ActivityKind,
    ActivityKindOptions, ActivityLog, ActivityQuerying, ActivityReadOps, ActivityStateManagement,
    ActivityStatusKind, ActivityStorage, ActivityWriteOps, DeleteOptions, EndOptions,
    FilteredActivities, HoldOptions, KeywordOptions, ResumeOptions, SyncStorage, UpdateOptions,
};
use pace_error::{ActivityLogErrorKind, PaceOptResult, PaceResult};

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
    #[must_use]
    pub fn new_with_activity_log(activity_log: ActivityLog) -> Self {
        Self {
            log: Arc::new(RwLock::new(activity_log)),
        }
    }

    /// Try to convert the `InMemoryActivityStorage` into an `ActivityLog`
    pub fn get_activity_log(&self) -> ActivityLog {
        let activity_log = self.log.read();

        debug!("Got activity log");

        activity_log.clone()
    }
}

impl Default for InMemoryActivityStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivityStorage for InMemoryActivityStorage {
    fn setup(&self) -> PaceResult<()> {
        debug!("Setting up in-memory storage");
        Ok(())
    }

    fn teardown(&self) -> PaceResult<()> {
        debug!("Tearing down in-memory storage");
        Ok(())
    }

    fn identify(&self) -> String {
        "In-memory storage".to_string()
    }
}

impl SyncStorage for InMemoryActivityStorage {
    fn sync(&self) -> PaceResult<()> {
        debug!("Syncing in-memory storage");

        Ok(())
    }
}

impl ActivityReadOps for InMemoryActivityStorage {
    #[tracing::instrument(skip(self))]
    fn read_activity(&self, activity_id: ActivityGuid) -> PaceResult<ActivityItem> {
        let activities = self.log.read();

        let activity =
            activities
                .get(&activity_id)
                .cloned()
                .ok_or(ActivityLogErrorKind::ActivityNotFound(
                    activity_id.to_string(),
                ))?;

        drop(activities);

        debug!("Activity with id {:?} found: {:?}", activity_id, activity);

        Ok((activity_id, activity).into())
    }

    #[tracing::instrument(skip(self))]
    fn list_activities(&self, filter: ActivityFilterKind) -> PaceOptResult<FilteredActivities> {
        let activity_log = self.log.read();

        let filtered = activity_log
            .par_iter()
            .filter(|(_, activity)| match filter {
                ActivityFilterKind::Everything => true,
                ActivityFilterKind::OnlyActivities => activity.kind().is_activity(),
                ActivityFilterKind::Active => activity.is_in_progress(),
                ActivityFilterKind::ActiveIntermission => activity.is_active_intermission(),
                ActivityFilterKind::Ended => activity.is_completed(),
                ActivityFilterKind::Archived => activity.is_archived(),
                ActivityFilterKind::Held => activity.is_paused(),
                ActivityFilterKind::Intermission => activity.kind().is_intermission(),
                ActivityFilterKind::TimeRange(time_range_opts) => {
                    // TODO: When adding Pomodoro support, we should also check for Pomodoro activities
                    time_range_opts.is_in_range(*activity.begin()) && activity.kind().is_activity()
                }
            })
            .map(|(activity_id, _)| activity_id)
            .cloned()
            .collect::<Vec<ActivityGuid>>();

        drop(activity_log);

        debug!("Filtered activities: {:?}", filtered);

        if filtered.is_empty() {
            return Ok(None);
        }

        match filter {
            ActivityFilterKind::Everything => Ok(Some(FilteredActivities::Everything(filtered))),
            ActivityFilterKind::OnlyActivities => {
                Ok(Some(FilteredActivities::OnlyActivities(filtered)))
            }
            ActivityFilterKind::Active => Ok(Some(FilteredActivities::Active(filtered))),
            ActivityFilterKind::ActiveIntermission => {
                Ok(Some(FilteredActivities::ActiveIntermission(filtered)))
            }
            ActivityFilterKind::Archived => Ok(Some(FilteredActivities::Archived(filtered))),
            ActivityFilterKind::Ended => Ok(Some(FilteredActivities::Ended(filtered))),
            ActivityFilterKind::Held => Ok(Some(FilteredActivities::Held(filtered))),
            ActivityFilterKind::Intermission => {
                Ok(Some(FilteredActivities::Intermission(filtered)))
            }
            ActivityFilterKind::TimeRange(_) => Ok(Some(FilteredActivities::TimeRange(filtered))),
        }
    }
}

impl ActivityWriteOps for InMemoryActivityStorage {
    #[tracing::instrument(skip(self))]
    fn create_activity(&self, activity: Activity) -> PaceResult<ActivityItem> {
        let activities = self.log.read();

        let activity_item = ActivityItem::from(activity);

        // Search for the activity in the list of activities to see if the ID is already in use.
        // We use a ULID as the ID for the activity, so it should be unique and not collide with
        // other activities. But still, let's check if the ID is already in use. If so, let's return
        // an error.
        // FIXME: We could essentially handle the case where the ID is already in use by creating a
        // new ID and trying to insert the activity again. But for now, let's just return an error as
        // it's not expected to happen.
        if activities.contains_key(activity_item.guid()) {
            debug!("Activity ID already in use: {:?}", activity_item.guid());
            return Err(ActivityLogErrorKind::ActivityIdAlreadyInUse(
                activity_item.guid().to_string(),
            )
            .into());
        }

        drop(activities);

        let mut activities = self.log.write();

        // We don't check for None here, because we know that the ID was not existing in the list of
        // activities.
        _ = activities
            .activities_mut()
            .insert(*activity_item.guid(), activity_item.activity().clone());

        drop(activities);

        Ok(activity_item)
    }

    #[tracing::instrument(skip(self))]
    fn update_activity(
        &self,
        activity_id: ActivityGuid,
        updated_activity: Activity,
        update_opts: UpdateOptions,
    ) -> PaceResult<ActivityItem> {
        let activities = self.log.read();

        let original_activity =
            activities
                .get(&activity_id)
                .cloned()
                .ok_or(ActivityLogErrorKind::ActivityNotFound(
                    activity_id.to_string(),
                ))?;

        debug!("Original activity: {:?}", original_activity);

        drop(activities);

        let mut activities = self.log.write();

        let _ = activities.entry(activity_id).and_modify(|activity| {
            debug!("Updating activity: {:?}", activity);
            activity.merge(updated_activity);
        });

        drop(activities);

        Ok((activity_id, original_activity).into())
    }

    #[tracing::instrument(skip(self))]
    fn delete_activity(
        &self,
        activity_id: ActivityGuid,
        delete_opts: DeleteOptions,
    ) -> PaceResult<ActivityItem> {
        let mut activities = self.log.write();

        let activity =
            activities
                .remove(&activity_id)
                .ok_or(ActivityLogErrorKind::ActivityNotFound(
                    activity_id.to_string(),
                ))?;

        drop(activities);

        Ok((activity_id, activity).into())
    }
}

impl ActivityStateManagement for InMemoryActivityStorage {
    #[tracing::instrument(skip(self))]
    fn end_activity(
        &self,
        activity_id: ActivityGuid,
        end_opts: EndOptions,
    ) -> PaceResult<ActivityItem> {
        let activities = self.log.read();

        let begin_time = *activities
            .get(&activity_id)
            .ok_or(ActivityLogErrorKind::ActivityNotFound(
                activity_id.to_string(),
            ))?
            .begin();

        drop(activities);

        let end_opts = ActivityEndOptions::new(
            *end_opts.end_time(),
            calculate_duration(&begin_time, end_opts.end_time())?,
        );

        debug!("End options: {:?}", end_opts);

        let mut activities = self.log.write();

        let _ = activities
            .entry(activity_id)
            .and_modify(|activity| activity.end_activity(end_opts));

        drop(activities);

        self.read_activity(activity_id)
    }

    #[tracing::instrument(skip(self))]
    fn end_last_unfinished_activity(&self, end_opts: EndOptions) -> PaceOptResult<ActivityItem> {
        let Some(most_recent) = self.most_recent_active_activity()? else {
            debug!("No active activity found.");
            return Ok(None);
        };

        debug!("Most recent activity: {:?}", most_recent);

        let activity = self.end_activity(*most_recent.guid(), end_opts)?;

        Ok(Some(activity))
    }

    #[tracing::instrument(skip(self))]
    fn end_all_activities(&self, end_opts: EndOptions) -> PaceOptResult<Vec<ActivityItem>> {
        let activities = self.log.read();

        let endable_activities = activities
            .par_iter()
            .filter_map(|(activity_id, activity)| {
                if activity.is_completable() {
                    Some(*activity_id)
                } else {
                    None
                }
            })
            .collect::<Vec<ActivityGuid>>();

        drop(activities);

        debug!("Endable activities: {:?}", endable_activities);

        // There are no active activities
        if endable_activities.is_empty() {
            debug!("No active activities found.");
            return Ok(None);
        }

        let ended_activities = endable_activities
            .par_iter()
            .map(|activity_id| -> PaceResult<ActivityItem> {
                self.end_activity(*activity_id, end_opts.clone())
            })
            .collect::<PaceResult<Vec<ActivityItem>>>()?;

        debug!("Ended activities: {:?}", ended_activities);

        if ended_activities.len() != endable_activities.len() {
            debug!("Not all activities were ended.");

            // This is weird, we should return an error about it
            return Err(ActivityLogErrorKind::ActivityNotEnded.into());
        }

        Ok(Some(ended_activities))
    }

    #[tracing::instrument(skip(self))]
    fn hold_most_recent_active_activity(
        &self,
        hold_opts: HoldOptions,
    ) -> PaceOptResult<ActivityItem> {
        // Get id from last activity that is not ended
        let Some(active_activity) = self.most_recent_active_activity()? else {
            debug!("No active activity found.");

            // There are no active activities
            return Ok(None);
        };

        Some(self.hold_activity(*active_activity.guid(), hold_opts)).transpose()
    }

    #[tracing::instrument(skip(self))]
    fn end_all_active_intermissions(
        &self,
        end_opts: EndOptions,
    ) -> PaceOptResult<Vec<ActivityGuid>> {
        let Some(active_intermissions) = self.list_active_intermissions()? else {
            debug!("No active intermissions found.");

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

        debug!("Ended intermissions: {:?}", ended_intermissions);

        if ended_intermissions.len() != active_intermissions.len() {
            debug!("Not all intermissions were ended.");

            // This is weird, we should return an error about it
            return Err(ActivityLogErrorKind::ActivityNotEnded.into());
        }

        Ok(Some(ended_intermissions))
    }

    #[tracing::instrument(skip(self))]
    fn resume_activity(
        &self,
        activity_id: ActivityGuid,
        resume_opts: ResumeOptions,
    ) -> PaceResult<ActivityItem> {
        let resumable_activity = self.read_activity(activity_id)?;

        debug!("Resumable activity: {:?}", resumable_activity);

        // If the activity is active, return early with an error
        if resumable_activity.activity().is_in_progress() {
            debug!("Activity is already active.");
            return Err(ActivityLogErrorKind::ActiveActivityFound(activity_id.to_string()).into());
        } else if resumable_activity.activity().is_completed() {
            debug!("Activity has ended.");
            return Err(ActivityLogErrorKind::ActivityAlreadyEnded(activity_id.to_string()).into());
        } else if resumable_activity.activity().is_archived() {
            debug!("Activity is archived.");
            return Err(
                ActivityLogErrorKind::ActivityAlreadyArchived(activity_id.to_string()).into(),
            );
        } else if !resumable_activity.activity().is_paused() {
            debug!("Activity is not held.");
            return Err(ActivityLogErrorKind::NoHeldActivityFound(activity_id.to_string()).into());
        };

        // If there are active intermissions for any activity, end the intermissions
        // because the user wants to resume from an intermission and time is limited,
        // so you can't have multiple intermissions at once, only one at a time.
        let ended_intermission_ids = self.end_all_active_intermissions(resume_opts.into())?;

        debug!("Ended intermission ids: {:?}", ended_intermission_ids);

        // Update the activity to be active again
        let mut editable_activity = resumable_activity.clone();

        let updated_activity = editable_activity
            .activity_mut()
            .set_status(ActivityStatusKind::InProgress)
            .clone();

        debug!("Updated activity: {:?}", updated_activity);

        let _ = self.update_activity(
            *resumable_activity.guid(),
            updated_activity,
            UpdateOptions::default(),
        )?;

        Ok(resumable_activity)
    }

    #[tracing::instrument(skip(self))]
    fn hold_activity(
        &self,
        activity_id: ActivityGuid,
        hold_opts: HoldOptions,
    ) -> PaceResult<ActivityItem> {
        // Get ActivityItem for activity that
        let active_activity = self.read_activity(activity_id)?;

        debug!("Active activity: {:?}", active_activity);

        // make sure, the activity is not already ended or archived
        if !active_activity.activity().is_in_progress() {
            debug!("Activity is not active.");
            return Err(
                ActivityLogErrorKind::NoActiveActivityFound(activity_id.to_string()).into(),
            );
        } else if active_activity.activity().is_completed() {
            debug!("Activity has ended.");
            return Err(ActivityLogErrorKind::ActivityAlreadyEnded(activity_id.to_string()).into());
        } else if active_activity.activity().is_archived() {
            debug!("Activity is archived.");
            return Err(
                ActivityLogErrorKind::ActivityAlreadyArchived(activity_id.to_string()).into(),
            );
        };

        // Check if the latest active activity is already having an intermission
        if let Some(intermissions) =
            self.list_active_intermissions_for_activity_id(*active_activity.guid())?
        {
            debug!("Active intermissions: {:?}", intermissions);

            // TODO!: What if there are any other intermissions ongoing for other activities?
            // TODO!: Should we end them as well? Or should we just end the intermission for the active activity?

            // If there are active intermissions and we want to extend return early with the active activity
            //
            // Handles the case, if someone wants to create an intermission for an
            // activity that already has an intermission, but hasn't set that we should
            // create a new intermission. In this case we don't want to create
            // another intermission, but return with the active activity.
            if !intermissions.is_empty() && hold_opts.action().is_extend() {
                debug!("Active intermission(s) found and action is extend.");

                return Ok(active_activity);
            }
        };

        // If there are active intermissions for any activity, end the intermissions
        // because the user wants to create a new intermission and time is limited,
        // so you can't have multiple intermissions at once, only one at a time.
        let active_intermission_ids =
            self.end_all_active_intermissions(hold_opts.clone().into())?;

        debug!(
            "Ended active intermission ids: {:?}",
            active_intermission_ids
        );

        // Create a new intermission for the active activity
        let activity_kind_opts = ActivityKindOptions::with_parent_id(*active_activity.guid());

        let description = hold_opts
            .reason()
            .clone()
            .unwrap_or_else(|| active_activity.activity().description().clone());

        let intermission = Activity::builder()
            .begin(*hold_opts.begin_time())
            .kind(ActivityKind::Intermission)
            .status(ActivityStatusKind::InProgress)
            .description(description)
            .category(active_activity.activity().category().clone())
            .activity_kind_options(Some(activity_kind_opts))
            .build();

        let created_intermission_item = self.begin_activity(intermission)?;

        debug!("Created intermission: {:?}", created_intermission_item);

        // Update the active activity to be held
        let mut editable_activity = active_activity.clone();

        let updated_activity = editable_activity
            .activity_mut()
            .set_status(ActivityStatusKind::Paused)
            .clone();

        debug!("Updated activity: {:?}", updated_activity);

        let _ = self.update_activity(
            *active_activity.guid(),
            updated_activity.clone(),
            UpdateOptions::default(),
        )?;

        Ok((*active_activity.guid(), updated_activity).into())
    }

    #[tracing::instrument(skip(self))]
    fn resume_most_recent_activity(
        &self,
        resume_opts: ResumeOptions,
    ) -> PaceOptResult<ActivityItem> {
        // Get id from last activity that is not ended
        let Some(active_activity) = self.most_recent_held_activity()? else {
            debug!("No held activity found.");

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
    #[tracing::instrument(skip(self))]
    fn list_activities_by_id(&self) -> PaceOptResult<BTreeMap<ActivityGuid, Activity>> {
        let activities = self.log.read();

        let activities_by_id = activities.activities().clone();

        drop(activities);

        debug!("Activities by id: {:?}", activities_by_id.keys());

        if activities_by_id.is_empty() {
            debug!("No activities found.");

            return Ok(None);
        }

        Ok(Some(activities_by_id))
    }

    #[tracing::instrument(skip(self))]
    fn group_activities_by_duration_range(
        &self,
    ) -> PaceOptResult<BTreeMap<PaceDurationRange, Vec<ActivityItem>>> {
        todo!("Implement grouping activities by duration range")
    }

    #[tracing::instrument(skip(self))]
    fn group_activities_by_start_date(
        &self,
    ) -> PaceOptResult<BTreeMap<PaceDate, Vec<ActivityItem>>> {
        let activities = self.log.read();

        Some(activities.activities().iter().try_fold(
            BTreeMap::new(),
            |mut acc: BTreeMap<PaceDate, Vec<ActivityItem>>, (activity_id, activity)| {
                let begin_date = activity.begin().date_naive();

                debug!("Begin date: {:?}", begin_date);

                acc.entry(begin_date)
                    .or_default()
                    .push(ActivityItem::from((*activity_id, activity.clone())));

                Ok(acc)
            },
        ))
        .transpose()
    }

    #[tracing::instrument(skip(self))]
    fn list_activities_with_intermissions(
        &self,
    ) -> PaceOptResult<BTreeMap<ActivityGuid, Vec<ActivityItem>>> {
        let Some(intermissions) = self
            .list_activities(ActivityFilterKind::Intermission)?
            .map(FilteredActivities::into_vec)
        else {
            debug!("No intermissions found.");

            return Ok(None);
        };

        debug!("Intermissions: {:?}", intermissions);

        Some(intermissions.into_iter().try_fold(
            BTreeMap::new(),
            |mut acc: BTreeMap<ActivityGuid, Vec<ActivityItem>>, intermission_id| {
                let intermission = self.read_activity(intermission_id)?;

                debug!("Intermission: {:?}", intermission);

                let parent_id = intermission
                    .activity()
                    .activity_kind_options()
                    .as_ref()
                    .ok_or(ActivityLogErrorKind::ActivityKindOptionsNotFound(
                        intermission_id.to_string(),
                    ))?
                    .parent_id()
                    .ok_or(ActivityLogErrorKind::ParentIdNotSet(
                        intermission_id.to_string(),
                    ))?;

                debug!("Parent id: {:?}", parent_id);

                let parent_activity = self.read_activity(parent_id)?;

                debug!("Parent activity: {:?}", parent_activity);

                acc.entry(parent_id).or_default().push(parent_activity);

                Ok(acc)
            },
        ))
        .transpose()
    }

    #[tracing::instrument(skip(self))]
    fn group_activities_by_keywords(
        &self,
        keyword_opts: KeywordOptions,
    ) -> PaceOptResult<BTreeMap<String, Vec<ActivityItem>>> {
        let activities = self.log.read();

        Some(activities.activities().iter().try_fold(
            BTreeMap::new(),
            |mut acc: BTreeMap<String, Vec<ActivityItem>>, (activity_id, activity)| {
                // Group by category
                if let Some(category) = keyword_opts.category() {
                    let category = category.to_lowercase();

                    debug!("Category: {:?}", category);

                    if activity
                        .category()
                        .as_ref()
                        .ok_or(ActivityLogErrorKind::CategoryNotSet(
                            activity_id.to_string(),
                        ))?
                        .to_lowercase()
                        .contains(category.as_str())
                    {
                        acc.entry(category)
                            .or_default()
                            .push(ActivityItem::from((*activity_id, activity.clone())));
                    }
                } else {
                    // Use the existing activity category as the keyword

                    debug!("No category specified. Using 'Uncategorized' as the category.");

                    acc.entry(
                        activity
                            .category()
                            .as_ref()
                            .unwrap_or(&"Uncategorized".to_string())
                            .to_string(),
                    )
                    .or_default()
                    .push(ActivityItem::from((*activity_id, activity.clone())));
                }

                Ok(acc)
            },
        ))
        .transpose()
    }

    #[tracing::instrument(skip(self))]
    fn group_activities_by_kind(&self) -> PaceOptResult<BTreeMap<ActivityKind, Vec<ActivityItem>>> {
        let activities = self.log.read();

        Some(activities.activities().iter().try_fold(
            BTreeMap::new(),
            |mut acc: BTreeMap<ActivityKind, Vec<ActivityItem>>, (activity_id, activity)| {
                debug!(
                    "Activity kind: {:?} for item {:?} with id {:?}",
                    activity.kind(),
                    activity,
                    activity_id
                );

                acc.entry(*activity.kind())
                    .or_default()
                    .push(ActivityItem::from((*activity_id, activity.clone())));

                Ok(acc)
            },
        ))
        .transpose()
    }

    #[tracing::instrument(skip(self))]
    fn group_activities_by_status(
        &self,
    ) -> PaceOptResult<BTreeMap<ActivityStatusKind, Vec<ActivityItem>>> {
        let activities = self.log.read();

        Some(activities.activities().iter().try_fold(
            BTreeMap::new(),
            |mut acc: BTreeMap<ActivityStatusKind, Vec<ActivityItem>>, (activity_id, activity)| {
                debug!(
                    "Activity status: {:?} for item {:?} with id {:?}",
                    activity.status(),
                    activity,
                    activity_id
                );

                acc.entry(*activity.status())
                    .or_default()
                    .push(ActivityItem::from((*activity_id, activity.clone())));

                Ok(acc)
            },
        ))
        .transpose()
    }

    #[tracing::instrument(skip(self))]
    fn list_activities_by_time_range(
        &self,
        time_range_opts: TimeRangeOptions,
    ) -> PaceOptResult<Vec<ActivityGuid>> {
        let Some(filtered_activities) = self
            .list_activities(ActivityFilterKind::TimeRange(time_range_opts))?
            .map(FilteredActivities::into_vec)
        else {
            debug!(
                "No activities found in time range between {} and {}.",
                time_range_opts.start(),
                time_range_opts.end()
            );

            return Ok(None);
        };

        if filtered_activities.is_empty() {
            debug!(
                "No activities found in time range between {} and {}.",
                time_range_opts.start(),
                time_range_opts.end()
            );

            return Ok(None);
        }

        Ok(Some(filtered_activities))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use chrono::Local;
    use pace_error::TestResult;
    use pace_time::date_time::PaceDateTime;
    use std::collections::HashSet;

    #[test]
    fn test_in_memory_activity_storage_passes() {
        let storage = InMemoryActivityStorage::new();

        assert_eq!(
            storage.get_activity_log().activities().len(),
            0,
            "Activity log is not empty."
        );
    }

    #[test]
    fn test_in_memory_activity_storage_from_activity_log_passes() {
        let activity_log = ActivityLog::default();
        let storage = InMemoryActivityStorage::from(activity_log);

        assert_eq!(
            storage.get_activity_log().activities().len(),
            0,
            "Activity log is not empty."
        );
    }

    #[test]
    fn test_create_read_activity_passes() -> TestResult<()> {
        let storage = InMemoryActivityStorage::new();

        let begin = Local::now().fixed_offset();
        let kind = ActivityKind::Activity;
        let description = "Test activity";
        let tags = vec!["test".to_string(), "activity".to_string()]
            .into_iter()
            .collect::<HashSet<String>>();

        let activity = Activity::builder()
            .begin(begin)
            .kind(kind)
            .description(description)
            .tags(tags)
            .build();

        let item = storage.create_activity(activity.clone())?;

        assert_eq!(
            storage.get_activity_log().activities().len(),
            1,
            "Activity was not created."
        );

        let stored_activity = storage.read_activity(*item.guid())?;

        assert_eq!(
            activity,
            *stored_activity.activity(),
            "Stored activity is not the same as the original activity."
        );

        Ok(())
    }

    #[test]
    fn test_list_activities_passes() -> TestResult<()> {
        let storage = InMemoryActivityStorage::new();

        let begin = Local::now().fixed_offset();
        let kind = ActivityKind::Activity;
        let description = "Test activity";
        let tags = vec!["test".to_string(), "activity".to_string()]
            .into_iter()
            .collect::<HashSet<String>>();

        let activity = Activity::builder()
            .begin(begin)
            .kind(kind)
            .description(description)
            .tags(tags)
            .build();

        let _activity_item = storage.create_activity(activity.clone())?;

        let filtered_activities = storage
            .list_activities(ActivityFilterKind::Everything)?
            .ok_or("No activities found.")?
            .into_vec();

        assert_eq!(
            filtered_activities.len(),
            1,
            "Amount of activities is not the same as the amount of created activities."
        );

        let stored_activity = storage.read_activity(filtered_activities[0])?;

        assert_eq!(
            activity,
            *stored_activity.activity(),
            "Filtered activities are not the same as the original activity."
        );

        Ok(())
    }

    #[test]
    fn test_update_activity_passes() -> TestResult<()> {
        let storage = InMemoryActivityStorage::new();

        let begin = Local::now().fixed_offset();
        let kind = ActivityKind::Activity;
        let description = "Test activity";
        let tags = vec!["test".to_string(), "activity".to_string()]
            .into_iter()
            .collect::<HashSet<String>>();

        let og_activity = Activity::builder()
            .begin(begin)
            .kind(kind)
            .description(description)
            .tags(tags)
            .build();

        let activity_item = storage.create_activity(og_activity.clone())?;

        let read_activity = storage.read_activity(*activity_item.guid())?;

        assert_eq!(
            og_activity,
            *read_activity.activity(),
            "Stored activity is not the same as the original activity."
        );

        let new_description = "Updated description";

        let tags = vec!["bla".to_string(), "test".to_string()]
            .into_iter()
            .collect::<HashSet<String>>();

        let new_begin = PaceDateTime::from(
            begin + chrono::TimeDelta::try_seconds(30).ok_or("Invalid time delta")?,
        );

        let updated_activity = Activity::builder()
            .begin(new_begin)
            .kind(ActivityKind::PomodoroWork)
            .status(ActivityStatusKind::InProgress)
            .description(new_description)
            .tags(tags.clone())
            .build();

        let old_activity = storage.update_activity(
            *activity_item.guid(),
            updated_activity,
            UpdateOptions::default(),
        )?;

        assert_eq!(
            og_activity,
            *old_activity.activity(),
            "Stored activity is not the same as the original activity."
        );

        let new_stored_activity = storage.read_activity(*activity_item.guid())?;

        assert_eq!(
            old_activity.guid(),
            new_stored_activity.guid(),
            "ID was updated, but shouldn't."
        );

        assert_eq!(
            new_stored_activity.activity().description(),
            new_description,
            "Description was not updated."
        );

        assert_eq!(
            *new_stored_activity.activity().tags(),
            Some(tags),
            "Tags were not updated, but should."
        );

        assert_eq!(
            old_activity.activity().kind(),
            new_stored_activity.activity().kind(),
            "Kind was updated, but shouldn't."
        );

        assert_eq!(
            &new_begin,
            new_stored_activity.activity().begin(),
            "Begin time was not updated, but should."
        );

        assert!(
            new_stored_activity.activity().is_in_progress(),
            "Activity should be active now, but was not updated."
        );

        Ok(())
    }

    #[test]
    fn test_crud_activity_passes() -> TestResult<()> {
        let storage = InMemoryActivityStorage::new();

        // Create activity
        let begin = Local::now().fixed_offset();
        let kind = ActivityKind::Activity;
        let description = "Test activity";
        let tags = vec!["test".to_string(), "activity".to_string()]
            .into_iter()
            .collect::<HashSet<String>>();

        let mut activity = Activity::builder()
            .begin(begin)
            .kind(kind)
            .description(description)
            .tags(tags)
            .build();

        assert_eq!(
            storage.get_activity_log().activities().len(),
            0,
            "Activity log is not empty."
        );

        let activity_item = storage.begin_activity(activity.clone())?;

        assert_eq!(
            storage.get_activity_log().activities().len(),
            1,
            "Activity was not created."
        );

        // Read activity
        let stored_activity = storage.read_activity(*activity_item.guid())?;

        // Make sure the activity is active now, as begin_activity should make it active automatically
        activity.make_active();

        assert_eq!(
            activity,
            *stored_activity.activity(),
            "Stored activity is not the same as the original activity."
        );

        assert_eq!(
            *stored_activity.activity().status(),
            ActivityStatusKind::InProgress,
            "Activity is not active."
        );

        // Update activity
        let new_description = "Updated description";

        let tags = vec!["bla".to_string(), "test".to_string()]
            .into_iter()
            .collect::<HashSet<String>>();

        let new_begin = PaceDateTime::from(
            begin + chrono::TimeDelta::try_seconds(30).ok_or("Invalid time delta")?,
        );

        let updated_activity = Activity::builder()
            .begin(new_begin)
            .kind(ActivityKind::PomodoroWork)
            .status(ActivityStatusKind::Created)
            .description(new_description)
            .tags(tags.clone())
            .build();

        let _ = storage.update_activity(
            *activity_item.guid(),
            updated_activity,
            UpdateOptions::default(),
        )?;

        let new_stored_activity = storage.read_activity(*activity_item.guid())?;

        assert_eq!(
            new_stored_activity.activity().description(),
            new_description,
            "Description was not updated."
        );

        assert_eq!(
            stored_activity.activity().kind(),
            new_stored_activity.activity().kind(),
            "Kind was updated, but shouldn't."
        );

        assert_eq!(
            Some(tags),
            *new_stored_activity.activity().tags(),
            "Tags were not updated, but should."
        );

        assert_eq!(
            &new_begin,
            new_stored_activity.activity().begin(),
            "Begin time was not updated, but should."
        );

        assert!(
            new_stored_activity.activity().is_inactive(),
            "Activity should be active now, but was not updated."
        );

        // Delete activity
        let deleted_activity =
            storage.delete_activity(*activity_item.guid(), DeleteOptions::default())?;

        assert_eq!(
            storage.get_activity_log().activities().len(),
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

        Ok(())
    }

    #[test]
    fn test_end_single_activity_passes() -> TestResult<()> {
        let storage = InMemoryActivityStorage::new();
        let now = Local::now().fixed_offset();
        let begin_time = now - chrono::TimeDelta::try_seconds(30).ok_or("Invalid time delta")?;
        let end_time = now + chrono::TimeDelta::try_seconds(30).ok_or("Invalid time delta")?;
        let kind = ActivityKind::Activity;
        let description = "Test activity";
        let tags = vec!["test".to_string(), "activity".to_string()]
            .into_iter()
            .collect::<HashSet<String>>();

        let activity = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .tags(tags)
            .build();

        let activity_item = storage.begin_activity(activity.clone())?;

        let end_opts = EndOptions::builder().end_time(end_time).build();

        let ended_activity = storage.end_activity(*activity_item.guid(), end_opts)?;

        assert_ne!(
            activity_item, ended_activity,
            "Activities do match, although they should be different."
        );

        assert!(ended_activity.activity().activity_end_options().is_some());

        let ended_activity = storage.read_activity(*activity_item.guid())?;

        assert!(
            ended_activity.activity().is_completed(),
            "Activity has not ended, but should have."
        );

        assert_eq!(
            activity.tags().as_ref().ok_or("Tags not set.")?,
            ended_activity
                .activity()
                .tags()
                .as_ref()
                .ok_or("Tags not set.")?,
            "Tags were updated, but shouldn't."
        );

        assert_eq!(
            ended_activity
                .activity()
                .activity_end_options()
                .as_ref()
                .ok_or("End options not set.")?
                .end(),
            &PaceDateTime::from(end_time),
            "End time was not set."
        );

        Ok(())
    }

    #[test]
    fn test_end_last_unfinished_activity_passes() -> TestResult<()> {
        let storage = InMemoryActivityStorage::new();
        let now = Local::now().fixed_offset();
        let begin_time = now - chrono::TimeDelta::try_seconds(30).ok_or("Invalid time delta")?;
        let kind = ActivityKind::Activity;
        let description = "Test activity";
        let tags = vec!["test".to_string(), "activity".to_string()]
            .into_iter()
            .collect::<HashSet<String>>();

        let activity = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .tags(tags)
            .build();

        let activity_item = storage.begin_activity(activity.clone())?;

        let ended_activity = storage
            .end_last_unfinished_activity(EndOptions::builder().end_time(now).build())?
            .ok_or("Activity was not ended.")?;

        assert_eq!(
            ended_activity.guid(),
            activity_item.guid(),
            "Activity IDs do not match."
        );

        assert!(
            ended_activity.activity().is_completed(),
            "Activity has not ended, but should have."
        );

        assert_eq!(
            activity.tags().as_ref().ok_or("Tags not set.")?,
            ended_activity
                .activity()
                .tags()
                .as_ref()
                .ok_or("Tags not set.")?,
            "Tags were updated, but shouldn't."
        );

        assert_eq!(
            ended_activity
                .activity()
                .activity_end_options()
                .as_ref()
                .ok_or("End options not set.")?
                .end(),
            &PaceDateTime::from(now),
            "End time was not set."
        );

        Ok(())
    }

    #[test]
    fn test_begin_and_auto_end_for_multiple_activities_passes() -> TestResult<()> {
        let storage = InMemoryActivityStorage::new();
        let now = Local::now().fixed_offset();
        let begin_time = now - chrono::TimeDelta::try_seconds(30).ok_or("Invalid time delta")?;
        let kind = ActivityKind::Activity;
        let description = "Test activity";
        let tags = vec!["test".to_string(), "activity".to_string()]
            .into_iter()
            .collect::<HashSet<String>>();

        let activity = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .tags(tags.clone())
            .build();

        // Begin the first activity
        let activity_item = storage.begin_activity(activity)?;

        let begin_time = now - chrono::TimeDelta::try_seconds(60).ok_or("Invalid time delta.")?;
        let kind = ActivityKind::Activity;
        let description = "Test activity 2";

        let activity2 = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .tags(tags)
            .build();

        // Begin the second activity, the first one should be ended automatically now
        let activity_item2 = storage.begin_activity(activity2)?;

        let ended_activity = storage.read_activity(*activity_item.guid())?;

        assert!(
            ended_activity.activity().is_completed(),
            "Activity has not ended, but should have."
        );

        assert_eq!(
            ended_activity
                .activity()
                .activity_end_options()
                .as_ref()
                .ok_or("End options not set.")?
                .end(),
            &PaceDateTime::from(now),
            "End time was not set."
        );

        let ended_activity2 = storage.read_activity(*activity_item2.guid())?;

        assert!(
            ended_activity2.activity().is_in_progress(),
            "Activity has not ended, but should have."
        );

        assert!(
            ended_activity2.activity().activity_end_options().is_none(),
            "End time should not be set."
        );

        Ok(())
    }

    #[test]
    fn test_hold_most_recent_active_activity_passes() -> TestResult<()> {
        let storage = InMemoryActivityStorage::new();
        let now = Local::now().fixed_offset();
        let begin_time = now - chrono::TimeDelta::try_seconds(30).ok_or("Invalid time delta.")?;
        let kind = ActivityKind::Activity;
        let description = "Test activity";
        let tags = vec!["test".to_string(), "activity".to_string()]
            .into_iter()
            .collect::<HashSet<String>>();

        let activity = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .tags(tags)
            .build();

        let activity_item = storage.begin_activity(activity.clone())?;

        let hold_time = now + chrono::TimeDelta::try_seconds(30).ok_or("Invalid time delta.")?;

        let hold_opts = HoldOptions::builder().begin_time(hold_time).build();

        let held_activity = storage
            .hold_most_recent_active_activity(hold_opts)?
            .ok_or("Activity was not held.")?;

        assert_eq!(
            held_activity.guid(),
            activity_item.guid(),
            "Activity IDs do not match."
        );

        assert_eq!(
            activity.tags().as_ref().ok_or("Tags not set.")?,
            held_activity
                .activity()
                .tags()
                .as_ref()
                .ok_or("Tags not set.")?,
            "Tags were updated, but shouldn't."
        );

        let intermission_guids = storage
            .list_active_intermissions_for_activity_id(*activity_item.guid())?
            .ok_or("Intermission was not created.")?;

        assert_eq!(intermission_guids.len(), 1, "Intermission was not created.");

        let intermission_item = storage.read_activity(intermission_guids[0])?;

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
                .ok_or("Activity kind options not set.")?
                .parent_id()
                .ok_or("Parent ID not set.")?,
            *activity_item.guid(),
            "Parent ID is not set."
        );

        Ok(())
    }

    #[test]
    fn test_hold_last_unfinished_activity_with_existing_intermission_does_nothing_passes(
    ) -> TestResult<()> {
        let storage = InMemoryActivityStorage::new();
        let now = Local::now().fixed_offset();
        let begin_time = now - chrono::TimeDelta::try_seconds(30).ok_or("Invalid time delta.")?;
        let kind = ActivityKind::Activity;
        let description = "Test activity";
        let tags = vec!["test".to_string(), "activity".to_string()]
            .into_iter()
            .collect::<HashSet<String>>();

        let activity = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .tags(tags)
            .build();

        let active_activity_item = storage.begin_activity(activity)?;

        let hold_opts = HoldOptions::builder()
            .begin_time(now + chrono::TimeDelta::try_seconds(30).ok_or("Invalid time delta.")?)
            .build();

        let _held_item = storage
            .hold_most_recent_active_activity(hold_opts)?
            .ok_or("Activity was not held.")?;

        let held_activity = storage.read_activity(*active_activity_item.guid())?;

        assert_eq!(
            *held_activity.activity().status(),
            ActivityStatusKind::Paused,
            "Activity was not held."
        );

        let intermission_guids = storage
            .list_active_intermissions_for_activity_id(*active_activity_item.guid())?
            .ok_or("Intermission was not created.")?;

        assert_eq!(intermission_guids.len(), 1, "Intermission was not created.");

        let hold_opts = HoldOptions::builder()
            .begin_time(now + chrono::TimeDelta::try_seconds(60).ok_or("Invalid time delta.")?)
            .build();

        assert!(
            storage
                .hold_most_recent_active_activity(hold_opts)?
                .is_none(),
            "Activity was held again."
        );

        let intermission_guids = storage
            .list_active_intermissions_for_activity_id(*active_activity_item.guid())?
            .ok_or("Intermission was not created.")?;

        assert_eq!(
            intermission_guids.len(),
            1,
            "Intermission was created again."
        );

        let intermission_item = storage.read_activity(intermission_guids[0])?;

        assert_eq!(
            *intermission_item.activity().kind(),
            ActivityKind::Intermission,
            "Intermission was not created."
        );

        assert!(
            intermission_item.activity().tags().is_none(),
            "Intermission has tags, but shouldn't."
        );

        Ok(())
    }

    #[test]
    fn test_end_all_active_intermissions_passes() -> TestResult<()> {
        let storage = InMemoryActivityStorage::new();
        let now = Local::now().fixed_offset();
        let begin_time = now - chrono::TimeDelta::try_seconds(30).ok_or("Invalid time delta.")?;
        let end_time = now + chrono::TimeDelta::try_seconds(60).ok_or("Invalid time delta.")?;
        let kind = ActivityKind::Activity;
        let description = "Test activity";

        let activity = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .build();

        let active_activity_item = storage.begin_activity(activity)?;

        let hold_opts = HoldOptions::builder()
            .begin_time(now + chrono::TimeDelta::try_seconds(30).ok_or("Invalid time delta.")?)
            .build();

        let _ = storage.hold_most_recent_active_activity(hold_opts)?;

        let intermission_guids = storage
            .list_active_intermissions_for_activity_id(*active_activity_item.guid())?
            .ok_or("Intermission was not created.")?;

        assert_eq!(intermission_guids.len(), 1, "Intermission was not created.");

        let end_opts = EndOptions::builder().end_time(end_time).build();

        let ended_intermissions = storage
            .end_all_active_intermissions(end_opts)?
            .ok_or("Intermissions were not ended.")?;

        assert_eq!(
            ended_intermissions.len(),
            1,
            "Not all intermissions were ended."
        );

        let ended_intermission = storage.read_activity(intermission_guids[0])?;

        assert!(
            ended_intermission.activity().is_completed(),
            "Intermission has not ended, but should have."
        );

        assert_eq!(
            ended_intermission
                .activity()
                .activity_end_options()
                .as_ref()
                .ok_or("End options not set.")?
                .end(),
            &PaceDateTime::from(end_time),
            "End time was not set."
        );

        Ok(())
    }

    #[test]
    fn test_group_activities_by_keywords_passes() -> TestResult<()> {
        let storage = InMemoryActivityStorage::new();
        let now = Local::now().fixed_offset();
        let begin_time = now - chrono::TimeDelta::try_seconds(30).ok_or("Invalid time delta")?;
        let kind = ActivityKind::Activity;
        let description = "Test activity";

        let activity = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .category("Project::Test".to_string())
            .build();

        let activity_item = storage.begin_activity(activity)?;

        let keyword_opts = KeywordOptions::builder().category("Test").build();

        let grouped_activities = storage.group_activities_by_keywords(keyword_opts)?.ok_or(
            "Grouped activities by keywords returned None, but should have returned Some.",
        )?;

        assert_eq!(
            grouped_activities.len(),
            1,
            "Grouped activities do not match the amount of created activities."
        );

        let grouped_activity = grouped_activities
            .values()
            .next()
            .ok_or("Grouped activities are empty.")?
            .first()
            .ok_or("Grouped activities are empty.")?
            .clone();

        assert_eq!(
            *grouped_activity.guid(),
            *activity_item.guid(),
            "Grouped activity is not the same as the original activity."
        );

        Ok(())
    }

    #[test]
    fn test_group_activities_by_kind_passes() -> TestResult<()> {
        let storage = InMemoryActivityStorage::new();
        let now = Local::now().fixed_offset();
        let begin_time = now - chrono::TimeDelta::try_seconds(30).ok_or("Invalid time delta")?;
        let kind = ActivityKind::Activity;
        let description = "Test activity";

        let activity = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .build();

        let activity_item = storage.begin_activity(activity)?;

        let grouped_activities = storage
            .group_activities_by_kind()?
            .ok_or("Grouped activities by kind returned None, but should have returned Some.")?;

        assert_eq!(
            grouped_activities.len(),
            1,
            "Grouped activities do not match the amount of created activities."
        );

        let grouped_activity = grouped_activities
            .values()
            .next()
            .ok_or("Grouped activities are empty.")?
            .first()
            .ok_or("Grouped activities are empty.")?
            .clone();

        assert_eq!(
            *grouped_activity.guid(),
            *activity_item.guid(),
            "Grouped activity is not the same as the original activity."
        );

        assert_eq!(
            *grouped_activity.activity().kind(),
            kind,
            "Grouped activity kind is not the same as the original activity kind."
        );

        assert_eq!(
            *grouped_activity.activity().description(),
            description,
            "Grouped activity description is not the same as the original activity description."
        );

        Ok(())
    }

    #[test]
    fn test_group_activities_by_status_passes() -> TestResult<()> {
        let storage = InMemoryActivityStorage::new();
        let now = Local::now().fixed_offset();
        let begin_time = now - chrono::TimeDelta::try_seconds(30).ok_or("Invalid time delta")?;
        let kind = ActivityKind::Activity;
        let description = "Test activity";

        let activity = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .build();

        let activity_item = storage.begin_activity(activity)?;

        let grouped_activities = storage
            .group_activities_by_status()?
            .ok_or("Grouped activities by status returned None, but should have returned Some.")?;

        assert_eq!(
            grouped_activities.len(),
            1,
            "Grouped activities do not match the amount of created activities."
        );

        let grouped_activity = grouped_activities
            .values()
            .next()
            .ok_or("Grouped activities are empty.")?
            .first()
            .ok_or("Grouped activities are empty.")?
            .clone();

        assert_eq!(
            *grouped_activity.guid(),
            *activity_item.guid(),
            "Grouped activity is not the same as the original activity."
        );

        assert_eq!(
            *grouped_activity.activity().status(),
            ActivityStatusKind::InProgress,
            "Grouped activity status is not the same as the original activity status."
        );

        assert_eq!(
            *grouped_activity.activity().kind(),
            kind,
            "Grouped activity kind is not the same as the original activity kind."
        );

        assert_eq!(
            *grouped_activity.activity().description(),
            description,
            "Grouped activity description is not the same as the original activity description."
        );

        Ok(())
    }

    #[test]
    fn test_group_activities_by_start_date_passes() -> TestResult<()> {
        let storage = InMemoryActivityStorage::new();
        let now = Local::now().fixed_offset();
        let begin_time = now - chrono::TimeDelta::try_seconds(30).ok_or("Invalid time delta.")?;
        let kind = ActivityKind::Activity;
        let description = "Test activity";

        let activity = Activity::builder()
            .begin(begin_time)
            .kind(kind)
            .description(description)
            .build();

        let activity_item = storage.begin_activity(activity)?;

        let grouped_activities = storage.group_activities_by_start_date()?.ok_or(
            "Grouped activities by start date returned None, but should have returned Some.",
        )?;

        assert_eq!(
            grouped_activities.len(),
            1,
            "Grouped activities do not match the amount of created activities."
        );

        let grouped_activity = grouped_activities
            .values()
            .next()
            .ok_or("Grouped activities are empty?")?
            .first()
            .ok_or("Grouped activities are empty?")?
            .clone();

        assert_eq!(
            *grouped_activity.guid(),
            *activity_item.guid(),
            "Grouped activity is not the same as the original activity."
        );

        assert_eq!(
            grouped_activity.activity().begin().date_naive(),
            PaceDate::new(begin_time.date_naive()),
            "Grouped activity date is not the same as the original activity date."
        );

        assert_eq!(
            *grouped_activity.activity().kind(),
            kind,
            "Grouped activity kind is not the same as the original activity kind."
        );

        assert_eq!(
            *grouped_activity.activity().description(),
            description,
            "Grouped activity description is not the same as the original activity description."
        );

        Ok(())
    }

    // TODO!: Implement the following tests
    // #[test]
    // fn test_group_multiple_activities_by_status_passes() {
    // }

    // #[test]
    // fn test_group_multiple_activities_by_kind_passes() {
    // }

    // #[test]
    // fn test_group_multiple_activities_by_keywords_passes() {
    // }
}
