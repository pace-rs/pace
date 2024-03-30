use sea_orm_migration::prelude::*;

use crate::entity::activities::ActivitiesEnum;
use crate::entity::activities_categories::ActivitiesCategoriesEnum;
use crate::entity::categories::CategoriesEnum;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &'life1 SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ActivitiesCategoriesEnum::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ActivitiesCategoriesEnum::Guid)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ActivitiesCategoriesEnum::ActivityGuid)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ActivitiesCategoriesEnum::CategoryGuid)
                            .text()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_activities_categories_category_guid")
                            .from(
                                ActivitiesCategoriesEnum::Table,
                                ActivitiesCategoriesEnum::CategoryGuid,
                            )
                            .to(CategoriesEnum::Table, CategoriesEnum::Guid),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_activities_categories_activity_guid")
                            .from(
                                ActivitiesCategoriesEnum::Table,
                                ActivitiesCategoriesEnum::ActivityGuid,
                            )
                            .to(ActivitiesEnum::Table, ActivitiesEnum::Guid),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &'life1 SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ActivitiesCategoriesEnum::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
