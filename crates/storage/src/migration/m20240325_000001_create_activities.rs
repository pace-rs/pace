use sea_orm_migration::prelude::*;

use crate::entity::activities::Activities;
use crate::entity::activity_kinds::ActivityKinds;
use crate::entity::activity_status::ActivityStatus;
use crate::entity::descriptions::Descriptions;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Activities::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Activities::Guid)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Activities::DescriptionGuid)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Activities::Begin)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Activities::End)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(ColumnDef::new(Activities::Duration).integer().null())
                    .col(ColumnDef::new(Activities::KindGuid).text().not_null())
                    .col(ColumnDef::new(Activities::StatusGuid).text().not_null())
                    .col(ColumnDef::new(Activities::ParentGuid).text().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_activities_parent_guid")
                            .from(Activities::Table, Activities::ParentGuid)
                            .to(Activities::Table, Activities::Guid),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_activities_kind")
                            .from(Activities::Table, Activities::KindGuid)
                            .to(ActivityKinds::Table, ActivityKinds::Guid),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_activities_status")
                            .from(Activities::Table, Activities::StatusGuid)
                            .to(ActivityStatus::Table, ActivityStatus::Guid),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_activities_description")
                            .from(Activities::Table, Activities::DescriptionGuid)
                            .to(Descriptions::Table, Descriptions::Guid),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(Activities::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
