use std::collections::BTreeMap;

use itertools::Itertools;
use rusqlite::Connection;
use tracing::debug;

use pace_core::prelude::{
    Activity, ActivityFilterKind, ActivityGuid, ActivityItem, ActivityKind, ActivityQuerying,
    ActivityReadOps, ActivityStateManagement, ActivityStatusKind, ActivityStorage,
    ActivityWriteOps, DeleteOptions, EndOptions, FilteredActivities, HoldOptions, KeywordOptions,
    PaceStorageOptResult, PaceStorageResult, ResumeOptions, SyncStorage, UpdateOptions,
};
use pace_time::{date::PaceDate, duration::PaceDurationRange, time_range::TimeRangeOptions};

use crate::{error::DatabaseStorageErrorKind, migration::SQLiteMigrator};

#[derive(Debug)]
pub struct SqliteActivityStorage {
    connection: Connection,
}

impl SqliteActivityStorage {
    pub fn new(connection_string: String) -> PaceStorageResult<Self> {
        let connection = Connection::open(connection_string.as_str()).map_err(|err| {
            DatabaseStorageErrorKind::ConnectionFailed(connection_string, err.to_string())
        })?;

        Ok(Self { connection })
    }
}

impl ActivityStorage for SqliteActivityStorage {
    fn setup(&self) -> PaceStorageResult<()> {
        let mut migrate = SQLiteMigrator::new(&self.connection)?;

        migrate.up()?;

        Ok(())
    }

    fn teardown(&self) -> PaceStorageResult<()> {
        // TODO: Do we need a teardown for sqlite?
        unimplemented!("teardown not yet implemented for sqlite storage")
    }

    fn identify(&self) -> String {
        "sqlite".to_string()
    }
}

impl SyncStorage for SqliteActivityStorage {
    fn sync(&self) -> PaceStorageResult<()> {
        // We sync activities to the database in each operation
        // so we don't need to do anything here

        Ok(())
    }
}

impl ActivityReadOps for SqliteActivityStorage {
    #[tracing::instrument]
    fn read_activity(&self, activity_id: ActivityGuid) -> PaceStorageResult<ActivityItem> {
        // let mut stmt = self
        //     .connection
        //     .prepare("SELECT * FROM activities WHERE id = ?1")?;

        // let activity_item_iter =
        //     stmt.query_map(&[&activity_id], |row| Ok(ActivityItem::from_row(&row)))?;

        // let activity_item = activity_item_iter
        //     .filter_map_ok(|item| item.ok())
        //     .next()
        //     .transpose()?
        //     .ok_or(DatabaseStorageErrorKind::ActivityNotFound(activity_id))?;

        // debug!("Read activity: {:?}", activity_item);

        // Ok(activity_item)
        todo!("implement read_activity for sqlite")
    }

    #[tracing::instrument]
    fn list_activities(
        &self,
        filter: ActivityFilterKind,
    ) -> PaceStorageOptResult<FilteredActivities> {
        // let mut stmt = self.connection.prepare(filter.to_sql_statement())?;

        // let activity_item_iter = stmt.query_map([], |row| Ok(ActivityGuid::from_row(&row)))?;

        // let activities = activity_item_iter
        //     .filter_map_ok(|item| item.ok())
        //     .collect::<Result<Vec<ActivityGuid>, _>>()?;

        // debug!("Listed activities: {:?}", activities);

        // if activities.is_empty() {
        //     return Ok(None);
        // }

        // let filtered_activities = match filter {
        //     ActivityFilterKind::Everything => FilteredActivities::Everything(activities),
        //     ActivityFilterKind::OnlyActivities => FilteredActivities::OnlyActivities(activities),
        //     ActivityFilterKind::Active => FilteredActivities::Active(activities),
        //     ActivityFilterKind::ActiveIntermission => {
        //         FilteredActivities::ActiveIntermission(activities)
        //     }
        //     ActivityFilterKind::Archived => FilteredActivities::Archived(activities),
        //     ActivityFilterKind::Ended => FilteredActivities::Ended(activities),
        //     ActivityFilterKind::Held => FilteredActivities::Held(activities),
        //     ActivityFilterKind::Intermission => FilteredActivities::Intermission(activities),
        //     ActivityFilterKind::TimeRange(_) => FilteredActivities::TimeRange(activities),
        // };

        // Ok(Some(filtered_activities))

        todo!("implement list_activities for sqlite")
    }
}

