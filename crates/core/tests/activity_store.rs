// Test the ActivityStore implementation with a InMemoryStorage backend.

use std::sync::Arc;

use chrono::{Local, NaiveDateTime};

use pace_core::{
    Activity, ActivityFilter, ActivityGuid, ActivityItem, ActivityKind, ActivityKindOptions,
    ActivityLog, ActivityReadOps, ActivityStateManagement, ActivityStore, ActivityWriteOps,
    EndOptions, HoldOptions, InMemoryActivityStorage, PaceDateTime, PaceResult, TestResult,
};
use rstest::{fixture, rstest};
use similar_asserts::assert_eq;

struct TestData {
    activities: Vec<ActivityItem>,
    store: ActivityStore,
}

enum ActivityStoreTestKind {
    Empty,
    WithActivitiesAndOpenIntermission,
    WithoutIntermissions,
}

#[fixture]
fn activity_store_empty() -> TestData {
    setup_activity_store(ActivityStoreTestKind::Empty)
}

#[fixture]
fn activity_store() -> TestData {
    setup_activity_store(ActivityStoreTestKind::WithActivitiesAndOpenIntermission)
}

#[fixture]
fn activity_store_no_intermissions() -> TestData {
    setup_activity_store(ActivityStoreTestKind::WithoutIntermissions)
}

fn setup_activity_store(kind: ActivityStoreTestKind) -> TestData {
    let begin_time = PaceDateTime::new(NaiveDateTime::new(
        NaiveDateTime::from_timestamp_opt(0, 0).unwrap().date(),
        NaiveDateTime::from_timestamp_opt(0, 0).unwrap().time(),
    ));

    let mut ended_activity = Activity::builder()
        .description("Activity with end".to_string())
        .begin(begin_time)
        .active(false)
        .build();
    ended_activity
        .end_activity_with_duration_calc(begin_time, PaceDateTime::now())
        .expect("Creating ended activity should not fail.");

    let ended_activity = ActivityItem::from((ActivityGuid::default(), ended_activity));

    let mut archived_activity = Activity::builder()
        .description("Activity with end".to_string())
        .begin(begin_time)
        .active(false)
        .build();
    archived_activity
        .end_activity_with_duration_calc(begin_time, PaceDateTime::now())
        .expect("Creating ended activity should not fail.");
    archived_activity.archive();

    let archived_activity = ActivityItem::from((ActivityGuid::default(), archived_activity));

    let time_30_min_ago = Local::now().naive_local() - chrono::Duration::minutes(30);
    let begin_time = PaceDateTime::new(time_30_min_ago);
    let intermission_begin_time =
        PaceDateTime::new(time_30_min_ago + chrono::Duration::minutes(15));
    let desc = "Activity with Intermission".to_string();
    let cat = "Test::Intermission".to_string();

    let active = ActivityItem::from((
        ActivityGuid::default(),
        Activity::builder()
            .begin(begin_time)
            .description(desc.clone())
            .category(cat.clone())
            .build(),
    ));

    let guid = active.guid();

    let intermission = ActivityItem::from((
        ActivityGuid::default(),
        Activity::builder()
            .begin(intermission_begin_time)
            .kind(ActivityKind::Intermission)
            .description(desc)
            .category(cat)
            .activity_kind_options(ActivityKindOptions::with_parent_id(*guid))
            .build(),
    ));

    let default_no_end = ActivityItem::from((
        ActivityGuid::default(),
        Activity::builder()
            .description("Default activity, but no end and not active.")
            .active(false)
            .build(),
    ));

    let mut activities = vec![];

    match kind {
        ActivityStoreTestKind::Empty => (),
        ActivityStoreTestKind::WithActivitiesAndOpenIntermission => {
            activities.push(default_no_end);
            activities.push(archived_activity);
            activities.push(ended_activity);
            activities.push(active);
            activities.push(intermission);
        }
        ActivityStoreTestKind::WithoutIntermissions => {
            activities.push(default_no_end);
            activities.push(archived_activity);
            activities.push(ended_activity);
            activities.push(active);
        }
    }

    TestData {
        activities: activities.clone(),
        store: ActivityStore::new(Arc::new(InMemoryActivityStorage::new_with_activity_log(
            ActivityLog::from_iter(activities),
        ))),
    }
}

