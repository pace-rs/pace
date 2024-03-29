use itertools::Itertools;
use pace_core::prelude::ActivityKind;
use sea_orm_migration::prelude::*;
use strum::IntoEnumIterator;
use ulid::Ulid;

use crate::entity::activity_kinds::ActivityKindsEnum;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let activity_kinds = ActivityKind::iter()
            .map(|kind| {
                Query::insert()
                    .into_table(ActivityKindsEnum::Table)
                    .columns([ActivityKindsEnum::Guid, ActivityKindsEnum::Kind])
                    .values_panic([Ulid::new().to_string().into(), kind.to_string().into()])
                    .to_owned()
            })
            .collect_vec();

        for kind in activity_kinds {
            manager.exec_stmt(kind).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .exec_stmt(
                Query::delete()
                    .from_table(ActivityKindsEnum::Table)
                    .to_owned(),
            )
            .await
    }
}
