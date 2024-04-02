use sea_orm_migration::prelude::*;

use crate::entity::activities::Activities;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &'life1 SchemaManager<'_>) -> Result<(), DbErr> {
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
                    .col(ColumnDef::new(Activities::Description).text().not_null())
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
                    .col(ColumnDef::new(Activities::Kind).integer().not_null())
                    .col(ColumnDef::new(Activities::Status).integer().not_null())
                    .col(ColumnDef::new(Activities::ParentGuid).text().null())
                    .col(ColumnDef::new(Activities::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(Activities::UpdatedAt).timestamp().null())
                    .col(ColumnDef::new(Activities::DeletedAt).timestamp().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_activities_parent_guid")
                            .from(Activities::Table, Activities::ParentGuid)
                            .to(Activities::Table, Activities::Guid),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(Activities::Table)
                    .name("idx_activities_parent_guid")
                    .col(Activities::ParentGuid)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(Activities::Table)
                    .name("idx_activities_description")
                    .col(Activities::Description)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &'life1 SchemaManager<'_>) -> Result<(), DbErr> {
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
