//! Test the `ActivityStore` implementation with a `InMemoryStorage` backend.

use std::sync::Arc;

use pace_core::prelude::{
    Activity, ActivityFilterKind, ActivityGuid, ActivityReadOps, ActivityStateManagement,
    ActivityStatusKind, ActivityStore, ActivityWriteOps, DeleteOptions, EndOptions, HoldOptions,
    PaceCategory, PaceDescription, PaceTagCollection, ResumeOptions, UpdateOptions,
};
use pace_error::TestResult;
use pace_storage::storage::in_memory::InMemoryActivityStorage;

use crate::util::{
    activity_store, activity_store_empty, activity_store_no_intermissions, TestData,
};

use rstest::rstest;
use similar_asserts::assert_eq;

#[rstest]
fn test_activity_store_create_activity_passes(
    activity_store_empty: TestResult<TestData>,
) -> TestResult<()> {
    let TestData {
        activities: _,
        store,
    } = activity_store_empty?;

    let activity = Activity::builder()
        .description(PaceDescription::new("Test Description"))
        .category(PaceCategory::new("Test::Category"))
        .build();

    let og_activity = activity.clone();

    let og_activity_item = store.create_activity(activity)?;

    let stored_activity = store.read_activity(*og_activity_item.guid())?;

    assert_eq!(
        *stored_activity.activity(),
        og_activity,
        "Should have the same activity."
    );

    Ok(())
}

#[rstest]
fn test_activity_store_read_activity_passes(
    activity_store: TestResult<TestData>,
) -> TestResult<()> {
    let TestData { activities, store } = activity_store?;

    let og_activity = activities[0].clone();
    let og_activity_id = *og_activity.guid();

    let stored_activity = store.read_activity(og_activity_id)?;

    assert_eq!(
        stored_activity, og_activity,
        "Should have the same activity."
    );

    Ok(())
}

#[rstest]
fn test_activity_store_read_activity_fails(activity_store: TestResult<TestData>) -> TestResult<()> {
    let TestData {
        activities: _,
        store,
    } = activity_store?;

    let activity_id = ActivityGuid::default();

    assert!(
        store.read_activity(activity_id).is_err(),
        "Can't read activity with non-existing ID."
    );

    Ok(())
}

#[rstest]
fn test_activity_store_list_activities_returns_none_on_empty_passes(
    activity_store_empty: TestResult<TestData>,
) -> TestResult<()> {
    let TestData {
        activities: _,
        store,
    } = activity_store_empty?;

    assert!(
        store
            .list_activities(ActivityFilterKind::Everything)?
            .is_none(),
        "Should have no activities."
    );

    Ok(())
}

// List activities can hardly fail, as it returns an empty list if no activities are found.
// Therefore, we only test the success case. It would fail if the RwLock is poisoned.
#[rstest]
fn test_activity_store_list_activities_passes(
    activity_store: TestResult<TestData>,
) -> TestResult<()> {
    use strum::IntoEnumIterator;

    let TestData { activities, store } = activity_store?;

    for filter in ActivityFilterKind::iter() {
        let loaded_activities = store
            .list_activities(filter)?
            .ok_or("Should have activities")?;

        match filter {
            ActivityFilterKind::OnlyActivities => {
                assert_eq!(
                    4,
                    loaded_activities.clone().into_vec().len(),
                    "Should have only 4 activities."
                );
            }
            ActivityFilterKind::Archived => {
                assert_eq!(
                    1,
                    loaded_activities.into_vec().len(),
                    "Should have one archived."
                );
            }
            ActivityFilterKind::ActiveIntermission => {
                assert_eq!(
                    1,
                    loaded_activities.into_vec().len(),
                    "Should have one active intermission."
                );
            }
            ActivityFilterKind::Active => {
                assert_eq!(
                    1,
                    loaded_activities.into_vec().len(),
                    "Should have one active activities."
                );
            }
            ActivityFilterKind::Ended => {
                assert_eq!(
                    1,
                    loaded_activities.into_vec().len(),
                    "Should have one ended activity."
                );
            }
            ActivityFilterKind::Everything => {
                assert_eq!(
                    activities.len(),
                    loaded_activities.into_vec().len(),
                    "Should be the same length as initial activities."
                );
            }
            ActivityFilterKind::Held => {
                assert_eq!(
                    1,
                    loaded_activities.into_vec().len(),
                    "Should have one held activity."
                );
            }
            ActivityFilterKind::Intermission => {
                assert_eq!(
                    1,
                    loaded_activities.into_vec().len(),
                    "Should have one intermission."
                );
            }
            ActivityFilterKind::TimeRange(_) => {
                // We don't have any time range here, so we can't test this.
            }
        }
    }

    Ok(())
}

