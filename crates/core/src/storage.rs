// #[cfg(feature = "rusqlite")]
// pub mod rusqlite;

use itertools::Itertools;
use std::{
    collections::BTreeMap,
    fmt::{self, Debug, Formatter},
};
use tracing::debug;

use pace_error::{PaceOptResult, PaceResult};
use pace_time::{date::PaceDate, duration::PaceDurationRange, time_range::TimeRangeOptions};

use crate::{
    domain::{
        activity::{Activity, ActivityItem, ActivityKind},
        filter::{ActivityFilterKind, FilteredActivities},
        id::ActivityGuid,
        status::ActivityStatusKind,
    },
    options::{
        DeleteOptions, EndOptions, HoldOptions, KeywordOptions, ResumeOptions, UpdateOptions,
    },
};

impl Debug for dyn ActivityStorage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ActivityStorage: {}", self.identify())
    }
}

/// A type of storage that can be synced to a persistent medium.
///
/// This is useful for in-memory storage that needs to be persisted to disk or a database.
pub trait SyncStorage {
    /// Sync the storage to a persistent medium.
    ///
    /// # Errors
    ///
    /// This function should return an error if the storage cannot be synced.
    ///
    /// # Returns
    ///
    /// If the storage was synced successfully it should return `Ok(())`.
    fn sync(&self) -> PaceResult<()>;
}

/// The trait that all storage backends must implement. This allows us to swap out the storage
/// backend without changing the rest of the application.
///
/// Storage backends can be in-memory, on-disk, or in a database. They can be any kind of
/// persistent storage that can be used to store activities.
pub trait ActivityStorage:
    ActivityReadOps + ActivityWriteOps + ActivityStateManagement + SyncStorage + ActivityQuerying
// TODO!: Implement other traits
// + ActivityTagging
// + ActivityArchiving
// + ActivityStatistics
{
    // This main trait combines all aspects of activity storage.
    // You can add methods here that require access to multiple areas of functionality,
    // or simply use it as a marker trait for objects that implement all aspects of activity storage.

    /// Setup the storage backend. This is called once when the application starts.
    ///
    /// This is where you would create the database tables, open the file, etc.
    ///
    /// # Errors
    ///
    /// This function should return an error if the storage backend cannot be setup.
    fn setup(&self) -> PaceResult<()>;

    /// Teardown the storage backend. This is called once when the application stops.
    ///
    /// This is where you would close the database connection, save the file, etc.
    ///
    /// # Errors
    ///
    /// This function should return an error if the storage backend cannot be torn down.
    ///
    /// # Returns
    ///
    /// If the storage backend was torn down successfully it should return `Ok(())`.
    fn teardown(&self) -> PaceResult<()>;

    /// Identify the storage backend.
    ///
    /// This is useful for logging and debugging purposes.
    ///
    /// # Returns
    ///
    /// The identifier of the storage backend.
    fn identify(&self) -> String;
}

/// Basic Read Operations for Activities in the storage backend.
///
/// Read operations are essential for loading activities from the storage backend.
/// These operations are used to get activities by their ID, list all activities, or filter activities by a specific criterion.
/// They are also used to get the current state of activities, such as the currently active activities.
pub trait ActivityReadOps {
    /// Read an activity from the storage backend.
    ///
    /// # Arguments
    ///
    /// * `activity_id` - The ID of the activity to read.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be read.
    ///
    /// # Returns
    ///
    /// The activity that was read from the storage backend. If no activity is found, it should return `Ok(None)`.
    fn read_activity(&self, activity_id: ActivityGuid) -> PaceResult<ActivityItem>;

    /// List activities from the storage backend.
    ///
    /// # Arguments
    ///
    /// * `filter` - The filter to apply to the activities.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activities cannot be loaded.
    ///
    /// # Returns
    ///
    /// A collection of the activities that were loaded from the storage backend. Returns Ok(None) if no activities are found.
    fn list_activities(&self, filter: ActivityFilterKind) -> PaceOptResult<FilteredActivities>;
}

