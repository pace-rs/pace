use sea_orm_migration::prelude::*;

use crate::entity::activities::ActivitiesEnum;
use crate::entity::activity_kinds::ActivityKindsEnum;
use crate::entity::activity_status::ActivityStatusEnum;
use crate::entity::descriptions::DescriptionsEnum;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ActivitiesEnum::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ActivitiesEnum::Guid)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ActivitiesEnum::DescriptionGuid)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ActivitiesEnum::Begin)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ActivitiesEnum::End)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(ColumnDef::new(ActivitiesEnum::Duration).integer().null())
                    .col(ColumnDef::new(ActivitiesEnum::KindGuid).text().not_null())
                    .col(ColumnDef::new(ActivitiesEnum::StatusGuid).text().not_null())
                    .col(ColumnDef::new(ActivitiesEnum::ParentGuid).text().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_activities_parent_guid")
                            .from(ActivitiesEnum::Table, ActivitiesEnum::ParentGuid)
                            .to(ActivitiesEnum::Table, ActivitiesEnum::Guid),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_activities_kind")
                            .from(ActivitiesEnum::Table, ActivitiesEnum::KindGuid)
                            .to(ActivityKindsEnum::Table, ActivityKindsEnum::Guid),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_activities_status")
                            .from(ActivitiesEnum::Table, ActivitiesEnum::StatusGuid)
                            .to(ActivityStatusEnum::Table, ActivityStatusEnum::Guid),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_activities_description")
                            .from(ActivitiesEnum::Table, ActivitiesEnum::DescriptionGuid)
                            .to(DescriptionsEnum::Table, DescriptionsEnum::Guid),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ActivitiesEnum::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