#[rstest]
fn test_activity_store_list_ended_activities_passes(
    activity_store: TestResult<TestData>,
) -> TestResult<()> {
    let TestData {
        activities: _,
        store,
    } = activity_store?;

    let loaded_activities = store
        .list_activities(ActivityFilterKind::Ended)?
        .ok_or("Should have activities.")?;

    assert_eq!(
        1,
        loaded_activities.into_vec().len(),
        "Should have one activity."
    );

    Ok(())
}

#[rstest]
fn test_activity_store_list_all_activities_passes(
    activity_store: TestResult<TestData>,
) -> TestResult<()> {
    let TestData { activities, store } = activity_store?;

    let loaded_activities = store
        .list_activities(ActivityFilterKind::Everything)?
        .ok_or("Should have activities.")?;

    assert_eq!(
        activities.len(),
        loaded_activities.into_vec().len(),
        "Should have activities."
    );

    Ok(())
}

#[rstest]
fn test_activity_store_list_all_activities_empty_result_passes(
    activity_store_empty: TestResult<TestData>,
) -> TestResult<()> {
    let TestData {
        activities: _,
        store,
    } = activity_store_empty?;

    assert!(
        store
            .list_activities(ActivityFilterKind::Everything)?
            .is_none(),
        "Should have no activities."
    );

    Ok(())
}

#[rstest]
fn test_activity_store_update_activity_passes(
    activity_store: TestResult<TestData>,
) -> TestResult<()> {
    let TestData { activities, store } = activity_store?;

    let tags = vec!["bla".to_string(), "cookie".to_string()]
        .into_iter()
        .collect::<PaceTagCollection>();

    let og_activity = activities[0].clone();
    let og_activity_id = *og_activity.guid();

    let updated_test_desc = PaceDescription::new("Updated Test Description");
    let updated_test_cat = PaceCategory::new("Test::UpdatedCategory");

    let new_activity = Activity::builder()
        .description(updated_test_desc.clone())
        .category(updated_test_cat.clone())
        .tags(tags.clone())
        .build();

    let old_activity =
        store.update_activity(og_activity_id, new_activity, UpdateOptions::default())?;

    assert_eq!(old_activity, og_activity, "Should have the same activity.");

    let stored_activity = store.read_activity(og_activity_id)?;

    // INFO: This should not have been updated, as we haven't set it explicitly.
    // If we set it, it should update, we test that directly in the in-memory storage tests.
    assert_eq!(
        stored_activity.activity().begin(),
        og_activity.activity().begin(),
        "Begin should not have been updated."
    );

    assert_eq!(
        stored_activity.activity().description(),
        &updated_test_desc,
        "Description should have been updated."
    );

    assert_eq!(
        stored_activity.activity().category(),
        &Some(updated_test_cat),
        "Category should have been updated."
    );

    assert_eq!(
        stored_activity.activity().tags(),
        &Some(tags),
        "Tags should have been updated."
    );

    Ok(())
}

#[rstest]
fn test_activity_store_delete_activity_passes(
    activity_store: TestResult<TestData>,
) -> TestResult<()> {
    let TestData { activities, store } = activity_store?;

    let og_activity = activities[0].clone();
    let og_activity_id = *og_activity.guid();

    let deleted_activity = store.delete_activity(og_activity_id, DeleteOptions::default())?;

    assert!(
        store.read_activity(og_activity_id).is_err(),
        "Should not exist anymore."
    );

    assert_eq!(
        deleted_activity, og_activity,
        "Should have the same activity."
    );

    Ok(())
}

