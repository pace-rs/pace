use itertools::Itertools;
use pace_core::prelude::ActivityStatusKind;
use strum::IntoEnumIterator;
use ulid::Ulid;

use sea_orm_migration::prelude::*;

use crate::entity::activity_status::ActivityStatus;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let activity_kinds = ActivityStatusKind::iter()
            .map(|kind| {
                Query::insert()
                    .into_table(ActivityStatus::Table)
                    .columns([ActivityStatus::Guid, ActivityStatus::Status])
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
            .exec_stmt(Query::delete().from_table(ActivityStatus::Table).to_owned())
            .await
    }
}
