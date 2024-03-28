use getset::Getters;
use rusqlite::{Error, Row};
use sea_query::enum_def;
use strum::EnumIter;
use typed_builder::TypedBuilder;

use crate::storage::SQLiteEntity;

#[derive(Clone, Debug, PartialEq, Eq, TypedBuilder, Getters)]
#[getset(get = "pub")]
#[enum_def]
pub struct ActivitiesTags {
    pub guid: String,
    pub tag_guid: String,
    pub activity_guid: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Activities,
    Tags,
}

impl SQLiteEntity for ActivitiesTags {
    fn from_row(row: Row<'_>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Self::try_from(row)
    }
}

impl TryFrom<Row<'_>> for ActivitiesTags {
    type Error = Error;

    fn try_from(row: Row<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            guid: row.get("guid")?,
            tag_guid: row.get("tag_guid")?,
            activity_guid: row.get("activity_guid")?,
        })
    }
}
