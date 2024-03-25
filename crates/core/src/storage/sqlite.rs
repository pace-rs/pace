use std::collections::BTreeMap;

use itertools::Itertools;
use rusqlite::Connection;

use pace_time::{date::PaceDate, duration::PaceDurationRange, time_range::TimeRangeOptions};
use tracing::debug;

use crate::{
    domain::activity::Activity,
    error::{DatabaseErrorKind, PaceResult},
    prelude::{
        ActivityFilterKind, ActivityGuid, ActivityItem, ActivityKind, ActivityStatusKind,
        DeleteOptions, EndOptions, FilteredActivities, HoldOptions, KeywordOptions, PaceOptResult,
        ResumeOptions, UpdateOptions,
    },
    storage::{
        ActivityQuerying, ActivityReadOps, ActivityStateManagement, ActivityStorage,
        ActivityWriteOps, SyncStorage,
    },
};

pub trait FromRow {
    fn from_row(row: &rusqlite::Row<'_>) -> PaceResult<Self>
    where
        Self: Sized;
}

#[derive(Debug)]
pub struct SqliteActivityStorage {
    connection: Connection,
}

impl SqliteActivityStorage {
    pub fn new(connection_string: String) -> PaceResult<Self> {
        let connection = Connection::open(connection_string.as_str()).map_err(|err| {
            DatabaseErrorKind::ConnectionFailed(connection_string, err.to_string())
        })?;

        Ok(Self { connection })
    }
}

impl ActivityStorage for SqliteActivityStorage {
    fn setup_storage(&self) -> PaceResult<()> {
        // TODO!: Check if the needed tables are existing or if we
        // are dealing with a fresh database, so we need to create
        // the tables.

        Ok(())
    }
}

impl SyncStorage for SqliteActivityStorage {
    fn sync(&self) -> PaceResult<()> {
        // TODO!: Sync the activities to the database

        Ok(())
    }
}

impl ActivityReadOps for SqliteActivityStorage {
    #[tracing::instrument]
    fn read_activity(&self, activity_id: ActivityGuid) -> PaceResult<ActivityItem> {
        let mut stmt = self
            .connection
            .prepare("SELECT * FROM activities WHERE id = ?1")?;

        let activity_item_iter =
            stmt.query_map(&[&activity_id], |row| Ok(ActivityItem::from_row(&row)))?;

        let activity_item = activity_item_iter
            .filter_map_ok(|item| item.ok())
            .next()
            .transpose()?
            .ok_or(DatabaseErrorKind::ActivityNotFound(activity_id))?;

        debug!("Read activity: {:?}", activity_item);

        Ok(activity_item)
    }

    #[tracing::instrument]
    fn list_activities(&self, filter: ActivityFilterKind) -> PaceOptResult<FilteredActivities> {
        let mut stmt = self.connection.prepare(filter.to_sql_statement())?;

        let activity_item_iter = stmt.query_map([], |row| Ok(ActivityGuid::from_row(&row)))?;

        let activities = activity_item_iter
            .filter_map_ok(|item| item.ok())
            .collect::<Result<Vec<ActivityGuid>, _>>()?;

        debug!("Listed activities: {:?}", activities);

        if activities.is_empty() {
            return Ok(None);
        }

        let activities = match filter {
            ActivityFilterKind::Everything => FilteredActivities::Everything(activities),
            ActivityFilterKind::OnlyActivities => FilteredActivities::OnlyActivities(activities),
            ActivityFilterKind::Active => FilteredActivities::Active(activities),
            ActivityFilterKind::ActiveIntermission => {
                FilteredActivities::ActiveIntermission(activities)
            }
            ActivityFilterKind::Archived => FilteredActivities::Archived(activities),
            ActivityFilterKind::Ended => FilteredActivities::Ended(activities),
            ActivityFilterKind::Held => FilteredActivities::Held(activities),
            ActivityFilterKind::Intermission => FilteredActivities::Intermission(activities),
            ActivityFilterKind::TimeRange(_) => FilteredActivities::TimeRange(activities),
        };

        Ok(Some(activities))
    }
}

impl ActivityWriteOps for SqliteActivityStorage {
    fn create_activity(&self, activity: Activity) -> PaceResult<ActivityItem> {
        let tx = self.connection.transaction()?;

        let mut stmt = tx.prepare(activity.to_sql_prepare_statement())?;

        let (guid, params) = activity.to_sql_execute_statement()?;

        if stmt.execute(params.as_slice())? > 0 {
            tx.commit()?;
            return Ok(ActivityItem::from((guid, activity)));
        }

        return Err(DatabaseErrorKind::ActivityCreationFailed(activity).into());
    }

