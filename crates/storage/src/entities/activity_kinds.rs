use getset::Getters;
use pace_core::prelude::ActivityKindGuid;
use rusqlite::{Error, Row};
use sea_query::enum_def;
use strum::EnumIter;
use typed_builder::TypedBuilder;

use crate::storage::SQLiteEntity;

#[derive(Clone, Debug, PartialEq, Eq, TypedBuilder, Getters)]
#[getset(get = "pub")]
#[enum_def]
pub struct ActivityKinds {
    pub guid: ActivityKindGuid,
    pub kind: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Activities,
}

impl SQLiteEntity for ActivityKinds {
    fn from_row(row: &Row<'_>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Self::try_from(row)
    }
}

impl TryFrom<&Row<'_>> for ActivityKinds {
    type Error = Error;

    fn try_from(row: &Row<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            guid: row.get("guid")?,
            kind: row.get("kind")?,
        })
    }
}
