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