/// Basic CUD Operations for Activities in the storage backend.
///
/// CUD stands for Create, Update, and Delete. These are the basic operations that can be performed on activities.
/// These operations are essential for managing activities in the storage backend.
pub trait ActivityWriteOps: ActivityReadOps {
    /// Create an activity in the storage backend.
    ///
    /// # Arguments
    ///
    /// * `activity` - The activity to create.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be created.
    ///
    /// # Returns
    ///
    /// If the activity was created successfully it should return the ID of the created activity.
    fn create_activity(&self, activity: Activity) -> PaceResult<ActivityItem>;

    /// Update an existing activity in the storage backend.
    ///
    /// # Note
    ///
    /// This function should not be used to update the state of an activity (e.g., start, end, hold, resume) directly.
    /// Use the `ActivityStateManagement` trait for that. This function is only for updating (as in replacing) the complete
    /// data of an activity.
    ///
    /// Warning: It can't be used to update the ID of an activity, because that's the primary key.
    /// So it is immutable.
    ///      
    /// # Arguments
    ///
    /// * `activity_id` - The ID of the activity to update.
    /// * `activity` - The updated activity data.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be updated.
    ///
    /// # Returns
    ///
    /// If the activity was updated successfully it should return the original activity before it was updated.
    fn update_activity(
        &self,
        activity_id: ActivityGuid,
        updated_activity: Activity,
        update_opts: UpdateOptions,
    ) -> PaceResult<ActivityItem>;

    /// Delete an activity from the storage backend.
    ///
    /// # Arguments
    ///
    /// * `activity_id` - The ID of the activity to delete.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be deleted.
    ///
    /// # Returns
    ///
    /// If the activity was deleted successfully it should return the activity that was deleted.
    fn delete_activity(
        &self,
        activity_id: ActivityGuid,
        delete_opts: DeleteOptions,
    ) -> PaceResult<ActivityItem>;
}

/// Managing Activity State
///
/// Managing activity state is a way to start, end, hold, or resume activities in the storage backend.
/// This is useful for keeping track of the current state of activities and making sure they are properly managed.
///
/// For example, you might want to start a new activity, end an activity that is currently running, or hold an activity temporarily.
pub trait ActivityStateManagement: ActivityReadOps + ActivityWriteOps + ActivityQuerying {
    /// Begin an activity in the storage backend. This makes the activity active.
    ///
    /// # Arguments
    ///
    /// * `activity` - The activity to start.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be started.
    ///
    /// # Returns
    ///
    /// If the activity was started successfully it should return the ID of the started activity.
    fn begin_activity(&self, mut activity: Activity) -> PaceResult<ActivityItem> {
        // End all unfinished activities before starting a new one,
        // we don't want to have multiple activities running at the same time
        let _ = self.end_all_activities(EndOptions::default())?;

        // Make the current activity active
        activity.make_active();

        // Create the activity in the storage backend
        self.create_activity(activity)
    }

    /// Hold an activity in the storage backend.
    ///
    /// # Arguments
    ///
    /// * `activity_id` - The ID of the activity to hold.
    /// * `hold_opts` - The options to hold the activity.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be held.
    ///
    /// # Returns
    ///
    /// If the activity was held successfully it should return the `ActivityItem` of the held activity.
    fn hold_activity(
        &self,
        activity_id: ActivityGuid,
        hold_opts: HoldOptions,
    ) -> PaceResult<ActivityItem>;

    /// Resume an activity in the storage backend.
    ///
    /// # Arguments
    ///
    /// * `activity_id` - The ID of the activity to resume. If `None`, the last unfinished activity is resumed.
    /// * `resume_time` - The time (HH:MM) to resume the activity at. If `None`, the current time is used.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be resumed.
    ///
    /// # Returns
    ///
    /// The activity that was resumed. Returns Ok(None) if no activity was resumed.
    fn resume_activity(
        &self,
        activity_id: ActivityGuid,
        resume_opts: ResumeOptions,
    ) -> PaceResult<ActivityItem>;

