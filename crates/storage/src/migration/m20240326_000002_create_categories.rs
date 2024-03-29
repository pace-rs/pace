use sea_orm_migration::prelude::*;

use crate::entity::categories::Categories;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .if_not_exists()
                    .table(Categories::Table)
                    .col(
                        ColumnDef::new(Categories::Guid)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Categories::Category).text().not_null())
                    .col(ColumnDef::new(Categories::Description).text().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(Categories::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
