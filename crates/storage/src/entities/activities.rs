use chrono::FixedOffset;
use getset::Getters;
use pace_core::prelude::{ActivityGuid, ActivityItem, Guid};
use rusqlite::{Error, Row};
use sea_query::enum_def;
use strum::EnumIter;
use typed_builder::TypedBuilder;

use crate::storage::SQLiteEntity;

#[derive(Clone, Debug, PartialEq, Eq, TypedBuilder, Getters)]
#[getset(get = "pub")]
#[enum_def]
pub struct Activities {
    pub guid: ActivityGuid,
    pub description_guid: Guid,
    pub begin: chrono::DateTime<FixedOffset>,
    pub end: Option<chrono::DateTime<FixedOffset>>,
    pub kind_guid: Guid,
    pub duration: Option<i32>,
    pub status_guid: Guid,
    pub parent_guid: Option<Guid>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    SelfRef,
    ActivitiesCategories,
    ActivitiesTags,
    ActivityKinds,
    ActivityStatus,
    Descriptions,
}

impl SQLiteEntity for Activities {
    fn from_row(row: &Row<'_>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Self::try_from(row)
    }
}

impl TryFrom<&Row<'_>> for Activities {
    type Error = Error;

    fn try_from(row: &Row<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            guid: row.get("guid")?,
            description_guid: row.get("description")?,
            begin: row.get("begin")?,
            end: row.get("end")?,
            duration: row.get("duration")?,
            kind_guid: row.get("kind")?,
            status_guid: row.get("status")?,
            parent_guid: row.get("parent_guid")?,
        })
    }
}
