use getset::Getters;
use rusqlite::{Error, Row};
use sea_query::enum_def;
use typed_builder::TypedBuilder;

use crate::storage::SQLiteEntity;

#[derive(Clone, Debug, PartialEq, Eq, TypedBuilder, Getters)]
#[getset(get = "pub")]
#[enum_def]
pub struct SchemaMigrations {
    pub guid: String,
    pub version: i64,
}

impl SQLiteEntity for SchemaMigrations {
    fn from_row(row: Row<'_>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Self::try_from(row)
    }
}

impl TryFrom<Row<'_>> for SchemaMigrations {
    type Error = Error;

    fn try_from(row: Row<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            guid: row.get("guid")?,
            version: row.get("version")?,
        })
    }
}
