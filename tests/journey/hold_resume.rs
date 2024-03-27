use pace_core::prelude::{
    Activity, ActivityQuerying, ActivityReadOps, ActivityStateManagement, HoldOptions,
    InMemoryActivityStorage, ResumeOptions, TestResult,
};

#[test]
#[allow(clippy::too_many_lines)]
fn test_hold_resume_journey_for_activities_passes() -> TestResult<()> {
    let storage = InMemoryActivityStorage::new();

    let first_og_activity = Activity::builder().description("Test activity").build();

    let first_begin_activity = storage.begin_activity(first_og_activity.clone())?;

    let first_stored_activity = storage.read_activity(*first_begin_activity.guid())?;

    assert_eq!(
        first_og_activity.begin(),
        first_stored_activity.activity().begin(),
        "Stored activity has not the same begin time as the original activity."
    );

    assert_eq!(
        first_og_activity.description(),
        first_stored_activity.activity().description(),
        "Stored activity has not the same description as the original activity."
    );

    assert_eq!(
        first_og_activity.kind(),
        first_stored_activity.activity().kind(),
        "Stored activity has not the same kind as the original activity."
    );

    assert_ne!(
            first_og_activity.status(),
            first_stored_activity.activity().status(),
            "Stored activity has the same status as the original activity. Which can't be, because it should be active."
        );

    assert!(
        first_stored_activity.activity().status().is_in_progress(),
        "Stored activity is not active."
    );

    assert!(
        first_og_activity.status().is_created(),
        "Original activity is not inactive."
    );

    // Now we create another activity, which should end the first one automatically

    let second_og_activity = Activity::builder().description("Our new activity").build();

    let second_begin_activity = storage.begin_activity(second_og_activity.clone())?;

    let second_stored_activity = storage.read_activity(*second_begin_activity.guid())?;

    let first_stored_activity = storage.read_activity(*first_begin_activity.guid())?;

    assert!(
        first_stored_activity.activity().status().is_completed(),
        "First activity is not ended."
    );

    assert_eq!(
        second_og_activity.begin(),
        second_stored_activity.activity().begin(),
        "Stored activity has not the same begin time as the original activity."
    );

    assert_eq!(
        second_og_activity.description(),
        second_stored_activity.activity().description(),
        "Stored activity has not the same description as the original activity."
    );

    assert_eq!(
        second_og_activity.kind(),
        second_stored_activity.activity().kind(),
        "Stored activity has not the same kind as the original activity."
    );

    assert_ne!(
            second_og_activity.status(),
            second_stored_activity.activity().status(),
            "Stored activity has the same status as the original activity. Which can't be, because it should be active."
        );

    assert!(
        second_stored_activity.activity().status().is_in_progress(),
        "Stored activity is not active."
    );

    assert!(
        second_og_activity.status().is_created(),
        "Original activity is not inactive."
    );

    // Now we create an intermission for the second activity

    let _ = storage
        .hold_most_recent_active_activity(HoldOptions::default())?
        .ok_or("Activity was not held.")?;

    let second_stored_activity = storage.read_activity(*second_begin_activity.guid())?;

    assert!(
        second_stored_activity.activity().status().is_paused(),
        "Second activity is not held."
    );

    // This is more complicated, but maybe also on purpose, as directly dealing with the intermission
    // is not the most common use case and should be discouraged as messing with it could lead to
    // inconsistencies in the data.
    let second_activity_intermission_id = storage
        .list_active_intermissions_for_activity_id(*second_begin_activity.guid())?
        .ok_or("Intermission was not created.")?;

    let second_activity_intermission_id = second_activity_intermission_id
        .first()
        .ok_or("Intermission was not created, but the ID was not found.")?;

    let second_stored_intermission = storage.read_activity(*second_activity_intermission_id)?;

    assert_eq!(
        second_stored_intermission
            .activity()
            .activity_kind_options()
            .as_ref()
            .ok_or("Activity kind options not set.")?
            .parent_id()
            .ok_or("Parent ID not set.")?,
        *second_begin_activity.guid(),
        "Parent IDs of intermission and parent activity do not match."
    );

    // Now we want to continue the activity, which should end the intermission automatically
    // and set the activity from held to active again

    let resumed_activity = storage
        .resume_most_recent_activity(ResumeOptions::default())?
        .ok_or("Activity was not resumed.")?;

    let resumed_stored_activity = storage.read_activity(*resumed_activity.guid())?;

    let second_stored_intermission = storage.read_activity(*second_activity_intermission_id)?;

    assert!(
        resumed_stored_activity.activity().status().is_in_progress(),
        "Resumed activity is not active."
    );

    assert!(
        second_stored_intermission
            .activity()
            .status()
            .is_completed(),
        "Intermission has not ended."
    );

    assert!(
        second_stored_intermission.activity().is_completed(),
        "Intermission has not ended."
    );

    assert!(
        resumed_stored_activity.activity().status().is_in_progress(),
        "Resumed activity is not active."
    );

    assert_eq!(
        resumed_stored_activity.guid(),
        second_stored_activity.guid(),
        "Resumed activity is not the same as the second stored activity."
    );

    assert_eq!(
        resumed_stored_activity.activity().begin(),
        second_stored_activity.activity().begin(),
        "Resumed activity has not the same begin time as the second stored activity."
    );

    assert_eq!(
        resumed_stored_activity.activity().description(),
        second_stored_activity.activity().description(),
        "Resumed activity has not the same description as the second stored activity."
    );

    assert_eq!(
        resumed_stored_activity.activity().kind(),
        second_stored_activity.activity().kind(),
        "Resumed activity has not the same kind as the second stored activity."
    );

    assert!(!resumed_stored_activity.activity().is_completed());

    Ok(())
}
