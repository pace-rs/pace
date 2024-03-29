//! This module contains the domain logic for tracking activities and their intermissions.

use pace_time::{time_frame::PaceTimeFrame, time_range::TimeRangeOptions};
use tracing::debug;

use pace_core::{domain::reflection::ReflectionSummary, options::FilterOptions};
use pace_error::PaceOptResult;

use crate::activity_store::ActivityStore;

// This struct represents the overall structure for tracking activities and their intermissions.
pub struct ActivityTracker {
    pub store: ActivityStore,
}

impl ActivityTracker {
    /// Create a new activity tracker with the given activity store.
    pub fn with_activity_store(store: ActivityStore) -> Self {
        debug!("Creating activity tracker with activity store");
        Self { store }
    }

    // TODO!
    // - [ ] implement the `detailed` flag
    // - [ ] implement the `comparative` flag
    // - [ ] implement the `recommendations` flag

    /// Generate a reflection for the specified time frame.
    #[tracing::instrument(skip(self))]
    pub fn generate_reflection(
        &self,
        filter_opts: FilterOptions,
        time_frame: PaceTimeFrame,
    ) -> PaceOptResult<ReflectionSummary> {
        let time_range_opts = TimeRangeOptions::try_from(time_frame)?;

        let Some(summary_groups) = self
            .store
            .summary_groups_by_category_for_time_range(filter_opts, time_range_opts)?
        else {
            return Ok(None);
        };

        let summary = ReflectionSummary::new(time_range_opts, summary_groups);

        debug!("Generated reflection: {:#?}", summary);

        Ok(Some(summary))
    }
}
