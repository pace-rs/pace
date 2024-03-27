use chrono::FixedOffset;
use eyre::OptionExt;
use pace_core::prelude::{Activity, ActivityReadOps, ActivityStateManagement, EndOptions};
use pace_error::TestResult;
use pace_storage::in_memory::InMemoryActivityStorage;
use pace_time::{date_time::PaceDateTime, duration::PaceDuration};

#[test]
#[allow(clippy::too_many_lines)]
fn test_begin_and_end_activity_in_different_time_zones_passes() -> TestResult<()> {
    let storage = InMemoryActivityStorage::new();

    // We start an activity in our time zone
    let now = PaceDateTime::now();

    let first_og_activity = Activity::builder()
        .description("Our time zone")
        .begin(now)
        .build();

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

    // use 3 hours as duration
    let artificial_duration = 3 * 60 * 60;

    // Now we end the activity in a different time zone
    let end = PaceDateTime::now_with_offset("-0200".parse::<FixedOffset>()?)
        .add_duration(PaceDuration::new(artificial_duration))?;

    let ended_activity = storage
        .end_last_unfinished_activity(EndOptions::builder().end_time(end).build())?
        .ok_or_eyre("Activity was not ended.")?;

    assert_eq!(
        first_og_activity.begin(),
        ended_activity.activity().begin(),
        "Ended activity has not the same begin time as the original activity."
    );

    assert_eq!(
        first_og_activity.description(),
        ended_activity.activity().description(),
        "Ended activity has not the same description as the original activity."
    );

    assert_eq!(
        first_og_activity.kind(),
        ended_activity.activity().kind(),
        "Ended activity has not the same kind as the original activity."
    );

    assert!(
        ended_activity.activity().status().is_completed(),
        "Activity is not ended."
    );

    let read_ended_activity = storage.read_activity(*ended_activity.guid())?;

    dbg!(&read_ended_activity);

    // check activity lasted 3 hours
    assert_eq!(
        read_ended_activity.activity().duration()?,
        PaceDuration::new(artificial_duration)
    );

    Ok(())
}
