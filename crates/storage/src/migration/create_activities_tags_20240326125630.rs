use sea_query::{ColumnDef, ForeignKey, SqliteQueryBuilder, Table};

use crate::{
    entities::{activities::ActivitiesIden, activities_tags::ActivitiesTagsIden, tags::TagsIden},
    migration::SQLiteMigration,
};

pub struct Migration;

impl SQLiteMigration for Migration {
    fn version(&self) -> String {
        "20240326125630".to_string()
    }

    fn up(&self) -> String {
        Table::create()
            .if_not_exists()
            .table(ActivitiesTagsIden::Table)
            .col(
                ColumnDef::new(ActivitiesTagsIden::Guid)
                    .text()
                    .not_null()
                    .primary_key(),
            )
            .col(
                ColumnDef::new(ActivitiesTagsIden::TagGuid)
                    .text()
                    .not_null(),
            )
            .col(
                ColumnDef::new(ActivitiesTagsIden::ActivityGuid)
                    .text()
                    .not_null(),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_activities_tags_tag_guid")
                    .from(ActivitiesTagsIden::Table, ActivitiesTagsIden::TagGuid)
                    .to(TagsIden::Table, TagsIden::Guid),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_activities_tags_activity_guid")
                    .from(ActivitiesTagsIden::Table, ActivitiesTagsIden::ActivityGuid)
                    .to(ActivitiesIden::Table, ActivitiesIden::Guid),
            )
            .build(SqliteQueryBuilder)
    }

    fn down(&self) -> String {
        Table::drop()
            .table(ActivitiesTagsIden::Table)
            .build(SqliteQueryBuilder)
    }
}
