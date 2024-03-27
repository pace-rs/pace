use rusqlite::{Error, Row};
use sea_query::{enum_def, ColumnDef, Expr, Func, Iden, Order, Query, SqliteQueryBuilder, Table};
use strum::EnumIter;

use crate::storage::SQLiteEntity;

#[derive(Clone, Debug, PartialEq, Eq)]
#[enum_def]
pub struct ActivityKinds {
    pub guid: String,
    pub kind: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Activities,
}

impl SQLiteEntity for ActivityKinds {
    fn from_row(row: Row<'_>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Self::try_from(row)
    }
}

impl TryFrom<Row<'_>> for ActivityKinds {
    type Error = Error;

    fn try_from(row: Row<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            guid: row.get("guid")?,
            kind: row.get("kind")?,
        })
    }
}
