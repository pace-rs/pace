use sea_orm_migration::prelude::*;

use crate::entity::descriptions::Descriptions;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .if_not_exists()
                    .table(Descriptions::Table)
                    .col(
                        ColumnDef::new(Descriptions::Guid)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Descriptions::Description).text().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(Descriptions::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
