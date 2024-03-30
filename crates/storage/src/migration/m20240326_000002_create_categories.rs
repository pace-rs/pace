use sea_orm_migration::prelude::*;

use crate::entity::categories::CategoriesEnum;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &'life1 SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .if_not_exists()
                    .table(CategoriesEnum::Table)
                    .col(
                        ColumnDef::new(CategoriesEnum::Guid)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CategoriesEnum::Category).text().not_null())
                    .col(ColumnDef::new(CategoriesEnum::Description).text().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &'life1 SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(CategoriesEnum::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
