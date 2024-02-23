use std::collections::{BTreeMap, HashSet, VecDeque};

use chrono::{prelude::NaiveDate, NaiveDateTime};
use serde_derive::{Deserialize, Serialize};

use crate::{
    domain::{
        activity::{Activity, ActivityGuid},
        activity_log::ActivityLog,
        filter::FilteredActivities,
    },
    error::{PaceOptResult, PaceResult},
    storage::{
        ActivityQuerying, ActivityReadOps, ActivityStateManagement, ActivityStorage,
        ActivityWriteOps, SyncStorage,
    },
};

/// The activity store entity
pub struct ActivityStore {
    /// In-memory cache for activities
    cache: ActivityStoreCache,

    /// The storage backend
    storage: Box<dyn ActivityStorage>,
}

/// TODO: Optimization for later to make lookup faster
#[derive(Serialize, Deserialize, Debug, Default)]
struct ActivityStoreCache {
    activity_ids: HashSet<ActivityGuid>,
    activities_by_id: BTreeMap<ActivityGuid, Activity>,
    last_entries: VecDeque<ActivityGuid>,
}

impl ActivityStore {
    /// Create a new `ActivityStore`
    #[must_use]
    pub fn new(storage: Box<dyn ActivityStorage>) -> Self {
        let store = Self {
            cache: ActivityStoreCache::default(),
            storage,
        };

        store
            .setup_storage()
            .expect("Should not fail to setup storage.");

        store
    }
}

impl ActivityStorage for ActivityStore {
    fn setup_storage(&self) -> PaceResult<()> {
        self.storage.setup_storage()
    }
}

impl SyncStorage for ActivityStore {
    fn sync(&self) -> PaceResult<()> {
        self.storage.sync()
    }
}

impl ActivityReadOps for ActivityStore {
    fn read_activity(&self, activity_id: ActivityGuid) -> PaceResult<Activity> {
        self.storage.read_activity(activity_id)
    }

    fn list_activities(
        &self,
        filter: crate::domain::filter::ActivityFilter,
    ) -> PaceOptResult<FilteredActivities> {
        self.storage.list_activities(filter)
    }
}

impl ActivityWriteOps for ActivityStore {
    fn create_activity(&self, activity: Activity) -> PaceResult<ActivityGuid> {
        self.storage.create_activity(activity)
    }

    fn update_activity(
        &self,
        activity_id: ActivityGuid,
        activity: Activity,
    ) -> PaceResult<Activity> {
        self.storage.update_activity(activity_id, activity)
    }

    fn delete_activity(&self, activity_id: ActivityGuid) -> PaceResult<Activity> {
        self.storage.delete_activity(activity_id)
    }
}

impl ActivityStateManagement for ActivityStore {
    fn begin_activity(&self, activity: Activity) -> PaceResult<ActivityGuid> {
        self.storage.begin_activity(activity)
    }

    fn end_single_activity(
        &self,
        activity_id: ActivityGuid,
        end_time: Option<NaiveDateTime>,
    ) -> PaceResult<ActivityGuid> {
        self.storage.end_single_activity(activity_id, end_time)
    }

    fn end_all_unfinished_activities(
        &self,
        time: Option<NaiveDateTime>,
    ) -> PaceOptResult<Vec<Activity>> {
        self.storage.end_all_unfinished_activities(time)
    }

    fn end_last_unfinished_activity(&self, time: Option<NaiveDateTime>) -> PaceOptResult<Activity> {
        self.storage.end_last_unfinished_activity(time)
    }

    fn hold_last_unfinished_activity(
        &self,
        hold_time: Option<NaiveDateTime>,
    ) -> PaceOptResult<Activity> {
        self.storage.hold_last_unfinished_activity(hold_time)
    }

    fn end_all_active_intermissions(
        &self,
        end_time: Option<NaiveDateTime>,
    ) -> PaceOptResult<Vec<Activity>> {
        self.storage.end_all_active_intermissions(end_time)
    }
}

impl ActivityQuerying for ActivityStore {
    fn find_activities_in_date_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> PaceResult<ActivityLog> {
        self.storage
            .find_activities_in_date_range(start_date, end_date)
    }

    fn list_activities_by_id(&self) -> PaceOptResult<BTreeMap<ActivityGuid, Activity>> {
        self.storage.list_activities_by_id()
    }
}