#[rstest]
fn test_activity_store_create_activity_passes(activity_store_empty: TestData) -> TestResult<()> {
    let TestData {
        activities: _,
        store,
    } = activity_store_empty;

    let activity = Activity::builder()
        .description("Test Description".to_string())
        .category("Test::Category".to_string())
        .build();

    let og_activity = activity.clone();

    let og_activity_item = store.create_activity(activity)?;

    let stored_activity = store.read_activity(*og_activity_item.guid())?;

    assert_eq!(*stored_activity.activity(), og_activity);

    Ok(())
}

// Creating the same activity twice should fail, as the GUID is the same.
// But this is not possible to test, as the GUID is generated by the store.
//
// #[rstest]
// fn test_activity_store_create_activity_fails(activity_store: TestData) {
//     let TestData { activities, store } = activity_store;

//     let id = *activities[0].guid();

//     let activity = Activity::builder()
//         .description("Test Description".to_string())
//         .category("Test::Category".to_string())
//         .build();

//     assert!(store.create_activity(activity).is_err());
// }

#[rstest]
fn test_activity_store_read_activity_passes(activity_store: TestData) -> TestResult<()> {
    let TestData { activities, store } = activity_store;

    let og_activity = activities[0].clone();
    let og_activity_id = *og_activity.guid();

    let stored_activity = store.read_activity(og_activity_id)?;

    assert_eq!(stored_activity, og_activity);

    Ok(())
}

#[rstest]
fn test_activity_store_read_activity_fails(activity_store: TestData) {
    let TestData {
        activities: _,
        store,
    } = activity_store;

    let activity_id = ActivityGuid::default();

    assert!(store.read_activity(activity_id).is_err());
}

#[rstest]
fn test_activity_store_list_activities_returns_none_on_empty_passes(
    activity_store_empty: TestData,
) -> TestResult<()> {
    let TestData {
        activities: _,
        store,
    } = activity_store_empty;

    assert!(store.list_activities(ActivityFilter::Everything)?.is_none());

    Ok(())
}

// List activities can hardly fail, as it returns an empty list if no activities are found.
// Therefore, we only test the success case. It would fail if the mutex is poisoned.
#[rstest]
fn test_activity_store_list_activities_passes(activity_store: TestData) -> TestResult<()> {
    use strum::IntoEnumIterator;

    let TestData { activities, store } = activity_store;

    for filter in ActivityFilter::iter() {
        match filter {
            ActivityFilter::OnlyActivities => {
                let loaded_activities = store
                    .list_activities(ActivityFilter::OnlyActivities)?
                    .expect("Should have only activities.");

                assert_eq!(
                    4,
                    loaded_activities.into_vec().len(),
                    "Should have only 4 activities."
                );
            }
            ActivityFilter::Archived => {
                let loaded_activities = store
                    .list_activities(ActivityFilter::Archived)?
                    .expect("Should have archived activities.");

                assert_eq!(1, loaded_activities.into_vec().len());
            }
            ActivityFilter::ActiveIntermission => {
                let loaded_activities = store
                    .list_activities(ActivityFilter::ActiveIntermission)?
                    .expect("Should have activities.");

                assert_eq!(
                    1,
                    loaded_activities.into_vec().len(),
                    "Should have one active intermission."
                );
            }
            ActivityFilter::Active => {
                let loaded_activities = store
                    .list_activities(ActivityFilter::Active)?
                    .expect("Should have active activities.");

                assert_eq!(
                    2,
                    loaded_activities.into_vec().len(),
                    "Should have two active activities."
                );
            }
            ActivityFilter::Ended => {
                let loaded_activities = store
                    .list_activities(ActivityFilter::Ended)?
                    .expect("Should have ended activities.");

                assert_eq!(
                    1,
                    loaded_activities.into_vec().len(),
                    "Should have one ended activity."
                );
            }
            ActivityFilter::Everything => {
                let loaded_activities = store
                    .list_activities(ActivityFilter::Everything)?
                    .expect("Should have everything (activities, intermissions, etc.).");

                assert_eq!(
                    activities.len(),
                    loaded_activities.into_vec().len(),
                    "Should be the same length as initial activities."
                );
            }
        }
    }

    Ok(())
}

