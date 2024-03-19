use std::{collections::HashSet, path::Path, sync::Arc};

use chrono::{DateTime, Local, NaiveDateTime};
use pace_core::{
    Activity, ActivityGuid, ActivityItem, ActivityKind, ActivityKindOptions, ActivityLog,
    ActivityStatus, ActivityStore, InMemoryActivityStorage, PaceNaiveDateTime, TestResult,
    TomlActivityStorage,
};

use rstest::fixture;

pub struct TestData {
    pub activities: Vec<ActivityItem>,
    pub store: ActivityStore,
}

pub enum ActivityStoreTestKind {
    Empty,
    WithActivitiesAndOpenIntermission,
    WithoutIntermissions,
}

#[fixture]
pub fn activity_store_empty() -> TestResult<TestData> {
    setup_activity_store(&ActivityStoreTestKind::Empty)
}

#[fixture]
pub fn activity_store() -> TestResult<TestData> {
    setup_activity_store(&ActivityStoreTestKind::WithActivitiesAndOpenIntermission)
}

#[fixture]
pub fn activity_store_no_intermissions() -> TestResult<TestData> {
    setup_activity_store(&ActivityStoreTestKind::WithoutIntermissions)
}

/// Sets up an activity store with activities for testing
///
/// # Arguments
///
/// * `kind` - The kind of activities for the store to setup
///
/// # Errors
///
/// Returns an error if the store cannot be created
///
/// # Returns
///
/// A `TestData` struct containing the activities and the store
#[allow(clippy::too_many_lines)]
// We need to use `#[cfg(not(tarpaulin_include))]` to exclude this from coverage reports
#[cfg(not(tarpaulin_include))]
pub fn setup_activity_store(kind: &ActivityStoreTestKind) -> TestResult<TestData> {
    let begin_time = PaceNaiveDateTime::new(NaiveDateTime::new(
        DateTime::from_timestamp(0, 0)
            .ok_or("Should have date time.")?
            .naive_local()
            .date(),
        DateTime::from_timestamp(0, 0)
            .ok_or("Should have date time.")?
            .naive_local()
            .time(),
    ));

    let tags = vec!["test".to_string(), "activity".to_string()]
        .into_iter()
        .collect::<HashSet<String>>();

    let mut ended_activity = Activity::builder()
        .description("Activity with end".to_string())
        .begin(begin_time)
        .status(ActivityStatus::Ended)
        .tags(tags.clone())
        .build();
    ended_activity.end_activity_with_duration_calc(begin_time, PaceNaiveDateTime::now())?;

    let ended_activity = ActivityItem::from((ActivityGuid::default(), ended_activity));

    let mut archived_activity = Activity::builder()
        .description("Activity with end".to_string())
        .begin(begin_time)
        .status(ActivityStatus::Archived)
        .tags(tags.clone())
        .build();
    archived_activity.end_activity_with_duration_calc(begin_time, PaceNaiveDateTime::now())?;
    archived_activity.archive();

    let archived_activity = ActivityItem::from((ActivityGuid::default(), archived_activity));

    let time_30_min_ago = Local::now().naive_local()
        - chrono::TimeDelta::try_minutes(30).ok_or("Should have time delta.")?;
    let begin_time = PaceNaiveDateTime::new(time_30_min_ago);
    let intermission_begin_time = PaceNaiveDateTime::new(
        time_30_min_ago + chrono::TimeDelta::try_minutes(15).ok_or("Should have time delta.")?,
    );
    let desc = "Activity with Intermission".to_string();
    let cat = "Test::Intermission".to_string();

    let held = ActivityItem::from((
        ActivityGuid::default(),
        Activity::builder()
            .begin(begin_time)
            .description(desc.clone())
            .kind(ActivityKind::Activity)
            .status(ActivityStatus::Held)
            .category(cat.clone())
            .tags(tags.clone())
            .build(),
    ));

    let active = ActivityItem::from((
        ActivityGuid::default(),
        Activity::builder()
            .begin(begin_time)
            .description(desc.clone())
            .kind(ActivityKind::Activity)
            .status(ActivityStatus::Active)
            .category(cat.clone())
            .tags(tags.clone())
            .build(),
    ));

    let guid = held.guid();

    let active_intermission = ActivityItem::from((
        ActivityGuid::default(),
        Activity::builder()
            .begin(intermission_begin_time)
            .kind(ActivityKind::Intermission)
            .status(ActivityStatus::Active)
            .description(desc)
            .category(cat)
            .activity_kind_options(ActivityKindOptions::with_parent_id(*guid))
            .build(),
    ));

    let inactive = ActivityItem::from((
        ActivityGuid::default(),
        Activity::builder()
            .description("Default activity, but no end and not active.")
            .status(ActivityStatus::Inactive)
            .tags(tags)
            .build(),
    ));

    let mut activities = vec![];

    match kind {
        ActivityStoreTestKind::Empty => (),
        ActivityStoreTestKind::WithActivitiesAndOpenIntermission => {
            activities.push(inactive);
            activities.push(archived_activity);
            activities.push(ended_activity);
            activities.push(held);
            activities.push(active_intermission);
        }
        ActivityStoreTestKind::WithoutIntermissions => {
            activities.push(inactive);
            activities.push(archived_activity);
            activities.push(ended_activity);
            activities.push(active);
        }
    }

    Ok(TestData {
        activities: activities.clone(),
        store: ActivityStore::with_storage(Arc::new(
            InMemoryActivityStorage::new_with_activity_log(ActivityLog::from_iter(activities))
                .into(),
        ))?,
    })
}

#[fixture]
pub fn setup_activity_store_for_activity_tracker() -> TestResult<ActivityStore> {
    let fixture_path =
        Path::new("../../tests/fixtures/activity_tracker/activities.pace.toml").canonicalize()?;

    let storage = TomlActivityStorage::new(fixture_path)?;

    Ok(ActivityStore::with_storage(Arc::new(storage.into()))?)
}