    fn update_activity(
        &self,
        activity_id: ActivityGuid,
        updated_activity: Activity,
        update_opts: UpdateOptions,
    ) -> PaceResult<ActivityItem> {
        todo!()
    }

    fn delete_activity(
        &self,
        activity_id: ActivityGuid,
        delete_opts: DeleteOptions,
    ) -> PaceResult<ActivityItem> {
        let activity = self.read_activity(activity_id)?;

        let tx = self.connection.transaction()?;
        let mut stmt = tx.prepare("DELETE FROM activities WHERE id = ?1 LIMIT = 1")?;

        if stmt.execute(&[&activity_id])? == 1 {
            tx.commit()?;
            return Ok(activity);
        }

        Err(DatabaseErrorKind::ActivityDeletionFailed(activity_id).into())
    }
}
impl ActivityStateManagement for SqliteActivityStorage {
    fn hold_activity(
        &self,
        activity_id: ActivityGuid,
        hold_opts: HoldOptions,
    ) -> PaceResult<ActivityItem> {
        todo!()
    }

    fn resume_activity(
        &self,
        activity_id: ActivityGuid,
        resume_opts: ResumeOptions,
    ) -> PaceResult<ActivityItem> {
        todo!()
    }

    fn resume_most_recent_activity(
        &self,
        resume_opts: ResumeOptions,
    ) -> PaceOptResult<ActivityItem> {
        todo!()
    }

    fn end_activity(
        &self,
        activity_id: ActivityGuid,
        end_opts: EndOptions,
    ) -> PaceResult<ActivityItem> {
        todo!()
    }

    fn end_all_activities(&self, end_opts: EndOptions) -> PaceOptResult<Vec<ActivityItem>> {
        todo!()
    }

    fn end_all_active_intermissions(
        &self,
        end_opts: EndOptions,
    ) -> PaceOptResult<Vec<ActivityGuid>> {
        todo!()
    }

    fn end_last_unfinished_activity(&self, end_opts: EndOptions) -> PaceOptResult<ActivityItem> {
        todo!()
    }

    fn hold_most_recent_active_activity(
        &self,
        hold_opts: HoldOptions,
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
        keyword_opts: KeywordOptions,
    ) -> PaceOptResult<BTreeMap<String, Vec<ActivityItem>>> {
        todo!()
    }

    fn group_activities_by_kind(&self) -> PaceOptResult<BTreeMap<ActivityKind, Vec<ActivityItem>>> {
        todo!()
    }

    fn list_activities_by_time_range(
        &self,
        time_range_opts: TimeRangeOptions,
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

pub mod sql_conversion {
    use std::{collections::HashSet, str::FromStr};

    use pace_time::date_time::PaceDateTime;
    use rusqlite::{types::FromSql, Row, ToSql};
    use ulid::Ulid;

    // use pace_time::rusqlite::*;

    use crate::{
        prelude::{
            Activity, ActivityEndOptions, ActivityFilterKind, ActivityGuid, ActivityItem,
            ActivityKind, ActivityKindOptions, ActivityStatusKind, PaceResult,
        },
        storage::sqlite::FromRow,
    };

    impl Activity {
        pub fn to_sql_prepare_statement(&self) -> &'static str {
            "INSERT INTO activities (id, category, description, begin, end, duration, kind, status, tags, parent_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
        }

        pub fn to_sql_execute_statement(&self) -> PaceResult<(ActivityGuid, Vec<&dyn ToSql>)> {
            let category = if let Some(category) = self.category() {
                category.to_sql()?
            } else {
                "NULL".to_sql()?
            };

            let (end, duration) = if let Some(end_opts) = self.activity_end_options().as_ref() {
                let (end, duration) = end_opts.as_tuple();
                (end.to_sql()?, duration.to_sql()?)
            } else {
                ("NULL".to_sql()?, "NULL".to_sql()?)
            };

            let parent_id = if let Some(parent_id) = self.parent_id() {
                parent_id.to_sql()?
            } else {
                "NULL".to_sql()?
            };

            let tags = if let Some(tags) = self.tags() {
                let tags = tags
                    .iter()
                    .map(|tag| tag.to_string())
                    .collect::<Vec<String>>();

                tags.join(",").to_sql()?
            } else {
                "NULL".to_sql()?
            };

            let guid = ActivityGuid::new();

            Ok((
                guid,
                vec![
                    // TODO: We create a new ID here, that should probably happen
                    // TODO: somewhere else and needs a refactoring
                    &guid,
                    &category,
                    &self.description(),
                    &self.begin(),
                    &end,
                    &duration,
                    &self.kind(),
                    &self.status(),
                    &tags,
                    &parent_id,
                ],
            ))
        }
    }

    impl ActivityFilterKind {
        pub fn to_sql_statement(&self) -> &'static str {
            match self {
                Self::Everything => "SELECT * FROM activities",
                ActivityFilterKind::OnlyActivities => todo!(),
                ActivityFilterKind::Active => {
                    "SELECT * FROM activities WHERE status = 'in-progress'"
                }
                ActivityFilterKind::ActiveIntermission => todo!(),
                ActivityFilterKind::Archived => {
                    "SELECT * FROM activities WHERE status = 'archived'"
                }
                ActivityFilterKind::Ended => "SELECT * FROM activities WHERE status = 'completed'",
                ActivityFilterKind::Held => "SELECT * FROM activities WHERE status = 'paused'",
                ActivityFilterKind::Intermission => todo!(),
                ActivityFilterKind::TimeRange(opts) => todo!(),
            }
        }
    }

