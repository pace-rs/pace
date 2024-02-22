// Test the ActivityStore implementation with a InMemoryStorage backend.

use chrono::{Local, NaiveDateTime};
use pace_core::{
    Activity, ActivityFilter, ActivityGuid, ActivityLog, ActivityReadOps, ActivityStore,
    ActivityWriteOps, BeginDateTime, InMemoryActivityStorage, PaceResult, TestResult,
};
use rstest::{fixture, rstest};
use similar_asserts::assert_eq;

#[fixture]
fn activity_log_empty() -> ActivityLog {
    let activities = vec![];

    ActivityLog::from_iter(activities)
}

#[fixture]
fn activity_log_with_variety_content() -> (Vec<Activity>, ActivityLog) {
    let begin_time = BeginDateTime::new(NaiveDateTime::new(
        NaiveDateTime::from_timestamp_opt(0, 0).unwrap().date(),
        NaiveDateTime::from_timestamp_opt(0, 0).unwrap().time(),
    ));

    let mut ended_activity = Activity::builder()
        .description("Test Description".to_string())
        .begin(begin_time)
        .build();
    ended_activity
        .end_activity_with_duration_calc(begin_time, Local::now().naive_local())
        .expect("Creating ended activity should not fail.");

    let activities = vec![
        Activity::default(),
        Activity::default(),
        ended_activity,
        Activity::default(),
        Activity::default(),
        Activity::default(),
    ];

    (activities.clone(), ActivityLog::from_iter(activities))
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
) -> TestResult<(ActivityGuid, Activity, ActivityStore)> {
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
    let og_activity_id = activity.guid().expect("Activity ID should be set.");

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

    let id = activities[0].guid().expect("Activity ID should be set.");

    let activity = Activity::builder()
        .guid(id)
        .description("Test Description".to_string())
        .category(Some("Test::Category".to_string()))
        .build();

    assert!(store.create_activity(activity).is_err());
}

#[rstest]
fn test_activity_store_read_activity_passes(
    activity_store_with_item: TestResult<(ActivityGuid, Activity, ActivityStore)>,
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

    let activity_id = ActivityGuid::default();

    assert!(store.read_activity(activity_id).is_err());
}

// TODO!: Test the list_activities method with all the other filters.
// List activities can hardly fail, as it returns an empty list if no activities are found.
// Therefore, we only test the success case. It would fail if the mutex is poisoned.
#[rstest]
fn test_activity_store_list_active_activities_passes(
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
fn test_activity_store_list_ended_activities_passes(
    activity_log_with_variety_content: (Vec<Activity>, ActivityLog),
) -> TestResult<()> {
    let (_activities, activity_log) = activity_log_with_variety_content;
    let store = ActivityStore::new(Box::new(InMemoryActivityStorage::new_with_activity_log(
        activity_log,
    )));

    let loaded_activities = store
        .list_activities(ActivityFilter::Ended)?
        .expect("Should have activities.");

    assert_eq!(1, loaded_activities.into_log().activities().len());

    Ok(())
}

#[rstest]
fn test_activity_store_list_all_activities_passes(
    activity_log_with_variety_content: (Vec<Activity>, ActivityLog),
) -> TestResult<()> {
    let (activities, activity_log) = activity_log_with_variety_content;
    let store = ActivityStore::new(Box::new(InMemoryActivityStorage::new_with_activity_log(
        activity_log,
    )));

    let loaded_activities = store
        .list_activities(ActivityFilter::All)?
        .expect("Should have activities.");

    assert_eq!(
        activities.len(),
        loaded_activities.into_log().activities().len()
    );

    Ok(())
}

#[rstest]
fn test_activity_store_list_all_activities_empty_result_passes(
    activity_log_empty: ActivityLog,
) -> TestResult<()> {
    let activity_log = activity_log_empty;
    let store = ActivityStore::new(Box::new(InMemoryActivityStorage::new_with_activity_log(
        activity_log,
    )));

    assert!(store.list_activities(ActivityFilter::All)?.is_none());

    Ok(())
}

#[rstest]
fn test_activity_store_update_activity_passes(
    activity_store_with_item: TestResult<(ActivityGuid, Activity, ActivityStore)>,
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

    _ = new_activity.guid_mut().replace(og_activity_id);

    assert_eq!(stored_activity, new_activity);

    Ok(())
}

#[rstest]
fn test_activity_store_delete_activity_passes(
    activity_store_with_item: TestResult<(ActivityGuid, Activity, ActivityStore)>,
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

    let activity_id = ActivityGuid::default();

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

    let activity_id = ActivityGuid::default();

    assert!(store.update_activity(activity_id, new_activity).is_err());
}

#[rstest]
fn test_activity_store_begin_intermission_passes() -> PaceResult<()> {
    let toml_string = r#"
[[activities]]
id = "01HQ8B27751H7QPBD2V7CZD1B7"
description = "Intermission Test"
begin = "2024-02-22T13:01:25"
kind = "intermission"
parent-id = "01HQ8B1WS5X0GZ660738FNED91"

[[activities]]
id = "01HQ8B1WS5X0GZ660738FNED91"
category = "MyCategory::SubCategory"
description = "Intermission Test"
begin = "2024-02-22T13:01:14"
kind = "activity"
"#;

    let activity_log = toml::from_str::<ActivityLog>(toml_string)?;

    let _store = ActivityStore::new(Box::new(InMemoryActivityStorage::new_with_activity_log(
        activity_log,
    )));

    // TODO!: Implement intermission handling.

    Ok(())
}
