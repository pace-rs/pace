use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    sync::Arc,
};

use chrono::{prelude::NaiveDate, NaiveDateTime};
use serde_derive::{Deserialize, Serialize};

use crate::{
    domain::{
        activity::{Activity, ActivityGuid, ActivityItem},
        activity_log::ActivityLog,
        filter::FilteredActivities,
    },
    error::{PaceOptResult, PaceResult},
    storage::{
        ActivityQuerying, ActivityReadOps, ActivityStateManagement, ActivityStorage,
        ActivityWriteOps, SyncStorage,
    },
    EndOptions, HoldOptions,
};

/// The activity store entity
pub struct ActivityStore {
    /// In-memory cache for activities
    cache: ActivityStoreCache,

    /// The storage backend
    storage: Arc<dyn ActivityStorage>,
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
    pub fn new(storage: Arc<dyn ActivityStorage>) -> Self {
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
    fn read_activity(&self, activity_id: ActivityGuid) -> PaceResult<ActivityItem> {
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
    fn create_activity(&self, activity: Activity) -> PaceResult<ActivityItem> {
        self.storage.create_activity(activity)
    }

    fn update_activity(
        &self,
        activity_id: ActivityGuid,
        activity: Activity,
    ) -> PaceResult<ActivityItem> {
        self.storage.update_activity(activity_id, activity)
    }

    fn delete_activity(&self, activity_id: ActivityGuid) -> PaceResult<ActivityItem> {
        self.storage.delete_activity(activity_id)
    }
}

impl ActivityStateManagement for ActivityStore {
    fn begin_activity(&self, activity: Activity) -> PaceResult<ActivityItem> {
        self.storage.begin_activity(activity)
    }

    fn end_activity(
        &self,
        activity_id: ActivityGuid,
        end_opts: EndOptions,
    ) -> PaceResult<ActivityItem> {
        self.storage.end_activity(activity_id, end_opts)
    }

    fn end_all_unfinished_activities(
        &self,
        end_opts: EndOptions,
    ) -> PaceOptResult<Vec<ActivityItem>> {
        self.storage.end_all_unfinished_activities(end_opts)
    }

    fn end_last_unfinished_activity(&self, end_opts: EndOptions) -> PaceOptResult<ActivityItem> {
        self.storage.end_last_unfinished_activity(end_opts)
    }

    fn hold_most_recent_active_activity(
        &self,
        hold_opts: HoldOptions,
    ) -> PaceOptResult<ActivityItem> {
        self.storage.hold_most_recent_active_activity(hold_opts)
    }

    fn end_all_active_intermissions(
        &self,
        end_opts: EndOptions,
    ) -> PaceOptResult<Vec<ActivityGuid>> {
        self.storage.end_all_active_intermissions(end_opts)
    }

    fn resume_activity(
        &self,
        activity_id: Option<ActivityGuid>,
        resume_time: Option<NaiveDateTime>,
    ) -> PaceOptResult<ActivityItem> {
        self.storage.resume_activity(activity_id, resume_time)
    }

    fn hold_activity(
        &self,
        activity_id: ActivityGuid,
        hold_opts: HoldOptions,
    ) -> PaceResult<ActivityItem> {
        self.storage.hold_activity(activity_id, hold_opts)
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