    /// Resume the most recent activity in the storage backend.
    ///
    /// # Arguments
    ///
    /// * `resume_opts` - The options to resume the activity.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be resumed.
    ///
    /// # Returns
    ///
    /// The activity that was resumed. Returns Ok(None) if no activity was resumed.
    fn resume_most_recent_activity(
        &self,
        resume_opts: ResumeOptions,
    ) -> PaceOptResult<ActivityItem>;

    /// End an activity in the storage backend.
    ///
    /// # Arguments
    ///
    /// * `activity_id` - The ID of the activity to end.
    /// * `end_opts` - The options to end the activity.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be ended.
    ///
    /// # Returns
    ///
    /// If the activity was ended successfully it should return the ID of the ended activity.
    fn end_activity(
        &self,
        activity_id: ActivityGuid,
        end_opts: EndOptions,
    ) -> PaceResult<ActivityItem>;

    /// End all activities in the storage backend.
    ///
    /// # Arguments
    ///
    /// * `end_opts` - The options to end the activities.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activities cannot be ended.
    ///
    /// # Returns
    ///
    /// A collection of the activities that were ended. Returns Ok(None) if no activities were ended.
    fn end_all_activities(&self, end_opts: EndOptions) -> PaceOptResult<Vec<ActivityItem>>;

    /// End all active intermissions in the storage backend.
    ///
    /// # Arguments
    ///
    /// * `end_opts` - The options to end the intermissions.
    ///
    /// # Errors
    ///
    /// This function should return an error if the intermissions cannot be ended.
    ///
    /// # Returns
    ///
    /// A collection of the intermissions that were ended. Returns Ok(None) if no intermissions were ended.
    fn end_all_active_intermissions(
        &self,
        end_opts: EndOptions,
    ) -> PaceOptResult<Vec<ActivityGuid>>;

    /// End the last unfinished activity in the storage backend.
    ///
    /// # Arguments
    ///
    /// * `end_opts` - The options to end the activity.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be ended.
    ///
    /// # Returns
    ///
    /// The activity that was ended. Returns Ok(None) if no activity was ended.
    fn end_last_unfinished_activity(&self, end_opts: EndOptions) -> PaceOptResult<ActivityItem>;

    /// Hold the most recent activity that is active in the storage backend.
    ///
    /// # Arguments
    ///
    /// * `hold_opts` - The options to hold the activity.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be held or if there are no activities to hold.
    ///
    /// # Returns
    ///
    /// The activity that was held if there was an unfinished activity to hold.
    /// If there are no activities to hold, it should return `Ok(None)`.
    ///
    /// # Note
    ///
    /// This function should not be used to hold an activity that is already held. It should only be used to hold the last unfinished activity.
    fn hold_most_recent_active_activity(
        &self,
        hold_opts: HoldOptions,
    ) -> PaceOptResult<ActivityItem>;
}

/// Querying Activities
///
/// Querying activities is a way to get information about them from the storage backend.
/// This is useful for getting a list of activities, finding activities within a specific
/// date range, or getting activities by their ID.
///
/// For example, you might want to list all activities that are currently active,
/// find all activities within a specific date range, or get a specific activity by its ID.
pub trait ActivityQuerying: ActivityReadOps {
    /// Group activities by predefined duration ranges (e.g., short, medium, long).
    ///
    /// This is useful for analyzing how time is spent on different activities.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activities cannot be loaded.
    ///
    /// # Returns
    ///
    /// A collection of the activities that are grouped by their duration range.
    /// The key is the duration range, and the value is a list of activities that
    /// fall within that range.
    /// If no activities are found, it should return `Ok(None)`.
    // TODO!: This method requires defining what constitutes short, medium, and long durations.
    fn group_activities_by_duration_range(
        &self,
    ) -> PaceOptResult<BTreeMap<PaceDurationRange, Vec<ActivityItem>>>;

