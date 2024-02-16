// Test the ActivityStore implementation with a InMemoryStorage backend.

use pace_core::{domain::activity_log::ActivityLog, service::activity_store::ActivityStore};
use pace_core::{
    domain::{
        activity::{Activity, ActivityId},
        filter::ActivityFilter,
    },
    error::TestResult,
    storage::{in_memory::InMemoryActivityStorage, ActivityReadOps, ActivityWriteOps},
};

use rstest::{fixture, rstest};

#[fixture]
fn activity_log_empty() -> ActivityLog {
    let activities = vec![];

    ActivityLog::from_iter(activities)
}

#[fixture]
fn activity_log_with_content() -> (Vec<Activity>, ActivityLog) {
    let activities = vec![
        Activity::default(),
        Activity::default(),
        Activity::default(),
        Activity::default(),
        Activity::default(),
        Activity::default(),
    ];

    (activities.clone(), ActivityLog::from_iter(activities))
}

#[fixture]
fn activity_store_with_item(
    activity_log_empty: ActivityLog,
) -> TestResult<(ActivityId, Activity, ActivityStore)> {
    let store = ActivityStore::new(Box::new(InMemoryActivityStorage::new_with_activity_log(
        activity_log_empty,
    )));

    let activity = Activity::builder()
        .description("Test Description".to_string())
        .category(Some("Test::Category".to_string()))
        .build();

    let activity_id = store.create_activity(activity.clone())?;

    Ok((activity_id, activity, store))
}

#[rstest]
fn test_activity_store_create_activity_passes(activity_log_empty: ActivityLog) -> TestResult<()> {
    let store = ActivityStore::new(Box::new(InMemoryActivityStorage::new_with_activity_log(
        activity_log_empty,
    )));

    let activity = Activity::builder()
        .description("Test Description".to_string())
        .category(Some("Test::Category".to_string()))
        .build();

    let og_activity = activity.clone();
    let og_activity_id = activity.id().expect("Activity ID should be set.");

    let activity_id = store.create_activity(activity)?;

    assert_eq!(activity_id, og_activity_id);

    let stored_activity = store.read_activity(og_activity_id)?;

    assert_eq!(stored_activity, og_activity);

    Ok(())
}

#[rstest]
fn test_activity_store_create_activity_fails(
    activity_log_with_content: (Vec<Activity>, ActivityLog),
) {
    let (activities, activity_log) = activity_log_with_content;
    let store = ActivityStore::new(Box::new(InMemoryActivityStorage::new_with_activity_log(
        activity_log,
    )));

    let id = activities[0].id().expect("Activity ID should be set.");

    let activity = Activity::builder()
        .id(id)
        .description("Test Description".to_string())
        .category(Some("Test::Category".to_string()))
        .build();

    assert!(store.create_activity(activity).is_err());
}

#[rstest]
fn test_activity_store_read_activity_passes(
    activity_store_with_item: TestResult<(ActivityId, Activity, ActivityStore)>,
) -> TestResult<()> {
    let (og_activity_id, og_activity, store) = activity_store_with_item?;

    let stored_activity = store.read_activity(og_activity_id)?;

    assert_eq!(stored_activity, og_activity);

    Ok(())
}

#[rstest]
fn test_activity_store_read_activity_fails(activity_log_empty: ActivityLog) {
    let store = ActivityStore::new(Box::new(InMemoryActivityStorage::new_with_activity_log(
        activity_log_empty,
    )));

    let activity_id = ActivityId::default();

    assert!(store.read_activity(activity_id).is_err());
}

// List activities can hardly fail, as it returns an empty list if no activities are found.
// Therefore, we only test the success case. It would fail if the mutex is poisoned.
#[rstest]
fn test_activity_store_list_activities_passes(
    activity_log_with_content: (Vec<Activity>, ActivityLog),
) -> TestResult<()> {
    let (activities, activity_log) = activity_log_with_content;
    let store = ActivityStore::new(Box::new(InMemoryActivityStorage::new_with_activity_log(
        activity_log,
    )));

    let loaded_activities = store
        .list_activities(ActivityFilter::Active)?
        .expect("Should have activities.");

    assert_eq!(
        activities.len(),
        loaded_activities.into_log().activities().len()
    );

    Ok(())
}

#[rstest]
fn test_activity_store_update_activity_passes(
    activity_store_with_item: TestResult<(ActivityId, Activity, ActivityStore)>,
) -> TestResult<()> {
    let (og_activity_id, og_activity, store) = activity_store_with_item?;

    let updated_test_desc = "Updated Test Description".to_string();
    let updated_test_cat = "Test::UpdatedCategory".to_string();

    let mut new_activity = Activity::builder()
        .description(updated_test_desc.to_string())
        .category(Some(updated_test_cat))
        .build();

    let old_activity = store.update_activity(og_activity_id, new_activity.clone())?;

    assert_eq!(old_activity, og_activity);

    let stored_activity = store.read_activity(og_activity_id)?;

    _ = new_activity.id_mut().replace(og_activity_id);

    assert_eq!(stored_activity, new_activity);

    Ok(())
}

#[rstest]
fn test_activity_store_delete_activity_passes(
    activity_store_with_item: TestResult<(ActivityId, Activity, ActivityStore)>,
) -> TestResult<()> {
    let (og_activity_id, og_activity, store) = activity_store_with_item?;

    let activity = store.delete_activity(og_activity_id)?;

    assert!(store.read_activity(og_activity_id).is_err());

    assert_eq!(activity, og_activity);

    Ok(())
}

#[rstest]
fn test_activity_store_delete_activity_fails(
    activity_log_with_content: (Vec<Activity>, ActivityLog),
) {
    let (_, activity_log) = activity_log_with_content;
    let store = ActivityStore::new(Box::new(InMemoryActivityStorage::new_with_activity_log(
        activity_log,
    )));

    let activity_id = ActivityId::default();

    assert!(store.delete_activity(activity_id).is_err());
}

#[rstest]
fn test_activity_store_update_activity_fails(
    activity_log_with_content: (Vec<Activity>, ActivityLog),
) {
    let (_, activity_log) = activity_log_with_content;
    let store = ActivityStore::new(Box::new(InMemoryActivityStorage::new_with_activity_log(
        activity_log,
    )));

    let new_activity = Activity::builder()
        .description("test".to_string())
        .category(Some("test".to_string()))
        .build();

    let activity_id = ActivityId::default();

    assert!(store.update_activity(activity_id, new_activity).is_err());
}
