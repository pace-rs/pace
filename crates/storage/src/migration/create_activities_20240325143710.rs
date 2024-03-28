use sea_query::{ColumnDef, ForeignKey, SqliteQueryBuilder, Table};

use crate::{
    entities::{
        activities::ActivitiesIden, activity_kinds::ActivityKindsIden,
        activity_status::ActivityStatusIden,
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
            .col(ColumnDef::new(ActivitiesIden::Category).text().not_null())
            .col(
                ColumnDef::new(ActivitiesIden::Description)
                    .text()
                    .not_null(),
            )
            .col(ColumnDef::new(ActivitiesIden::Begin).date_time().not_null())
            .col(ColumnDef::new(ActivitiesIden::End).text().null())
            .col(ColumnDef::new(ActivitiesIden::Duration).integer().null())
            .col(ColumnDef::new(ActivitiesIden::Kind).text().not_null())
            .col(ColumnDef::new(ActivitiesIden::Status).text().not_null())
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
                    .from(ActivitiesIden::Table, ActivitiesIden::Kind)
                    .to(ActivityKindsIden::Table, ActivityKindsIden::Guid),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_activities_status")
                    .from(ActivitiesIden::Table, ActivitiesIden::Status)
                    .to(ActivityStatusIden::Table, ActivityStatusIden::Guid),
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
