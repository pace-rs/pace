//! This module contains the domain logic for tracking activities and their intermissions.

use tracing::debug;

use crate::{ActivityStore, PaceOptResult, PaceTimeFrame, ReviewSummary, TimeRangeOptions};

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

    /// Generate a review summary for the specified time frame.
    #[tracing::instrument(skip(self))]
    pub fn generate_review_summary(
        &self,
        time_frame: PaceTimeFrame,
    ) -> PaceOptResult<ReviewSummary> {
        let time_range_opts = TimeRangeOptions::try_from(time_frame)?;

        let Some(summary_groups) = self
            .store
            .summary_groups_by_category_for_time_range(time_range_opts)?
        else {
            return Ok(None);
        };

        let summary = ReviewSummary::new(time_range_opts, summary_groups);

        debug!("Generated review summary: {:#?}", summary);

        Ok(Some(summary))
    }
}
