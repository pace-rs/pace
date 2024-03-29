use sea_orm_migration::prelude::*;

use crate::entity::activities::Activities;
use crate::entity::activities_categories::ActivitiesCategories;
use crate::entity::categories::Categories;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ActivitiesCategories::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ActivitiesCategories::Guid)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ActivitiesCategories::ActivityGuid)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ActivitiesCategories::CategoryGuid)
                            .text()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_activities_categories_category_guid")
                            .from(
                                ActivitiesCategories::Table,
                                ActivitiesCategories::CategoryGuid,
                            )
                            .to(Categories::Table, Categories::Guid),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_activities_categories_activity_guid")
                            .from(
                                ActivitiesCategories::Table,
                                ActivitiesCategories::ActivityGuid,
                            )
                            .to(Activities::Table, Activities::Guid),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ActivitiesCategories::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
