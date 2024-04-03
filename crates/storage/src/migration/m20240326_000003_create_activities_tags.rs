use sea_orm_migration::prelude::*;

use crate::entity::activities::Activities;
use crate::entity::activities_tags::ActivitiesTags;
use crate::entity::tags::Tags;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &'life1 SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .if_not_exists()
                    .table(ActivitiesTags::Table)
                    .col(
                        ColumnDef::new(ActivitiesTags::Guid)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ActivitiesTags::TagGuid).text().not_null())
                    .col(
                        ColumnDef::new(ActivitiesTags::ActivityGuid)
                            .text()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_activities_tags_tag_guid")
                            .from(ActivitiesTags::Table, ActivitiesTags::TagGuid)
                            .to(Tags::Table, Tags::Guid),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_activities_tags_activity_guid")
                            .from(ActivitiesTags::Table, ActivitiesTags::ActivityGuid)
                            .to(Activities::Table, Activities::Guid),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &'life1 SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ActivitiesTags::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
