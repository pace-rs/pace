use std::{collections::HashSet, str::FromStr};

use pace_time::date_time::PaceDateTime;
use rusqlite::{types::FromSql, Row, ToSql};
use ulid::Ulid;

// use pace_time::rusqlite::*;

use pace_core::prelude::{
    Activity, ActivityEndOptions, ActivityFilterKind, ActivityGuid, ActivityItem, ActivityKind,
    ActivityKindOptions, ActivityStatusKind, PaceResult,
};

use crate::sqlite::FromRow;

pub struct SqliteActivity(Activity);

impl std::ops::Deref for SqliteActivity {
    type Target = Activity;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
pub struct SqliteActivityFilterKind(ActivityFilterKind);

impl std::ops::Deref for SqliteActivityFilterKind {
    type Target = ActivityFilterKind;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct SqliteActivityGuid(ActivityGuid);

impl std::ops::Deref for SqliteActivityGuid {
    type Target = ActivityGuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
pub struct SqliteActivityKind(ActivityKind);

impl std::ops::Deref for SqliteActivityKind {
    type Target = ActivityKind;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
pub struct SqliteActivityStatusKind(ActivityStatusKind);

impl std::ops::Deref for SqliteActivityStatusKind {
    type Target = ActivityStatusKind;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl SqliteActivity {
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

impl SqliteActivityFilterKind {
    pub fn to_sql_statement(&self) -> &'static str {
        match self {
            Self::Everything => "SELECT * FROM activities",
            ActivityFilterKind::OnlyActivities => todo!(),
            ActivityFilterKind::Active => "SELECT * FROM activities WHERE status = 'in-progress'",
            ActivityFilterKind::ActiveIntermission => todo!(),
            ActivityFilterKind::Archived => "SELECT * FROM activities WHERE status = 'archived'",
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

impl ToSql for SqliteActivityGuid {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::Owned(
            rusqlite::types::Value::Text(self.to_string()),
        ))
    }
}

impl FromSql for SqliteActivityGuid {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(ActivityGuid::with_id(
            Ulid::from_string(value.as_str()?)
                .map_err(|err| rusqlite::types::FromSqlError::Other(Box::new(err)))?,
        ))
    }
}

impl ToSql for SqliteActivityKind {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::Owned(
            rusqlite::types::Value::Text(self.to_string()),
        ))
    }
}

impl FromSql for SqliteActivityKind {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(ActivityKind::from_str(value.as_str()?)
            .map_err(|err| rusqlite::types::FromSqlError::Other(Box::new(err)))?)
    }
}

impl ToSql for SqliteActivityStatusKind {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::Owned(
            rusqlite::types::Value::Text(self.to_string()),
        ))
    }
}

impl FromSql for SqliteActivityStatusKind {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(ActivityStatusKind::from_str(value.as_str()?)
            .map_err(|err| rusqlite::types::FromSqlError::Other(Box::new(err)))?)
    }
}