#[rstest]
fn test_activity_store_delete_activity_fails(
    activity_store: TestResult<TestData>,
) -> TestResult<()> {
    let TestData {
        activities: _,
        store,
    } = activity_store?;

    let activity_id = ActivityGuid::default();

    assert!(
        store
            .delete_activity(activity_id, DeleteOptions::default())
            .is_err(),
        "Can't delete activity with non-existing ID."
    );

    Ok(())
}

#[rstest]
fn test_activity_store_update_activity_fails(
    activity_store: TestResult<TestData>,
) -> TestResult<()> {
    let TestData {
        activities: _,
        store,
    } = activity_store?;

    let new_activity = Activity::builder()
        .description(PaceDescription::new("test"))
        .category(PaceCategory::new("test"))
        .build();

    let activity_id = ActivityGuid::default();

    assert!(
        store
            .update_activity(activity_id, new_activity, UpdateOptions::default())
            .is_err(),
        "Can't update activity with non-existing ID."
    );

    Ok(())
}

#[rstest]
fn test_activity_store_begin_intermission_passes(
    activity_store_no_intermissions: TestResult<TestData>,
) -> TestResult<()> {
    let TestData { activities, store } = activity_store_no_intermissions?;

    let og_activity = activities
        .into_iter()
        .find(|a| a.activity().is_in_progress())
        .ok_or("Should have an active activity.")?;

    let og_activity_id = og_activity.guid();

    let held_activity = store.hold_most_recent_active_activity(HoldOptions::default())?;

    assert!(held_activity.is_some(), "Should return an active activity.");

    let mut edited_activity = og_activity.clone();
    let edited_activity = edited_activity
        .activity_mut()
        .set_status(ActivityStatusKind::Paused)
        .clone();

    assert_eq!(
        edited_activity,
        *held_activity.ok_or("Should have activity.")?.activity(),
        "Activity should be the same after holding."
    );

    let active_intermissions = store
        .list_activities(ActivityFilterKind::ActiveIntermission)?
        .ok_or("Should have activities.")?
        .into_vec();

    assert_eq!(
        active_intermissions.len(),
        1,
        "Should have one intermission."
    );

    let intermission = active_intermissions
        .first()
        .ok_or("Should have intermission.")?;

    let intermission = store.read_activity(*intermission)?;

    assert_eq!(
        intermission.activity().category(),
        &Some(PaceCategory::new("Test::Intermission"))
    );

    assert_eq!(
        intermission.activity().description(),
        &PaceDescription::new("Activity with Intermission")
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
        intermission
            .activity()
            .parent_id()
            .ok_or("Should have parent ID.")?,
        *og_activity_id,
        "Parent ID should be the same as original activity."
    );

    Ok(())
}

#[rstest]
fn test_activity_store_begin_intermission_with_existing_does_nothing_passes(
    activity_store: TestResult<TestData>,
) -> TestResult<()> {
    let TestData { activities, store } = activity_store?;

    assert!(
        store
            .hold_most_recent_active_activity(HoldOptions::default())?
            .is_none(),
        "Should not contain an active activity."
    );

    assert_eq!(
        activities.len(),
        store
            .list_activities(ActivityFilterKind::Everything)?
            .ok_or("Should have activities.")?
            .into_vec()
            .len(),
        "Should have no new activities."
    );

    // check that the intermission is still active
    let activities = store
        .list_activities(ActivityFilterKind::ActiveIntermission)?
        .ok_or("Should have activities.")?
        .into_vec();

    let intermission = activities.first().ok_or("Should have intermission.")?;

    let intermission = store.read_activity(*intermission)?;

    assert_eq!(
        intermission.activity().category(),
        &Some(PaceCategory::new("Test::Intermission")),
        "Category should be the same as original activity."
    );

    assert_eq!(
        intermission.activity().description(),
        &PaceDescription::new("Activity with Intermission")
    );

    assert_eq!(
        intermission.activity().is_active_intermission(),
        true,
        "Intermission should be considered active."
    );

    Ok(())
}

