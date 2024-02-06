use std::collections::{BTreeMap, VecDeque};

use chrono::NaiveTime;
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    domain::activity::{Activity, ActivityId, ActivityLog},
    error::PaceResult,
    storage::ActivityStorage,
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

impl ActivityStorage for ActivityStore {
    fn setup_storage(&self) -> PaceResult<()> {
        self.storage.setup_storage()
    }

    fn load_all_activities(&self) -> PaceResult<ActivityLog> {
        self.storage.load_all_activities()
    }

    fn list_current_activities(&self) -> PaceResult<Option<Vec<Activity>>> {
        self.storage.list_current_activities()
    }

    fn save_activity(&self, activity: &Activity) -> PaceResult<()> {
        self.storage.save_activity(activity)
    }

    fn end_all_unfinished_activities(
        &self,
        time: Option<NaiveTime>,
    ) -> PaceResult<Option<Vec<Activity>>> {
        self.storage.end_all_unfinished_activities(time)
    }

    fn end_last_unfinished_activity(
        &self,
        time: Option<NaiveTime>,
    ) -> PaceResult<Option<Activity>> {
        self.storage.end_last_unfinished_activity(time)
    }

    fn get_activities_by_id(
        &self,
        uuid: Uuid,
    ) -> PaceResult<Option<BTreeMap<ActivityId, Activity>>> {
        self.storage.get_activities_by_id(uuid)
    }
}

impl ActivityStore {
    pub fn new(storage: Box<dyn ActivityStorage>) -> Self {
        ActivityStore {
            cache: ActivityStoreCache::default(),
            storage,
        }
    }
    // pub fn get_activities_by_id(&self, id: &ActivityId) -> PaceResult<Activity> {
    //     self.load_activities().and_then(|activity_log|)
    //     self.activities_by_id.get(&id)
    // }

    // pub fn init(&mut self) -> PaceResult<()> {
    //             self.load_activities().and_then(|activity_log|)
    //             Ok(())
    // }
}