impl ActivityWriteOps for SqliteActivityStorage {
    fn create_activity(&self, activity: Activity) -> PaceStorageResult<ActivityItem> {
        // let tx = self.connection.transaction()?;

        // let mut stmt = tx.prepare(activity.to_sql_prepare_statement())?;

        // let (guid, params) = activity.to_sql_execute_statement()?;

        // if stmt.execute(params.as_slice())? > 0 {
        //     tx.commit()?;
        //     return Ok(ActivityItem::from((guid, activity)));
        // }

        // return Err(DatabaseStorageErrorKind::ActivityCreationFailed(activity).into());
        todo!("implement create_activity for sqlite")
    }

    fn update_activity(
        &self,
        activity_id: ActivityGuid,
        updated_activity: Activity,
        update_opts: UpdateOptions,
    ) -> PaceStorageResult<ActivityItem> {
        todo!()
    }

    fn delete_activity(
        &self,
        activity_id: ActivityGuid,
        delete_opts: DeleteOptions,
    ) -> PaceStorageResult<ActivityItem> {
        // let activity = self.read_activity(activity_id)?;

        // let tx = self.connection.transaction()?;
        // let mut stmt = tx.prepare("DELETE FROM activities WHERE id = ?1 LIMIT = 1")?;

        // if stmt.execute(&[&activity_id])? == 1 {
        //     tx.commit()?;
        //     return Ok(activity);
        // }

        // Err(DatabaseStorageErrorKind::ActivityDeletionFailed(activity_id).into())
        todo!("implement delete_activity for sqlite")
    }
}
impl ActivityStateManagement for SqliteActivityStorage {
    fn hold_activity(
        &self,
        activity_id: ActivityGuid,
        hold_opts: HoldOptions,
    ) -> PaceStorageResult<ActivityItem> {
        todo!()
    }

    fn resume_activity(
        &self,
        activity_id: ActivityGuid,
        resume_opts: ResumeOptions,
    ) -> PaceStorageResult<ActivityItem> {
        todo!()
    }

    fn resume_most_recent_activity(
        &self,
        resume_opts: ResumeOptions,
    ) -> PaceStorageOptResult<ActivityItem> {
        todo!()
    }

    fn end_activity(
        &self,
        activity_id: ActivityGuid,
        end_opts: EndOptions,
    ) -> PaceStorageResult<ActivityItem> {
        todo!()
    }

    fn end_all_activities(&self, end_opts: EndOptions) -> PaceStorageOptResult<Vec<ActivityItem>> {
        todo!()
    }

    fn end_all_active_intermissions(
        &self,
        end_opts: EndOptions,
    ) -> PaceStorageOptResult<Vec<ActivityGuid>> {
        todo!()
    }

    fn end_last_unfinished_activity(
        &self,
        end_opts: EndOptions,
    ) -> PaceStorageOptResult<ActivityItem> {
        todo!()
    }

    fn hold_most_recent_active_activity(
        &self,
        hold_opts: HoldOptions,
    ) -> PaceStorageOptResult<ActivityItem> {
        todo!()
    }
}
impl ActivityQuerying for SqliteActivityStorage {
    fn group_activities_by_duration_range(
        &self,
    ) -> PaceStorageOptResult<BTreeMap<PaceDurationRange, Vec<ActivityItem>>> {
        todo!()
    }

    fn group_activities_by_start_date(
        &self,
    ) -> PaceStorageOptResult<BTreeMap<PaceDate, Vec<ActivityItem>>> {
        todo!()
    }

    fn list_activities_with_intermissions(
        &self,
    ) -> PaceStorageOptResult<BTreeMap<ActivityGuid, Vec<ActivityItem>>> {
        todo!()
    }

    fn group_activities_by_keywords(
        &self,
        keyword_opts: KeywordOptions,
    ) -> PaceStorageOptResult<BTreeMap<String, Vec<ActivityItem>>> {
        todo!()
    }

    fn group_activities_by_kind(
        &self,
    ) -> PaceStorageOptResult<BTreeMap<ActivityKind, Vec<ActivityItem>>> {
        todo!()
    }

    fn list_activities_by_time_range(
        &self,
        time_range_opts: TimeRangeOptions,
    ) -> PaceStorageOptResult<Vec<ActivityGuid>> {
        todo!()
    }

    fn group_activities_by_status(
        &self,
    ) -> PaceStorageOptResult<BTreeMap<ActivityStatusKind, Vec<ActivityItem>>> {
        todo!()
    }

    fn list_activities_by_id(&self) -> PaceStorageOptResult<BTreeMap<ActivityGuid, Activity>> {
        todo!()
    }
}