    /// Group activities by their start date. This can help in analyzing how
    /// activities are distributed over time.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activities cannot be loaded.
    ///
    /// # Returns
    ///
    /// A collection of the activities that are grouped by their start date.
    /// The key is the start date of the activity, and the value is a list of activities
    /// that started on that date.
    // TODO!: Do we actually also want to group by end date? If so, we might want to introduce a new type
    // TODO!: for the groupings, so we can distinguish between start and end date groupings.
    fn group_activities_by_start_date(
        &self,
    ) -> PaceOptResult<BTreeMap<PaceDate, Vec<ActivityItem>>>;

    /// Retrieve activities that have one or more intermissions, useful for identifying
    /// potential inefficiencies or breaks.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activities cannot be loaded.
    ///
    /// # Returns
    ///
    /// A collection of the activities that have intermissions.
    /// The key is the ID of the activity, and the value is a list of intermissions.
    /// If no activities are found, it should return `Ok(None)`.
    fn list_activities_with_intermissions(
        &self,
    ) -> PaceOptResult<BTreeMap<ActivityGuid, Vec<ActivityItem>>>;

    /// Group activities based on keywords, e.g., category, tags, etc.
    ///
    /// This is useful for analyzing time spent on different projects or areas of work.
    ///
    /// # Arguments
    ///
    /// * `keyword_opts` - The keyword options to filter the activities by.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activities cannot be loaded.
    ///
    /// # Returns
    ///
    /// A collection of the activities that are matching the `KeywordOptions`.
    /// The key is the keyword, and the value is a list of matching activities.
    /// If no activities are found, it should return `Ok(None)`.
    fn group_activities_by_keywords(
        &self,
        keyword_opts: KeywordOptions,
    ) -> PaceOptResult<BTreeMap<String, Vec<ActivityItem>>>;

    /// Group activities based on their kind (e.g., Task, Intermission).
    ///
    /// # Errors
    ///
    /// This function should return an error if the activities cannot be loaded.
    ///
    /// # Returns
    ///
    /// A collection of the activities with their kind.
    /// The key is the kind of the activity, and the value is a list of activities of that kind.
    /// If no activities are found, it should return `Ok(None)`.
    fn group_activities_by_kind(&self) -> PaceOptResult<BTreeMap<ActivityKind, Vec<ActivityItem>>>;

    /// List activities by time range from the storage backend.
    ///
    /// # Arguments
    ///
    /// * `time_range_opts` - The range options to filter the activities by.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activities cannot be loaded.
    ///
    /// # Returns
    ///
    /// A collection of the activities guids that are matching the `RangeOptions`.
    /// If no activities are found, it should return `Ok(None)`.
    fn list_activities_by_time_range(
        &self,
        time_range_opts: TimeRangeOptions,
    ) -> PaceOptResult<Vec<ActivityGuid>>;

    /// Group activities by their status from the storage backend.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activities cannot be loaded.
    ///
    /// # Returns
    ///
    /// A collection of the activities by their status.
    /// If no activities are found, it should return `Ok(None)`.
    fn group_activities_by_status(
        &self,
    ) -> PaceOptResult<BTreeMap<ActivityStatusKind, Vec<ActivityItem>>>;

    /// List all current activities from the storage backend matching an `ActivityFilter`.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activities cannot be loaded.
    /// In case of no activities, it should return `Ok(None)`.
    ///
    /// # Returns
    ///
    /// A collection of the activities that are matching the `ActivityFilter`.
    fn list_current_activities(
        &self,
        filter: ActivityFilterKind,
    ) -> PaceOptResult<Vec<ActivityGuid>> {
        Ok(self
            .list_activities(filter)?
            .map(FilteredActivities::into_vec))
    }

