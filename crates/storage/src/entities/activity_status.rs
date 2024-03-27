use rusqlite::{Error, Row};
use sea_query::{enum_def, ColumnDef, Expr, Func, Iden, Order, Query, SqliteQueryBuilder, Table};
use strum::EnumIter;

use crate::storage::SQLiteEntity;

#[derive(Clone, Debug, PartialEq, Eq)]
#[enum_def]
pub struct ActivityStatus {
    pub guid: String,
    pub status: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Activities,
}

impl SQLiteEntity for ActivityStatus {
    fn from_row(row: Row<'_>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Self::try_from(row)
    }
}

impl TryFrom<Row<'_>> for ActivityStatus {
    type Error = Error;

    fn try_from(row: Row<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            guid: row.get("guid")?,
            status: row.get("status")?,
        })
    }
}
