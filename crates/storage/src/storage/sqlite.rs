use std::collections::BTreeMap;

use itertools::Itertools;
use rusqlite::Connection;

use pace_core::prelude::{
    Activity, ActivityFilterKind, ActivityGuid, ActivityItem, ActivityKind, ActivityQuerying,
    ActivityReadOps, ActivityStateManagement, ActivityStatusKind, ActivityStorage,
    ActivityWriteOps, DeleteOptions, EndOptions, FilteredActivities, HoldOptions, KeywordOptions,
    ResumeOptions, SyncStorage, UpdateOptions,
};
use pace_error::{DatabaseStorageErrorKind, PaceOptResult, PaceResult};
use pace_time::{date::PaceDate, duration::PaceDurationRange, time_range::TimeRangeOptions};
use sea_query::{Expr, Query, SqliteQueryBuilder};
use tracing::debug;

use crate::{
    convert::Convert,
    entities::activities::{Activities, ActivitiesIden},
    migration::SQLiteMigrator,
    storage::SQLiteEntity,
};

#[derive(Debug)]
pub struct SqliteActivityStorage {
    connection: Connection,
}

impl SqliteActivityStorage {
    pub fn new(url: String) -> PaceResult<Self> {
        let connection = Connection::open(url.as_str())
            .map_err(|err| DatabaseStorageErrorKind::ConnectionFailed(url, err.to_string()))?;

        Ok(Self { connection })
    }
}

impl ActivityStorage for SqliteActivityStorage {
    fn setup(&self) -> PaceResult<()> {
        let mut migrate = SQLiteMigrator::new(&self.connection)?;

        migrate.up()?;

        Ok(())
    }

    fn teardown(&self) -> PaceResult<()> {
        // TODO: Do we need a teardown for sqlite?
        unimplemented!("teardown not yet implemented for sqlite storage")
    }

    fn identify(&self) -> String {
        "sqlite".to_string()
    }
}

impl SyncStorage for SqliteActivityStorage {
    fn sync(&self) -> PaceResult<()> {
        // We sync activities to the database in each operation
        // so we don't need to do anything here

        Ok(())
    }
}

impl ActivityReadOps for SqliteActivityStorage {
    #[tracing::instrument]
    fn read_activity(&self, activity_id: ActivityGuid) -> PaceResult<ActivityItem> {
        let query = Query::select()
            .from(ActivitiesIden::Table)
            .cond_where(Expr::col(ActivitiesIden::Guid).eq(activity_id.to_string()))
            .limit(1)
            .to_string(SqliteQueryBuilder);

        debug!("Read activity query: {query}");

        let mut stmt = self.connection.prepare(&query).map_err(|source| {
            DatabaseStorageErrorKind::ActivityReadFailed {
                guid: activity_id.to_string(),
                source,
            }
        })?;

        let activity_item_iter = stmt
            .query_map([&activity_id], |row| Ok(Activities::from_row(row)))
            .map_err(|source| DatabaseStorageErrorKind::ActivityReadFailed {
                guid: activity_id.to_string(),
                source,
            })?;

        let database_item = activity_item_iter
            .filter_map_ok(std::result::Result::ok)
            .next()
            .ok_or(DatabaseStorageErrorKind::NoItemContained(
                activity_id.to_string(),
            ))?
            .map_err(|source| DatabaseStorageErrorKind::ActivityNotFound {
                guid: activity_id.to_string(),
                source,
            })?;

        debug!("Read activity: {:?}", database_item);

        // TODO: Now we need to get the rest of the data from the database
        // and return the ActivityItem
        //
        // Missing data:
        // Description (1:1) (in extra table for deduplication)
        // ActivityStatus (1:N)
        // ActivityKind (1:N)
        // Categories (M:N)
        // Tags (M:N)

        Ok(ActivityItem::default())
    }

    #[tracing::instrument]
    fn list_activities(&self, filter: ActivityFilterKind) -> PaceOptResult<FilteredActivities> {
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
    fn create_activity(&self, _activity: Activity) -> PaceResult<ActivityItem> {
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
        _activity_id: ActivityGuid,
        _updated_activity: Activity,
        _update_opts: UpdateOptions,
    ) -> PaceResult<ActivityItem> {
        todo!()
    }

    fn delete_activity(
        &self,
        _activity_id: ActivityGuid,
        _delete_opts: DeleteOptions,
    ) -> PaceResult<ActivityItem> {
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
        _activity_id: ActivityGuid,
        _hold_opts: HoldOptions,
    ) -> PaceResult<ActivityItem> {
        todo!()
    }

    fn resume_activity(
        &self,
        _activity_id: ActivityGuid,
        _resume_opts: ResumeOptions,
    ) -> PaceResult<ActivityItem> {
        todo!()
    }

    fn resume_most_recent_activity(
        &self,
        _resume_opts: ResumeOptions,
    ) -> PaceOptResult<ActivityItem> {
        todo!()
    }

    fn end_activity(
        &self,
        _activity_id: ActivityGuid,
        _end_opts: EndOptions,
    ) -> PaceResult<ActivityItem> {
        todo!()
    }

    fn end_all_activities(&self, _end_opts: EndOptions) -> PaceOptResult<Vec<ActivityItem>> {
        todo!()
    }

    fn end_all_active_intermissions(
        &self,
        _end_opts: EndOptions,
    ) -> PaceOptResult<Vec<ActivityGuid>> {
        todo!()
    }

    fn end_last_unfinished_activity(&self, _end_opts: EndOptions) -> PaceOptResult<ActivityItem> {
        todo!()
    }

    fn hold_most_recent_active_activity(
        &self,
        _hold_opts: HoldOptions,
    ) -> PaceOptResult<ActivityItem> {
        todo!()
    }
}
impl ActivityQuerying for SqliteActivityStorage {
    fn group_activities_by_duration_range(
        &self,
    ) -> PaceOptResult<BTreeMap<PaceDurationRange, Vec<ActivityItem>>> {
        todo!()
    }

    fn group_activities_by_start_date(
        &self,
    ) -> PaceOptResult<BTreeMap<PaceDate, Vec<ActivityItem>>> {
        todo!()
    }

    fn list_activities_with_intermissions(
        &self,
    ) -> PaceOptResult<BTreeMap<ActivityGuid, Vec<ActivityItem>>> {
        todo!()
    }

    fn group_activities_by_keywords(
        &self,
        _keyword_opts: KeywordOptions,
    ) -> PaceOptResult<BTreeMap<String, Vec<ActivityItem>>> {
        todo!()
    }

    fn group_activities_by_kind(&self) -> PaceOptResult<BTreeMap<ActivityKind, Vec<ActivityItem>>> {
        todo!()
    }

    fn list_activities_by_time_range(
        &self,
        _time_range_opts: TimeRangeOptions,
    ) -> PaceOptResult<Vec<ActivityGuid>> {
        todo!()
    }

    fn group_activities_by_status(
        &self,
    ) -> PaceOptResult<BTreeMap<ActivityStatusKind, Vec<ActivityItem>>> {
        todo!()
    }

    fn list_activities_by_id(&self) -> PaceOptResult<BTreeMap<ActivityGuid, Activity>> {
        todo!()
    }
}
