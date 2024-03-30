use sea_orm_migration::prelude::*;

use crate::entity::activity_status::ActivityStatusEnum;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &'life1 SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .if_not_exists()
                    .table(ActivityStatusEnum::Table)
                    .col(
                        ColumnDef::new(ActivityStatusEnum::Guid)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ActivityStatusEnum::Status).text().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &'life1 SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ActivityStatusEnum::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
