use sea_orm_migration::prelude::*;

use crate::entity::activity_kinds::ActivityKinds;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .if_not_exists()
                    .table(ActivityKinds::Table)
                    .col(
                        ColumnDef::new(ActivityKinds::Guid)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ActivityKinds::Kind).text().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ActivityKinds::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