    impl FromRow for ActivityEndOptions {
        fn from_row(row: &Row<'_>) -> PaceResult<Self> {
            Ok(Self::new(row.get("end")?, row.get("duration")?))
        }
    }

    impl FromRow for ActivityKindOptions {
        fn from_row(row: &Row<'_>) -> PaceResult<Self> {
            Ok(Self::with_parent_id(row.get("parent_id")?))
        }
    }

    impl FromRow for Activity {
        fn from_row(row: &Row<'_>) -> PaceResult<Self> {
            let begin_time: PaceDateTime = row.get("begin")?;

            let description: String = row.get("description")?;

            let tags_string: String = row.get("tags")?;

            let tags = tags_string
                .split(',')
                .map(|tag| tag.to_string())
                .collect::<HashSet<String>>();

            Ok(Activity::builder()
                .category(Some(row.get("category")?)) // TODO: Check for None
                .description(description)
                .begin(begin_time)
                .activity_end_options(Some(ActivityEndOptions::from_row(row)?)) // TODO: Check for None
                .kind(row.get("kind")?)
                .activity_kind_options(Some(ActivityKindOptions::from_row(row)?)) // TODO: Check for None
                .tags(tags)
                .status(row.get("status")?)
                .build())
        }
    }

    impl FromRow for ActivityGuid {
        fn from_row(row: &Row<'_>) -> PaceResult<Self> {
            Ok(row.get("guid")?)
        }
    }

    impl FromRow for ActivityItem {
        fn from_row(row: &Row<'_>) -> PaceResult<Self> {
            let activity_end_opts = ActivityEndOptions::from_row(row)?;

            let activity_kind_opts = ActivityKindOptions::from_row(row)?;

            let activity = Activity::from_row(row)?;

            let guid = ActivityGuid::from_row(row)?;

            Ok(Self::builder().guid(guid).activity(activity).build())
        }
    }

    impl ToSql for ActivityGuid {
        fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
            Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Text(self.to_string()),
            ))
        }
    }

    impl FromSql for ActivityGuid {
        fn column_result(
            value: rusqlite::types::ValueRef<'_>,
        ) -> rusqlite::types::FromSqlResult<Self> {
            Ok(ActivityGuid::with_id(
                Ulid::from_string(value.as_str()?)
                    .map_err(|err| rusqlite::types::FromSqlError::Other(Box::new(err)))?,
            ))
        }
    }

    impl ToSql for ActivityKind {
        fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
            Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Text(self.to_string()),
            ))
        }
    }

    impl FromSql for ActivityKind {
        fn column_result(
            value: rusqlite::types::ValueRef<'_>,
        ) -> rusqlite::types::FromSqlResult<Self> {
            Ok(ActivityKind::from_str(value.as_str()?)
                .map_err(|err| rusqlite::types::FromSqlError::Other(Box::new(err)))?)
        }
    }

    impl ToSql for ActivityStatusKind {
        fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
            Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Text(self.to_string()),
            ))
        }
    }

    impl FromSql for ActivityStatusKind {
        fn column_result(
            value: rusqlite::types::ValueRef<'_>,
        ) -> rusqlite::types::FromSqlResult<Self> {
            Ok(ActivityStatusKind::from_str(value.as_str()?)
                .map_err(|err| rusqlite::types::FromSqlError::Other(Box::new(err)))?)
        }
    }
}
