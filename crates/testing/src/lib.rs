use std::{collections::HashSet, sync::Arc};

use chrono::{Local, NaiveDateTime};
use pace_core::{
    Activity, ActivityGuid, ActivityItem, ActivityKind, ActivityKindOptions, ActivityLog,
    ActivityReadOps, ActivityStateManagement, ActivityStatus, ActivityStatusFilter, ActivityStore,
    ActivityWriteOps, DeleteOptions, EndOptions, HoldOptions, InMemoryActivityStorage,
    PaceDateTime, ResumeOptions, TestResult, UpdateOptions,
};

use rstest::{fixture, rstest};
use similar_asserts::assert_eq;

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

pub fn setup_activity_store(kind: &ActivityStoreTestKind) -> TestResult<TestData> {
    let begin_time = PaceDateTime::new(NaiveDateTime::new(
        NaiveDateTime::from_timestamp_opt(0, 0)
            .ok_or("Should have date time.")?
            .date(),
        NaiveDateTime::from_timestamp_opt(0, 0)
            .ok_or("Should have date time.")?
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
    ended_activity.end_activity_with_duration_calc(begin_time, PaceDateTime::now())?;

    let ended_activity = ActivityItem::from((ActivityGuid::default(), ended_activity));

    let mut archived_activity = Activity::builder()
        .description("Activity with end".to_string())
        .begin(begin_time)
        .status(ActivityStatus::Archived)
        .tags(tags.clone())
        .build();
    archived_activity.end_activity_with_duration_calc(begin_time, PaceDateTime::now())?;
    archived_activity.archive();

    let archived_activity = ActivityItem::from((ActivityGuid::default(), archived_activity));

    let time_30_min_ago = Local::now().naive_local() - chrono::Duration::minutes(30);
    let begin_time = PaceDateTime::new(time_30_min_ago);
    let intermission_begin_time =
        PaceDateTime::new(time_30_min_ago + chrono::Duration::minutes(15));
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
