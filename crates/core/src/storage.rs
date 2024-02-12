use std::collections::{BTreeMap, VecDeque};

use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        activity::{Activity, ActivityDequeCollection, ActivityId, ActivityLog},
        filter::{ActivityFilter, FilteredActivities},
        review::ActivityStats,
        time::TimeFrame,
    },
    error::{PaceErrorKind, PaceResult},
};

pub mod file;
pub mod in_memory;
// TODO: Implement conversion FromSQL and ToSQL
// pub mod sqlite;

/// The trait that all storage backends must implement. This allows us to swap out the storage
/// backend without changing the rest of the application.
pub trait ActivityStorage: ActivityReadOps + ActivityWriteOps + ActivityStateManagement
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
    fn setup_storage(&self) -> PaceResult<()>;
}

/// Basic Read Operations for Activities in the storage backend.
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
    fn read_activity(&self, activity_id: ActivityId) -> PaceResult<Option<Activity>>;

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
    /// A collection of the activities that were loaded from the storage backend.
    fn list_activities(&self, filter: ActivityFilter) -> PaceResult<Option<FilteredActivities>>;
}

/// Basic CRUD Operations for Activities in the storage backend.
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
    fn create_activity(&self, activity: &Activity) -> PaceResult<ActivityId>;

    /// Update an existing activity in the storage backend.
    ///
    /// # Arguments
    ///
    /// * `activity_id` - The ID of the activity to update.
    /// * `activity` - The updated activity data.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be updated.
    fn update_activity(&self, activity_id: ActivityId, activity: &Activity) -> PaceResult<()>;

    /// Delete an activity from the storage backend.
    ///
    /// # Arguments
    ///
    /// * `activity_id` - The ID of the activity to delete.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be deleted.
    fn delete_activity(&self, activity_id: ActivityId) -> PaceResult<()>;
}

/// Managing Activity State
pub trait ActivityStateManagement: ActivityReadOps + ActivityWriteOps {
    /// Start an activity in the storage backend.
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
    fn start_activity(&self, activity: &Activity) -> PaceResult<ActivityId>;

    /// End an activity in the storage backend.
    ///
    /// # Arguments
    ///
    /// * `activity_id` - The ID of the activity to end.
    /// * `end_time` - The time (HH:MM) to end the activity at. If `None`, the current time is used.
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
        activity_id: ActivityId,
        end_time: Option<NaiveTime>,
    ) -> PaceResult<ActivityId>;

    /// End all unfinished activities in the storage backend.
    ///
    /// # Arguments
    ///
    /// * `time` - The time (HH:MM) to end the activities at. If `None`, the current time is used.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activities cannot be ended.
    ///
    /// # Returns
    ///
    /// A collection of the activities that were ended.
    fn end_all_unfinished_activities(
        &self,
        time: Option<NaiveTime>,
    ) -> PaceResult<Option<Vec<Activity>>>;

    /// End the last unfinished activity in the storage backend.
    ///
    /// # Arguments
    ///
    /// * `time` - The time (HH:MM) to end the activity at. If `None`, the current time is used.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activity cannot be ended.
    ///
    /// # Returns
    ///
    /// The activity that was ended.
    fn end_last_unfinished_activity(&self, time: Option<NaiveTime>)
        -> PaceResult<Option<Activity>>;
}

/// Querying Activities
pub trait ActivityQuerying: ActivityReadOps {
    /// List all currently active activities from the storage backend.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activities cannot be loaded.
    /// In case of no activities, it should return `Ok(None)`.
    ///
    /// # Returns
    ///
    /// A collection of the activities that are currently active.
    // TODO: should just use `list_activities` with a filter for `active = true`
    // TODO: Implement this as default
    fn list_current_activities(&self) -> PaceResult<Option<ActivityDequeCollection>>;

    /// Find activities within a specific date range.
    ///
    /// # Arguments
    ///
    /// * `start_date` - The start date of the range.
    /// * `end_date` - The end date of the range.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activities cannot be loaded.
    ///
    /// # Returns
    ///
    /// A collection of the activities that fall within the specified date range.
    // TODO: should just use `list_activities` with a filter for `start_date <= date <= end_date`
    // TODO: Implement this as default
    fn find_activities_in_date_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> PaceResult<ActivityDequeCollection>;

    /// Get all activities by their ID.
    ///
    /// # Errors
    ///
    /// This function should return an error if the activities cannot be loaded.
    ///
    /// # Returns
    ///
    /// A collection of the activities that were loaded from the storage backend by their ID in a BTreeMap.
    /// If no activities are found, it should return `Ok(None)`.
    fn list_activities_by_id(&self) -> PaceResult<Option<BTreeMap<ActivityId, Activity>>>;
}

pub trait ActivityTagging {
    /// Add a tag to an activity.
    ///
    /// # Arguments
    ///
    /// * `activity_id` - The ID of the activity to tag.
    /// * `tag` - The tag to add.
    fn add_tag_to_activity(&self, activity_id: ActivityId, tag: &str) -> PaceResult<()>;

    /// Remove a tag from an activity.
    ///
    /// # Arguments
    ///
    /// * `activity_id` - The ID of the activity to untag.
    /// * `tag` - The tag to remove.
    fn remove_tag_from_activity(&self, activity_id: ActivityId, tag: &str) -> PaceResult<()>;
}

pub trait ActivityArchiving {
    /// Archive an activity.
    ///
    /// # Arguments
    ///
    /// * `activity_id` - The ID of the activity to archive.
    fn archive_activity(&self, activity_id: ActivityId) -> PaceResult<()>;

    /// Unarchive an activity.
    ///
    /// # Arguments
    ///
    /// * `activity_id` - The ID of the activity to unarchive.
    fn unarchive_activity(&self, activity_id: ActivityId) -> PaceResult<()>;
}

pub trait ActivityStatistics {
    /// Generate statistics or summary of activities.
    ///
    /// # Arguments
    ///
    /// * `time_frame` - The time frame to generate statistics for (e.g., daily, weekly, monthly).
    ///
    /// # Errors
    ///
    /// This function should return an error if the statistics cannot be generated.
    ///
    /// # Returns
    ///
    /// A summary or statistics of activities within the specified time frame.
    fn generate_activity_statistics(&self, time_frame: TimeFrame) -> PaceResult<ActivityStats>;
}
