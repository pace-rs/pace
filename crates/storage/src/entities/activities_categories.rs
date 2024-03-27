use rusqlite::{Error, Row};
use sea_query::{
    enum_def, ColumnDef, Expr, ForeignKey, Func, Iden, Order, Query, SqliteQueryBuilder, Table,
};
use strum::EnumIter;

use crate::{
    entities::{activities::ActivitiesIden, categories::CategoriesIden},
    storage::SQLiteEntity,
};

#[derive(Clone, Debug, PartialEq, Eq)]
#[enum_def]
pub struct ActivitiesCategories {
    pub guid: String,
    pub category_guid: String,
    pub activity_guid: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Activities,
    Categories,
}

impl SQLiteEntity for ActivitiesCategories {
    fn from_row(row: Row<'_>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Self::try_from(row)
    }
}

impl TryFrom<Row<'_>> for ActivitiesCategories {
    type Error = Error;

    fn try_from(row: Row<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            guid: row.get("guid")?,
            category_guid: row.get("category_guid")?,
            activity_guid: row.get("activity_guid")?,
        })
    }
}