    /// Get all activities by their ID.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activities cannot be loaded.
    ///
    /// # Returns
    ///
    /// A collection of the activities that were loaded from the storage backend by their ID in a `BTreeMap`.
    /// If no activities are found, it should return `Ok(None)`.
    fn list_activities_by_id(&self) -> PaceOptResult<BTreeMap<ActivityGuid, Activity>>;

    /// List all active intermissions from the storage backend.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activities cannot be loaded.
    ///
    /// # Returns
    ///
    /// A collection of the activities that are currently active intermissions.
    /// If no activities are found, it should return `Ok(None)`.
    fn list_active_intermissions(&self) -> PaceOptResult<Vec<ActivityGuid>> {
        Ok(self
            .list_activities(ActivityFilterKind::ActiveIntermission)?
            .map(FilteredActivities::into_vec))
    }

    /// List the most recent activities from the storage backend.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of activities to list.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activities cannot be loaded.
    ///
    /// # Returns
    ///
    /// A collection of the most recent activities.
    /// If no activities are found, it should return `Ok(None)`.
    fn list_most_recent_activities(&self, count: usize) -> PaceOptResult<Vec<ActivityGuid>> {
        let filtered = self
            .list_activities(ActivityFilterKind::OnlyActivities)?
            .map(FilteredActivities::into_vec);

        let Some(filtered) = filtered else {
            debug!("No recent activities found");

            return Ok(None);
        };

        if filtered.len() > count {
            debug!("Found more than {count} recent activities, dropping some...");

            Ok(Some(
                (*filtered)
                    .iter()
                    .sorted()
                    .rev()
                    .take(count)
                    .rev()
                    .copied()
                    .collect(),
            ))
        } else {
            debug!("Found {} recent activities", filtered.len());
            Ok(Some(filtered))
        }
    }

    /// Check if an activity is currently active.
    ///
    /// # Arguments
    ///
    /// * `activity_id` - The ID of the activity to check.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be checked.
    ///
    /// # Returns
    ///
    /// If the activity is active, it should return `Ok(true)`. If it is not active, it should return `Ok(false)`.
    fn is_activity_active(&self, activity_id: ActivityGuid) -> PaceResult<bool> {
        let activity = self.read_activity(activity_id)?;

        debug!(
            "Checking if Activity with id {:?} is active: {}",
            activity_id,
            if activity.activity().is_in_progress() {
                "yes"
            } else {
                "no"
            }
        );

        Ok(activity.activity().is_in_progress())
    }

    /// List all intermissions for an activity id from the storage backend.
    ///
    /// # Arguments
    ///
    /// * `activity_id` - The ID of the activity to list intermissions for.
    ///
    /// # Errors
    ///
    /// This function should return an error if the intermissions cannot be loaded.
    ///
    /// # Returns
    ///
    /// A collection of the intermissions for the activity.
    /// If no intermissions are found, it should return `Ok(None)`.
    fn list_intermissions_for_activity_id(
        &self,
        activity_id: ActivityGuid,
    ) -> PaceOptResult<Vec<ActivityItem>> {
        let Some(filtered) = self
            .list_activities(ActivityFilterKind::Intermission)?
            .map(FilteredActivities::into_vec)
        else {
            debug!("No intermissions found.");
            return Ok(None);
        };

        let intermissions = filtered
            .iter()
            .filter_map(|activity| {
                let activity_item = self.read_activity(*activity).ok()?;

                if activity_item.activity().parent_id() == Some(activity_id) {
                    debug!("Found intermission for activity: {activity_id}");
                    Some(activity_item)
                } else {
                    debug!("Not an intermission for activity: {activity_id}");
                    None
                }
            })
            .collect::<Vec<ActivityItem>>();

        if intermissions.is_empty() {
            debug!("No intermissions found for activity: {activity_id}");
            return Ok(None);
        }

        debug!("Activity with id {activity_id:?} has intermissions: {intermissions:?}");

        Ok(Some(intermissions))
    }

