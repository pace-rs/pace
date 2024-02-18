//! This module contains the domain logic for tracking activities and their intermissions.

use std::collections::BTreeMap;

use crate::domain::activity::{Activity, ActivityGuid};

// This struct represents the overall structure for tracking activities and their intermissions.
#[derive(Default, Debug, Clone)]
struct ActivityTracker {
    activities: BTreeMap<ActivityGuid, Activity>,
    intermissions: BTreeMap<ActivityGuid, Vec<Activity>>,
}
