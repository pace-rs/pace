//! Test the `ActivityStore` implementation with a `InMemoryStorage` backend.

use pace_core::prelude::{
    ActivityStore, ActivityTracker, FilterOptions, PaceDuration, TestResult, TimeRangeOptions,
};
use rstest::rstest;
use similar_asserts::assert_eq;

use pace_testing::setup_activity_store_for_activity_tracker;

#[rstest]
fn test_activity_tracker(
    setup_activity_store_for_activity_tracker: TestResult<ActivityStore>,
) -> TestResult<()> {
    let activity_tracker =
        ActivityTracker::with_activity_store(setup_activity_store_for_activity_tracker?);

    assert_eq!(
        activity_tracker.store.cache().by_start_date().len(),
        2,
        "Should have 2 start dates."
    );

    let time_range_opts = TimeRangeOptions::specific_date("2024-02-26".parse()?)?;

    let summary_groups_by_category = activity_tracker
        .store
        .summary_groups_by_category_for_time_range(FilterOptions::default(), time_range_opts)?
        .ok_or("Should have dates.")?;

    assert_eq!(
        summary_groups_by_category.len(),
        1,
        "Should have 1 activity group."
    );

    let group = summary_groups_by_category
        .get(&("development".to_string(), "pace".to_string()))
        .ok_or("Should have a category.")?;

    assert_eq!(group.len(), 1, "Should have 1 activity.");

    // assert on duration for activity group
    let mut activity = group.activity_groups_by_description().values();

    assert_eq!(
        activity.len(),
        1,
        "Should have 1 activity group by description."
    );

    let activity = activity.next().ok_or("Should have an activity.")?;

    assert_eq!(
        activity.adjusted_duration(),
        &PaceDuration::from_seconds(202),
        "Should have a duration of 202."
    );

    // assert on duration for summary group
    assert_eq!(
        group.total_duration(),
        &PaceDuration::from_seconds(202),
        "Should have a total duration of 202."
    );

    dbg!(&summary_groups_by_category);

    Ok(())
}