#[rstest]
fn test_activity_store_list_ended_activities_passes(activity_store: TestData) -> TestResult<()> {
    let TestData {
        activities: _,
        store,
    } = activity_store;

    let loaded_activities = store
        .list_activities(ActivityFilter::Ended)?
        .expect("Should have activities.");

    assert_eq!(1, loaded_activities.into_vec().len());

    Ok(())
}

#[rstest]
fn test_activity_store_list_all_activities_passes(activity_store: TestData) -> TestResult<()> {
    let TestData { activities, store } = activity_store;

    let loaded_activities = store
        .list_activities(ActivityFilter::Everything)?
        .expect("Should have activities.");

    assert_eq!(activities.len(), loaded_activities.into_vec().len());

    Ok(())
}

#[rstest]
fn test_activity_store_list_all_activities_empty_result_passes(
    activity_store_empty: TestData,
) -> TestResult<()> {
    let TestData {
        activities: _,
        store,
    } = activity_store_empty;

    assert!(store.list_activities(ActivityFilter::Everything)?.is_none());

    Ok(())
}

#[rstest]
fn test_activity_store_update_activity_passes(activity_store: TestData) -> TestResult<()> {
    let TestData { activities, store } = activity_store;

    let og_activity = activities[0].clone();
    let og_activity_id = *og_activity.guid();

    let updated_test_desc = "Updated Test Description".to_string();
    let updated_test_cat = "Test::UpdatedCategory".to_string();

    let new_activity = Activity::builder()
        .description(updated_test_desc.to_string())
        .category(updated_test_cat)
        .build();

    let old_activity = store.update_activity(og_activity_id, new_activity.clone())?;

    assert_eq!(old_activity, og_activity);

    let stored_activity = store.read_activity(og_activity_id)?;

    assert_eq!(
        stored_activity.activity().begin(),
        og_activity.activity().begin()
    );

    assert_eq!(
        stored_activity.activity().begin(),
        og_activity.activity().begin()
    );

    Ok(())
}

#[rstest]
fn test_activity_store_delete_activity_passes(activity_store: TestData) -> TestResult<()> {
    let TestData { activities, store } = activity_store;

    let og_activity = activities[0].clone();
    let og_activity_id = *og_activity.guid();

    let activity = store.delete_activity(og_activity_id)?;

    assert!(store.read_activity(og_activity_id).is_err());

    assert_eq!(activity, og_activity);

    Ok(())
}

#[rstest]
fn test_activity_store_delete_activity_fails(activity_store: TestData) {
    let TestData {
        activities: _,
        store,
    } = activity_store;

    let activity_id = ActivityGuid::default();

    assert!(store.delete_activity(activity_id).is_err());
}

#[rstest]
fn test_activity_store_update_activity_fails(activity_store: TestData) {
    let TestData {
        activities: _,
        store,
    } = activity_store;

    let new_activity = Activity::builder()
        .description("test".to_string())
        .category("test".to_string())
        .build();

    let activity_id = ActivityGuid::default();

    assert!(store.update_activity(activity_id, new_activity).is_err());
}