#[rstest]
fn test_activity_store_end_intermission_passes(
    activity_store: TestResult<TestData>,
) -> TestResult<()> {
    let TestData {
        activities: og_activities,
        store,
    } = activity_store?;

    let ended_intermissions = store
        .end_all_active_intermissions(EndOptions::default())?
        .ok_or("Should have ended intermissions.")?;

    // There should be one ended intermission
    assert_eq!(
        ended_intermissions.len(),
        1,
        "Should have one intermission."
    );

    let intermission = ended_intermissions
        .first()
        .ok_or("Should have intermission.")?;

    let intermission = store.read_activity(*intermission)?;

    let activities = store
        .list_activities(ActivityFilterKind::Everything)?
        .ok_or("Should have activities.")?
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
    activity_store_empty: TestResult<TestData>,
) -> TestResult<()> {
    let TestData {
        activities: _,
        store,
    } = activity_store_empty?;

    let result = store.end_all_active_intermissions(EndOptions::default())?;

    assert!(result.is_none(), "Should have no intermissions.");

    Ok(())
}

#[rstest]
fn test_activity_store_resume_activity_passes(
    activity_store: TestResult<TestData>,
) -> TestResult<()> {
    let TestData {
        activities: test_activities,
        store,
    } = activity_store?;

    let activities = store
        .list_activities(ActivityFilterKind::Everything)?
        .ok_or("Should have activities.")?
        .into_vec();

    assert_eq!(
        activities.len(),
        test_activities.len(),
        "Should have activities."
    );

    let held_activity = store
        .list_activities(ActivityFilterKind::Held)?
        .ok_or("Should have activities.")?
        .into_vec();

    assert_eq!(held_activity.len(), 1, "Should have one held activity.");

    let active_intermission = store
        .list_activities(ActivityFilterKind::ActiveIntermission)?
        .ok_or("Should have activities.")?
        .into_vec();

    assert_eq!(
        active_intermission.len(),
        1,
        "Should have one active intermission."
    );

    let resumed_activity = store
        .resume_most_recent_activity(ResumeOptions::default())?
        .ok_or("Should have an activity.")?;

    assert!(
        store
            .list_activities(ActivityFilterKind::ActiveIntermission)?
            .is_none(),
        "Should have no active intermissions."
    );

    let resumed_activity = store.read_activity(*resumed_activity.guid())?;

    assert!(
        resumed_activity.activity().status().is_in_progress(),
        "Activity should be active again."
    );

    Ok(())
}

#[rstest]
fn test_begin_activity_with_held_activity() -> TestResult<()> {
    let store = ActivityStore::with_storage(Arc::new(InMemoryActivityStorage::new()))?;

    // Begin activity
    let activity = Activity::builder()
        .description(PaceDescription::new("Test Description"))
        .build();

    let activity = store.begin_activity(activity)?;

    let read_activity = store.read_activity(*activity.guid())?;

    assert!(
        read_activity.activity().status().is_in_progress(),
        "Activity should be active."
    );

    // Hold this activity
    let _held_activity = store.hold_most_recent_active_activity(HoldOptions::default())?;

    let held_activity = store.read_activity(*read_activity.guid())?;

    assert!(
        held_activity.activity().status().is_paused(),
        "Activity should be held."
    );

    // Begin another activity although there is a held activity
    let new_activity = Activity::builder()
        .description(PaceDescription::new("New Description"))
        .build();

    let new_activity = store.begin_activity(new_activity)?;

    let new_activity = store.read_activity(*new_activity.guid())?;

    assert!(
        new_activity.activity().status().is_in_progress(),
        "Activity should be active."
    );

    assert!(
        store
            .read_activity(*held_activity.guid())?
            .activity()
            .status()
            .is_completed(),
        "Held activity should be ended."
    );

    assert!(
        store
            .list_activities(ActivityFilterKind::ActiveIntermission)?
            .is_none(),
        "Should have no active intermissions."
    );

    assert!(
        store
            .resume_most_recent_activity(ResumeOptions::default())?
            .is_none(),
        "Should have no activity to resume."
    );

    Ok(())
}
