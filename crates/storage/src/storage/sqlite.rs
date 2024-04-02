use std::{collections::BTreeMap, fs::File, path::PathBuf};

// use itertools::Itertools;
use sea_orm::{Database, DatabaseConnection, EntityTrait, ModelTrait};
use tracing::debug;

use pace_core::{
    config::DatabaseEngineKind,
    options::{
        DeleteOptions, EndOptions, HoldOptions, KeywordOptions, ResumeOptions, UpdateOptions,
    },
    prelude::{
        Activity, ActivityFilterKind, ActivityGuid, ActivityItem, ActivityKind, ActivityQuerying,
        ActivityReadOps, ActivityStateManagement, ActivityStatusKind, ActivityStorage,
        ActivityWriteOps, FilteredActivities, SyncStorage,
    },
};
use pace_error::{DatabaseStorageErrorKind, PaceOptResult, PaceResult};
use pace_time::{date::PaceDate, duration::PaceDurationRange, time_range::TimeRangeOptions};

use crate::{
    entity::prelude::Activities,
    migration::{Migrator, MigratorTrait},
    runtime,
};

#[derive(Debug)]
pub struct DatabaseActivityStorage {
    connection: DatabaseConnection,
}

impl DatabaseActivityStorage {
    /// Create a new database activity storage instance.
    ///
    /// # Arguments
    ///
    /// * `kind` - The database engine kind.
    /// * `url` - The database connection URL.
    ///
    /// # Errors
    ///
    /// This function returns an error if the database connection engine is not supported.
    ///
    /// # Panics
    ///
    /// This function panics if the database connection fails as its a critical operation.
    ///
    /// # Returns
    ///
    /// A new database activity storage instance.
    #[allow(clippy::expect_used)]
    pub fn new(kind: DatabaseEngineKind, url: &str) -> PaceResult<Self> {
        let connection_string = match kind {
            DatabaseEngineKind::Sqlite => {
                debug!("Connecting to SQLite database: {url}");

                let path = PathBuf::from(&url);

                if !path.exists() {
                    _ = File::create(&path)?;
                }

                format!("sqlite://{url}")
            }
            engine => {
                return Err(
                    DatabaseStorageErrorKind::UnsupportedDatabaseEngine(engine.to_string()).into(),
                )
            }
        };

        runtime().block_on(async {
            let connection = Database::connect(connection_string)
                .await
                .expect("Failed to connect to the database");

            Ok(Self { connection })
        })
    }
}

impl ActivityStorage for DatabaseActivityStorage {
    fn setup(&self) -> PaceResult<()> {
        runtime().block_on(async {
            Migrator::up(&self.connection, None)
                .await
                .map_err(|source| DatabaseStorageErrorKind::MigrationFailed { source })?;

            Ok(())
        })
    }

    fn teardown(&self) -> PaceResult<()> {
        // TODO: Do we need a teardown for sqlite?
        unimplemented!("teardown not yet implemented for sqlite storage")
    }

    fn identify(&self) -> String {
        "sqlite".to_string()
    }
}

impl SyncStorage for DatabaseActivityStorage {
    fn sync(&self) -> PaceResult<()> {
        // We sync activities to the database in each operation
        // so we don't need to do anything here

        Ok(())
    }
}

impl ActivityReadOps for DatabaseActivityStorage {
    #[tracing::instrument]
    fn read_activity(&self, activity_id: ActivityGuid) -> PaceResult<ActivityItem> {
        runtime().block_on(async {
            let Ok(Some(activity)) = Activities::find_by_id(activity_id.to_string())
                .one(&self.connection)
                .await
            else {
                return Err(DatabaseStorageErrorKind::ActivityNotFound {
                    guid: activity_id.to_string(),
                }
                .into());
            };

            // let _description = activity
            //     .find_related(descriptions::Entity)
            //     .one(&self.connection)
            //     .await;

            todo!("implement read_activity for sqlite");

            // Ok(ActivityItem::default())
        })
    }

    #[tracing::instrument]
    fn list_activities(&self, filter: ActivityFilterKind) -> PaceOptResult<FilteredActivities> {
        // let mut stmt = self.connection.prepare(filter.to_sql_statement())?;

        // let activity_item_iter = stmt.query_map([], |row| Ok(ActivityGuid::from_row(&row)))?;

        // let activities = activity_item_iter
        //     .filter_map_ok(|item| item.ok())
        //     .collect::<Result<Vec<ActivityGuid>, _>>()?;

        // debug!("Listed activities: {activities:?}");

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

impl ActivityWriteOps for DatabaseActivityStorage {
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
impl ActivityStateManagement for DatabaseActivityStorage {
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

impl ActivityQuerying for DatabaseActivityStorage {
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
