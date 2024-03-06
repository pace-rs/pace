//! Test the `ActivityStore` implementation with a `InMemoryStorage` backend.

use pace_core::{ActivityStore, ActivityTracker, TestResult};
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

    // let time_range_opts = TimeRangeOptions::from(
    //     PaceDate::try_from((2024, 2, 26))?,
    //     PaceDate::try_from((2024, 2, 26))?,
    // );

    // let dates = activity_tracker
    //     .store
    //     .activity_log_for_date_range(time_range_opts)
    //     .keys()
    //     .sorted()
    //     .cloned()
    //     .collect::<Vec<_>>();

    // assert_eq!(
    //     dates,
    //     vec![
    //         PaceDate::try_from((2024, 2, 26))?,
    //         PaceDate::try_from((2024, 2, 27))?
    //     ],
    //     "Should have the start dates in the correct order."
    // );

    // let activities_for_date = activity_tracker
    //     .store
    //     .cache()
    //     .by_start_date()
    //     .get(&PaceDate::try_from((2024, 2, 26))?)
    //     .ok_or("Should have activities for the date.")?
    //     .clone();

    // let activity_log = ActivityLog::from_iter(activities_for_date);

    // dbg!(&activity_log);

    Ok(())
}
