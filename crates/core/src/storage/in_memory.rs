use std::sync::{Arc, Mutex};

use crate::{
    domain::activity::{self, Activity, ActivityLog},
    error::PaceResult,
    storage::ActivityStorage,
};

type SharedActivityLog = Arc<Mutex<ActivityLog>>;

struct InMemoryActivityStorage {
    activities: SharedActivityLog,
}

impl InMemoryActivityStorage {
    fn new() -> Self {
        InMemoryActivityStorage {
            activities: Arc::new(Mutex::new(ActivityLog::default())),
        }
    }
}

impl ActivityStorage for InMemoryActivityStorage {
    fn load_activities(&self) -> PaceResult<ActivityLog> {
        // Cloning the vector to simulate loading from a persistent store
        Ok(self
            .activities
            .into_inner()
            .expect("Getting inner from unpoisened Mutex should succeed."))
    }

    fn save_activity(&self, activity: &Activity) -> PaceResult<()> {
        // Simply push the new activity onto the vector
        let mut guard = self
            .activities
            .lock()
            .expect("Mutex should not be poisened.");
        guard.add(activity.clone());
        Ok(())
    }
}
