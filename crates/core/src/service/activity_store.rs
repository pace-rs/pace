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

impl ActivityStore {
    pub fn new(storage: Box<dyn ActivityStorage>) -> Self {
        ActivityStore {
            cache: ActivityStoreCache::default(),
            storage,
        }
    }
}
