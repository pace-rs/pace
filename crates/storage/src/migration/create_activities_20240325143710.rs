use sea_query::{ColumnDef, ForeignKey, SqliteQueryBuilder, Table};

use crate::{
    entities::{
        activities::ActivitiesIden, activity_kinds::ActivityKindsIden,
        activity_status::ActivityStatusIden, description::DescriptionsIden,
    },
    migration::SQLiteMigration,
};

pub struct Migration;

impl SQLiteMigration for Migration {
    fn version(&self) -> String {
        "20240325143710".to_string()
    }

    fn up(&self) -> String {
        Table::create()
            .table(ActivitiesIden::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(ActivitiesIden::Guid)
                    .text()
                    .not_null()
                    .primary_key(),
            )
            .col(
                ColumnDef::new(ActivitiesIden::DescriptionGuid)
                    .text()
                    .not_null(),
            )
            .col(
                ColumnDef::new(ActivitiesIden::Begin)
                    .timestamp_with_time_zone()
                    .not_null(),
            )
            .col(
                ColumnDef::new(ActivitiesIden::End)
                    .timestamp_with_time_zone()
                    .null(),
            )
            .col(ColumnDef::new(ActivitiesIden::Duration).integer().null())
            .col(ColumnDef::new(ActivitiesIden::KindGuid).text().not_null())
            .col(ColumnDef::new(ActivitiesIden::StatusGuid).text().not_null())
            .col(ColumnDef::new(ActivitiesIden::ParentGuid).text().null())
            .foreign_key(
                ForeignKey::create()
                    .name("fk_activities_parent_guid")
                    .from(ActivitiesIden::Table, ActivitiesIden::ParentGuid)
                    .to(ActivitiesIden::Table, ActivitiesIden::Guid),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_activities_kind")
                    .from(ActivitiesIden::Table, ActivitiesIden::KindGuid)
                    .to(ActivityKindsIden::Table, ActivityKindsIden::Guid),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_activities_status")
                    .from(ActivitiesIden::Table, ActivitiesIden::StatusGuid)
                    .to(ActivityStatusIden::Table, ActivityStatusIden::Guid),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_activities_description")
                    .from(ActivitiesIden::Table, ActivitiesIden::DescriptionGuid)
                    .to(DescriptionsIden::Table, DescriptionsIden::Guid),
            )
            .build(SqliteQueryBuilder)
    }

    fn down(&self) -> String {
        Table::drop()
            .table(ActivitiesIden::Table)
            .if_exists()
            .build(SqliteQueryBuilder)
    }
}
