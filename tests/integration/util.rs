use std::{collections::HashSet, path::Path, sync::Arc};

use chrono::Local;

use pace_core::prelude::{
    Activity, ActivityGuid, ActivityItem, ActivityKind, ActivityKindOptions, ActivityLog,
    ActivityStatusKind, ActivityStore,
};
use pace_error::TestResult;

use pace_storage::storage::file::TomlActivityStorage;
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
    use pace_core::prelude::PaceCategory;
    use pace_storage::storage::in_memory::InMemoryActivityStorage;
    use pace_time::date_time::PaceDateTime;

    let begin_time = PaceDateTime::default();

    let tags = vec!["test".to_string(), "activity".to_string()]
        .into_iter()
        .collect::<HashSet<String>>();

    let mut completed = Activity::builder()
        .description("Activity with end".to_string())
        .begin(begin_time)
        .status(ActivityStatusKind::Completed)
        .tags(tags.clone())
        .build();
    completed.end_activity_with_duration_calc(begin_time, PaceDateTime::now())?;

    let completed = ActivityItem::from((ActivityGuid::default(), completed));

    let mut archived_activity = Activity::builder()
        .description("Activity with end".to_string())
        .begin(begin_time)
        .status(ActivityStatusKind::Archived)
        .tags(tags.clone())
        .build();
    archived_activity.end_activity_with_duration_calc(begin_time, PaceDateTime::now())?;
    archived_activity.archive();

    let archived_activity = ActivityItem::from((ActivityGuid::default(), archived_activity));

    let time_30_min_ago = Local::now().with_timezone(&chrono::Utc).fixed_offset()
        - chrono::TimeDelta::try_minutes(30).ok_or("Should have time delta.")?;

    let begin_time = PaceDateTime::with_date_time_fixed_offset(time_30_min_ago);

    let intermission_begin_time = PaceDateTime::with_date_time_fixed_offset(
        time_30_min_ago + chrono::TimeDelta::try_minutes(15).ok_or("Should have time delta.")?,
    );

    let desc = "Activity with Intermission".to_string();

    let cat = PaceCategory::new("Test::Intermission");

    let paused = ActivityItem::from((
        ActivityGuid::default(),
        Activity::builder()
            .begin(begin_time)
            .description(desc.clone())
            .kind(ActivityKind::Activity)
            .status(ActivityStatusKind::Paused)
            .category(cat.clone())
            .tags(tags.clone())
            .build(),
    ));

    let in_progress = ActivityItem::from((
        ActivityGuid::default(),
        Activity::builder()
            .begin(begin_time)
            .description(desc.clone())
            .kind(ActivityKind::Activity)
            .status(ActivityStatusKind::InProgress)
            .category(cat.clone())
            .tags(tags.clone())
            .build(),
    ));

    let guid = paused.guid();

    let active_intermission = ActivityItem::from((
        ActivityGuid::default(),
        Activity::builder()
            .begin(intermission_begin_time)
            .kind(ActivityKind::Intermission)
            .status(ActivityStatusKind::InProgress)
            .description(desc)
            .category(cat)
            .activity_kind_options(ActivityKindOptions::with_parent_id(*guid))
            .build(),
    ));

    let created = ActivityItem::from((
        ActivityGuid::default(),
        Activity::builder()
            .description("Default activity, but no end and not active.")
            .status(ActivityStatusKind::Created)
            .tags(tags)
            .build(),
    ));

    let mut activities = vec![];

    match kind {
        ActivityStoreTestKind::Empty => (),
        ActivityStoreTestKind::WithActivitiesAndOpenIntermission => {
            activities.push(created);
            activities.push(archived_activity);
            activities.push(completed);
            activities.push(paused);
            activities.push(active_intermission);
        }
        ActivityStoreTestKind::WithoutIntermissions => {
            activities.push(created);
            activities.push(archived_activity);
            activities.push(completed);
            activities.push(in_progress);
        }
    }

    Ok(TestData {
        activities: activities.clone(),
        store: ActivityStore::with_storage(Arc::new(
            InMemoryActivityStorage::new_with_activity_log(ActivityLog::from_iter(activities)),
        ))?,
    })
}

#[fixture]
pub fn setup_activity_store_for_activity_tracker() -> TestResult<ActivityStore> {
    let fixture_path =
        Path::new("../../tests/fixtures/activity_tracker/activities.pace.toml").canonicalize()?;

    Ok(ActivityStore::with_storage(Arc::new(
        TomlActivityStorage::new(fixture_path)?,
    ))?)
}