    /// Check if an activity currently has one or more active intermissions.
    ///
    /// # Arguments
    ///
    /// * `activity_id` - The ID of the activity to check.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be checked.
    ///
    /// # Returns
    ///
    /// If the activity has active intermissions, it should return `Ok(Option<VecDeque<ActivityGuid>>)` with the IDs of the active intermissions.
    /// If it has no active intermissions, it should return `Ok(None)`.
    fn list_active_intermissions_for_activity_id(
        &self,
        activity_id: ActivityGuid,
    ) -> PaceOptResult<Vec<ActivityGuid>> {
        let guids = self.list_active_intermissions()?.map(|log| {
            log.iter()
                .filter_map(|active_intermission_id| {
                    if self
                        .read_activity(*active_intermission_id)
                        .ok()?
                        .activity()
                        .parent_id()
                        == Some(activity_id)
                    {
                        debug!("Found active intermission for activity: {activity_id}");
                        Some(*active_intermission_id)
                    } else {
                        debug!("No active intermission found for activity: {activity_id}");
                        None
                    }
                })
                .collect::<Vec<ActivityGuid>>()
        });

        debug!("Activity with id {activity_id:?} has active intermissions: {guids:?}");

        Ok(guids)
    }

    /// Get the latest active activity.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be loaded.
    ///
    /// # Returns
    ///
    /// The latest active activity.
    /// If no activity is found, it should return `Ok(None)`.
    fn most_recent_active_activity(&self) -> PaceOptResult<ActivityItem> {
        let Some(current) = self.list_current_activities(ActivityFilterKind::Active)? else {
            debug!("No active activities found");

            // There are no active activities at all
            return Ok(None);
        };

        current
            .into_iter()
            .sorted()
            .rev()
            .find(|activity_id| {
                self.read_activity(*activity_id)
                    .map(|activity| {
                        activity.activity().is_in_progress()
                            && activity.activity().kind().is_activity()
                            && !activity.activity().is_active_intermission()
                    })
                    .unwrap_or(false)
            })
            .map(|activity_id| self.read_activity(activity_id))
            .transpose()
    }

    /// Get the latest held activity.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be loaded.
    ///
    /// # Returns
    ///
    /// The latest held activity.
    /// If no activity is found, it should return `Ok(None)`.
    fn most_recent_held_activity(&self) -> PaceOptResult<ActivityItem> {
        let Some(current) = self.list_current_activities(ActivityFilterKind::Held)? else {
            debug!("No held activities found");

            // There are no active activities at all
            return Ok(None);
        };

        current
            .into_iter()
            .sorted()
            .rev()
            .find(|activity_id| {
                self.read_activity(*activity_id)
                    .map(|activity| {
                        activity.activity().is_paused()
                            && activity.activity().kind().is_activity()
                            && !activity.activity().is_active_intermission()
                    })
                    .unwrap_or(false)
            })
            .map(|activity_id| self.read_activity(activity_id))
            .transpose()
    }
}

// /// Tagging Activities
// ///
// /// Tagging activities is a way to categorize them. This is useful for grouping activities together that have something in common.
// /// For example, you might want to tag all activities that are related to a specific project, or all activities that are related to a specific client.
// /// Tags can be used to generate statistics or summaries of activities, or to filter activities by a specific tag.
// pub trait ActivityTagging {
//     /// Add a tag to an activity.
//     ///
//     /// # Arguments
//     ///
//     /// * `activity_id` - The ID of the activity to tag.
//     /// * `tag` - The tag to add.
//     ///
//     /// # Errors
//     ///
//     /// This function should return an error if the tag cannot be added.
//     ///
//     /// # Returns
//     ///
//     /// If the tag was added successfully it should return `Ok(())`.
//     fn add_tag_to_activity(&self, activity_id: ActivityGuid, tag: &str) -> PaceResult<()>;

