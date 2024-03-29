use sea_orm_migration::prelude::*;

use crate::entity::activities::ActivitiesEnum;
use crate::entity::activities_tags::ActivitiesTagsEnum;
use crate::entity::tags::TagsEnum;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .if_not_exists()
                    .table(ActivitiesTagsEnum::Table)
                    .col(
                        ColumnDef::new(ActivitiesTagsEnum::Guid)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ActivitiesTagsEnum::TagGuid)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ActivitiesTagsEnum::ActivityGuid)
                            .text()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_activities_tags_tag_guid")
                            .from(ActivitiesTagsEnum::Table, ActivitiesTagsEnum::TagGuid)
                            .to(TagsEnum::Table, TagsEnum::Guid),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_activities_tags_activity_guid")
                            .from(ActivitiesTagsEnum::Table, ActivitiesTagsEnum::ActivityGuid)
                            .to(ActivitiesEnum::Table, ActivitiesEnum::Guid),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ActivitiesTagsEnum::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
