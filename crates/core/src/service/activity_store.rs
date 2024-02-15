use std::collections::{BTreeMap, VecDeque};

use chrono::{NaiveDateTime, NaiveTime};
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    domain::{
        activity::{Activity, ActivityId, ActivityLog},
        filter::FilteredActivities,
    },
    error::PaceResult,
    storage::{ActivityReadOps, ActivityStateManagement, ActivityStorage, ActivityWriteOps},
};

pub struct ActivityStore {
    cache: ActivityStoreCache,
    storage: Box<dyn ActivityStorage>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ActivityStoreCache {
    activities_by_id: BTreeMap<ActivityId, Activity>,
    last_entries: VecDeque<ActivityId>,
}

impl ActivityStore {
    pub fn new(storage: Box<dyn ActivityStorage>) -> Self {
        ActivityStore {
            cache: ActivityStoreCache::default(),
            storage,
        }
    }
}

impl ActivityStorage for ActivityStore {
    fn setup_storage(&self) -> PaceResult<()> {
        self.storage.setup_storage()
    }
}

impl ActivityReadOps for ActivityStore {
    fn read_activity(&self, activity_id: ActivityId) -> PaceResult<Option<Activity>> {
        self.storage.read_activity(activity_id)
    }

    fn list_activities(
        &self,
        filter: crate::domain::filter::ActivityFilter,
    ) -> PaceResult<Option<FilteredActivities>> {
        self.storage.list_activities(filter)
    }
}

impl ActivityWriteOps for ActivityStore {
    fn create_activity(&self, activity: &Activity) -> PaceResult<ActivityId> {
        self.storage.create_activity(activity)
    }

    fn update_activity(&self, activity_id: ActivityId, activity: &Activity) -> PaceResult<()> {
        self.storage.update_activity(activity_id, activity)
    }

    fn delete_activity(&self, activity_id: ActivityId) -> PaceResult<()> {
        self.storage.delete_activity(activity_id)
    }
}

impl ActivityStateManagement for ActivityStore {
    fn start_activity(&self, activity: &Activity) -> PaceResult<ActivityId> {
        self.storage.start_activity(activity)
    }

    fn end_activity(
        &self,
        activity_id: ActivityId,
        end_time: Option<NaiveDateTime>,
    ) -> PaceResult<ActivityId> {
        self.storage.end_activity(activity_id, end_time)
    }

    fn end_all_unfinished_activities(
        &self,
        time: Option<NaiveDateTime>,
    ) -> PaceResult<Option<Vec<Activity>>> {
        self.storage.end_all_unfinished_activities(time)
    }

    fn end_last_unfinished_activity(
        &self,
        time: Option<NaiveDateTime>,
    ) -> PaceResult<Option<Activity>> {
        self.storage.end_last_unfinished_activity(time)
    }
}