#[rstest]
fn test_activity_store_begin_intermission_passes(
    activity_store_no_intermissions: TestData,
) -> PaceResult<()> {
    let TestData { activities, store } = activity_store_no_intermissions;

    let og_activity = activities
        .into_iter()
        .find(|a| a.activity().is_active())
        .expect("Should have an active activity.");

    let og_activity_id = og_activity.guid();

    let held_activity = store.hold_most_recent_active_activity(HoldOptions::default())?;

    assert!(held_activity.is_some());

    assert_eq!(og_activity, held_activity.unwrap());

    let active_intermissions = store
        .list_activities(ActivityFilter::ActiveIntermission)?
        .expect("Should have activities.")
        .into_vec();

    assert_eq!(
        active_intermissions.len(),
        1,
        "Should have one intermission."
    );

    let intermission = active_intermissions
        .first()
        .expect("Should have intermission.");

    let intermission = store
        .read_activity(*intermission)
        .expect("Should have intermission.");

    assert_eq!(
        intermission.activity().category(),
        &Some("Test::Intermission".to_string())
    );

    assert_eq!(
        intermission.activity().description(),
        &Some("Activity with Intermission".to_string())
    );

    assert_eq!(
        intermission.activity().is_active_intermission(),
        true,
        "Intermission should be considered active."
    );

    assert!(
        intermission.activity().activity_end_options().is_none(),
        "Intermission should not contain end options."
    );

    assert_eq!(
        intermission.activity().parent_id().unwrap(),
        *og_activity_id,
        "Parent ID should be the same as original activity."
    );

    Ok(())
}

#[rstest]
fn test_activity_store_begin_intermission_with_existing_does_nothing_passes(
    activity_store: TestData,
) -> TestResult<()> {
    let TestData { activities, store } = activity_store;

    assert!(
        store
            .hold_most_recent_active_activity(HoldOptions::default())?
            .is_some_and(|a| a.activity().is_active()),
        "Should contain and active activity."
    );

    assert_eq!(
        activities.len(),
        store
            .list_activities(ActivityFilter::Everything)?
            .expect("Should have activities.")
            .into_vec()
            .len(),
        "Should have no new activities."
    );

    // check that the intermission is still active
    let activities = store
        .list_activities(ActivityFilter::ActiveIntermission)?
        .expect("Should have activities.")
        .into_vec();

    let intermission = activities.first().expect("Should have intermission.");

    let intermission = store
        .read_activity(*intermission)
        .expect("Should have intermission.");

    assert_eq!(
        intermission.activity().category(),
        &Some("Test::Intermission".to_string())
    );

    assert_eq!(
        intermission.activity().description(),
        &Some("Activity with Intermission".to_string())
    );

    assert_eq!(
        intermission.activity().is_active_intermission(),
        true,
        "Intermission should be considered active."
    );

    Ok(())
}

#[rstest]
fn test_activity_store_end_intermission_passes(activity_store: TestData) -> TestResult<()> {
    let TestData {
        activities: og_activities,
        store,
    } = activity_store;

    let ended_intermissions = store
        .end_all_active_intermissions(EndOptions::default())?
        .unwrap();

    // There should be one ended intermission
    assert_eq!(
        ended_intermissions.len(),
        1,
        "Should have one intermission."
    );

    let intermission = ended_intermissions.first().unwrap();

    let intermission = store
        .read_activity(*intermission)
        .expect("Should have intermission.");

    let activities = store
        .list_activities(ActivityFilter::Everything)?
        .expect("Should have activities.")
        .into_vec();

    assert_eq!(
        activities.len(),
        og_activities.len(),
        "No new intermissions should be created."
    );

    assert!(
        intermission.activity().activity_end_options().is_some(),
        "Intermission should have end options."
    );

    assert_eq!(
        intermission.activity().is_active_intermission(),
        false,
        "Intermission shouldn't be considered active anymore."
    );

    Ok(())
}

#[rstest]
fn test_activity_store_end_intermission_with_empty_log_passes(
    activity_store_empty: TestData,
) -> TestResult<()> {
    let TestData {
        activities: _,
        store,
    } = activity_store_empty;

    let result = store.end_all_active_intermissions(EndOptions::default())?;

    assert!(result.is_none());

    Ok(())
}

#[rstest]
fn test_activity_store_resume_activity_passes(activity_store: TestData) -> PaceResult<()> {
    let TestData {
        activities: test_activities,
        store,
    } = activity_store;

    let activities = store
        .list_activities(ActivityFilter::Everything)?
        .expect("Should have activities.")
        .into_vec();

    dbg!(&activities);

    assert_eq!(activities.len(), test_activities.len());

    // let resumed_activity = store.resume_last_unfinished_activity(None)?;

    Ok(())
}
