use rusqlite::{Error, Row};
use sea_query::{
    enum_def, ColumnDef, Expr, ForeignKey, Func, Iden, Order, Query, SqliteQueryBuilder, Table,
};
use strum::EnumIter;

use crate::{
    entities::{activity_kinds::ActivityKindsIden, activity_status::ActivityStatusIden},
    storage::SQLiteEntity,
};

#[derive(Clone, Debug, PartialEq, Eq)]
#[enum_def]
pub struct Activities {
    pub guid: String,
    pub category: String,
    pub description: String,
    pub begin: String,
    pub end: Option<String>,
    pub duration: Option<i32>,
    pub kind: String,
    pub status: String,
    pub parent_guid: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    SelfRef,
    ActivitiesCategories,
    ActivitiesTags,
    ActivityKinds,
    ActivityStatus,
}

impl SQLiteEntity for Activities {
    fn from_row(row: Row<'_>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Self::try_from(row)
    }
}

impl TryFrom<Row<'_>> for Activities {
    type Error = Error;

    fn try_from(row: Row<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            guid: row.get("guid")?,
            category: row.get("category")?,
            description: row.get("description")?,
            begin: row.get("begin")?,
            end: row.get("end")?,
            duration: row.get("duration")?,
            kind: row.get("kind")?,
            status: row.get("status")?,
            parent_guid: row.get("parent_guid")?,
        })
    }
}
