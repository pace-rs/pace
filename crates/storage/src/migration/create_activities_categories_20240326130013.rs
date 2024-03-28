use sea_query::{ColumnDef, ForeignKey, SqliteQueryBuilder, Table};

use crate::{
    entities::{
        activities::ActivitiesIden, activities_categories::ActivitiesCategoriesIden,
        categories::CategoriesIden,
    },
    migration::SQLiteMigration,
};

pub struct Migration;

impl SQLiteMigration for Migration {
    fn version(&self) -> String {
        "20240326130013".to_string()
    }

    fn up(&self) -> String {
        Table::create()
            .table(ActivitiesCategoriesIden::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(ActivitiesCategoriesIden::Guid)
                    .text()
                    .not_null()
                    .primary_key(),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_activities_categories_category_guid")
                    .from(
                        ActivitiesCategoriesIden::Table,
                        ActivitiesCategoriesIden::CategoryGuid,
                    )
                    .to(CategoriesIden::Table, CategoriesIden::Guid),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_activities_categories_activity_guid")
                    .from(
                        ActivitiesCategoriesIden::Table,
                        ActivitiesCategoriesIden::ActivityGuid,
                    )
                    .to(ActivitiesIden::Table, ActivitiesIden::Guid),
            )
            .build(SqliteQueryBuilder)
    }

    fn down(&self) -> String {
        Table::drop()
            .table(ActivitiesCategoriesIden::Table)
            .build(SqliteQueryBuilder)
    }
}