//     /// Remove a tag from an activity.
//     ///
//     /// # Arguments
//     ///
//     /// * `activity_id` - The ID of the activity to untag.
//     /// * `tag` - The tag to remove.
//     ///
//     /// # Errors
//     ///
//     /// This function should return an error if the tag cannot be removed.
//     ///
//     /// # Returns
//     ///
//     /// If the tag was removed successfully it should return `Ok(())`.
//     fn remove_tag_from_activity(&self, activity_id: ActivityGuid, tag: &str) -> PaceResult<()>;
// }

// /// Archiving Activities
// ///
// /// Archiving activities is a way to remove them from the main list of activities, but still keep them around for reference.
// /// This is useful for activities that are no longer relevant, but you still want to keep them around for historical purposes.
// ///
// /// For example, you might want to archive all activities from a previous year to keep the main list of activities clean and relevant.
// /// Archiving is different from deleting an activity, as it doesn't remove the activity from the system, it just moves it to a different list.
// pub trait ActivityArchiving {
//     /// Archive an activity.
//     ///
//     /// # Arguments
//     ///
//     /// * `activity_id` - The ID of the activity to archive.
//     ///
//     /// # Errors
//     ///
//     /// This function should return an error if the activity cannot be archived.
//     ///
//     /// # Returns
//     ///
//     /// If the activity was archived successfully it should return `Ok(())`.
//     fn archive_activity(&self, activity_id: ActivityGuid) -> PaceResult<()>;

//     /// Unarchive an activity.
//     ///
//     /// # Arguments
//     ///
//     /// * `activity_id` - The ID of the activity to unarchive.
//     ///
//     /// # Errors
//     ///
//     /// This function should return an error if the activity cannot be unarchived.
//     ///
//     /// # Returns
//     ///
//     /// If the activity was unarchived successfully it should return `Ok(())`.
//     fn unarchive_activity(&self, activity_id: ActivityGuid) -> PaceResult<()>;
// }

// /// Generate Statistics for Activities
// ///
// /// Generating statistics for activities is a way to summarize them and get insights into your activities.
// /// This is useful for understanding how you spend your time and how productive you are.
// ///
// /// For example, you might want to generate statistics for all activities within a specific time frame, such as daily, weekly, or monthly.
// /// Statistics can include things like the total time spent on activities, the average time spent on activities, the most active days, etc.
// pub trait ActivityStatistics {
//     /// Generate statistics or summary of activities.
//     ///
//     /// # Arguments
//     ///
//     /// * `time_frame` - The time frame to generate statistics for (e.g., daily, weekly, monthly).
//     ///
//     /// # Errors
//     ///
//     /// This function should return an error if the statistics cannot be generated.
//     ///
//     /// # Returns
//     ///
//     /// A summary or statistics of activities within the specified time frame.
//     fn generate_activity_statistics(&self, time_frame: TimeFrame) -> PaceResult<ReviewSummary>;
// }

// /// Reviewing Activities
// ///
// /// Reviewing activities is a way to look back at your activities and get insights into how you've been spending your time.
// /// This is useful for understanding how productive you are, identifying patterns in your activities, and finding areas for improvement.
// ///
// /// For example, you might want to review all activities within a specific time frame, such as daily, weekly, or monthly.
// /// Reviews can include things like the total time spent on activities, the average time spent on activities, the most active days, etc.
// pub trait ActivityReview {
//     /// Review activities within a specific date range.
//     ///
//     /// # Arguments
//     ///
//     /// * `start_date` - The start date of the range.
//     /// * `end_date` - The end date of the range.
//     ///
//     /// # Errors
//     ///
//     /// This function should return an error if the activities cannot be loaded.
//     ///
//     /// # Returns
//     ///
//     /// A collection of the activities that fall within the specified date range.
//     fn review_activities_in_date_range(
//         &self,
//         start: PaceDateTime,
//         end: PaceDateTime,
//     ) -> PaceResult<ActivityLog>;
// }
