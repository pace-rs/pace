//! This module contains the domain logic for tracking activities and their intermissions.

use std::collections::HashMap;

use tracing::debug;

use crate::{
    error::ActivityLogErrorKind, ActivityQuerying, ActivityStore, KeywordOptions, PaceDuration,
    PaceOptResult, PaceTimeFrame, ReviewSummary,
};

// This struct represents the overall structure for tracking activities and their intermissions.
pub struct ActivityTracker {
    store: ActivityStore,
}

impl ActivityTracker {
    /// Create a new activity tracker with the given activity store.
    pub fn with_activity_store(store: ActivityStore) -> Self {
        debug!("Creating activity tracker with activity store");
        Self { store }
    }

    /// Generate a review summary for the specified time frame.
    #[tracing::instrument(skip(self))]
    pub fn generate_review_summary(
        &self,
        time_frame: PaceTimeFrame,
    ) -> PaceOptResult<ReviewSummary> {
        let keyword_opts = KeywordOptions::default();

        let grouped_by_keywords = self
            .store
            .group_activities_by_keywords(keyword_opts)?
            .ok_or(ActivityLogErrorKind::FailedToGroupByKeywords)?;

        debug!("Grouped activities by keywords: {:#?}", grouped_by_keywords);

        for (keyword, activities) in grouped_by_keywords {
            debug!("Keyword: {}", keyword);
            debug!("{} Activities", activities.len());

            let mut grouped_by_description: HashMap<String, PaceDuration> = HashMap::new();

            for activity_item in activities {
                // Skip activities with no duration
                let Ok(duration) = activity_item.activity().duration() else {
                    debug!(
                        "Activity: {} has no duration",
                        activity_item.activity().description()
                    );
                    continue;
                };

                _ = grouped_by_description
                    .entry(activity_item.activity().description().to_string())
                    .and_modify(|e| *e += duration)
                    .or_insert(PaceDuration::zero());

                // Handle intermissions for the activity
                if let Some(intermissions) = self
                    .store
                    .list_intermissions_for_activity_id(*activity_item.guid())?
                {
                    debug!(
                        "Activity: {} has intermissions: {:#?}",
                        activity_item.activity().description(),
                        intermissions.len()
                    );

                    for intermission in intermissions {
                        let Ok(duration) = intermission.activity().duration() else {
                            debug!(
                                "Intermission: {} has no duration",
                                intermission.activity().description()
                            );
                            continue;
                        };

                        _ = grouped_by_description
                            .entry(activity_item.activity().description().to_string())
                            .and_modify(|e| *e -= duration)
                            .or_insert(PaceDuration::zero());
                    }
                };
            }

            println!(
                "{}\nGrouped by description: {:#?}",
                keyword, grouped_by_description
            );
        }

        let summary = ReviewSummary::default();

        debug!("Generated review summary: {:#?}", summary);

        Ok(Some(summary))
    }
}
