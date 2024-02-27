//! This module contains the domain logic for tracking activities and their intermissions.

use crate::ActivityStore;

// This struct represents the overall structure for tracking activities and their intermissions.
pub struct ActivityTracker {
    store: ActivityStore,
    // activities: BTreeMap<ActivityGuid, Activity>,
    // intermissions: BTreeMap<ActivityGuid, Vec<Activity>>,
}

impl ActivityTracker {
    /// Create a new activity tracker with the given activity store.
    pub fn with_activity_store(store: ActivityStore) -> Self {
        Self { store }
    }
}
