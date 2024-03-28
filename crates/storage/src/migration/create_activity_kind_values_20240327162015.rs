use itertools::Itertools;
use pace_core::prelude::ActivityKind;
use sea_query::{Query, SqliteQueryBuilder};
use strum::IntoEnumIterator;
use ulid::Ulid;

use crate::{entities::activity_kinds::ActivityKindsIden, migration::SQLiteMigration};

pub struct Migration;

impl SQLiteMigration for Migration {
    fn version(&self) -> String {
        "20240327162015".to_string()
    }

    fn up(&self) -> String {
        let activity_kinds = ActivityKind::iter()
            .map(|kind| {
                Query::insert()
                    .into_table(ActivityKindsIden::Table)
                    .columns([ActivityKindsIden::Guid, ActivityKindsIden::Kind])
                    .values_panic([Ulid::new().to_string().into(), kind.to_string().into()])
                    .to_owned()
                    .to_string(SqliteQueryBuilder)
            })
            .collect_vec();

        activity_kinds.join("; ")
    }

    fn down(&self) -> String {
        Query::delete()
            .from_table(ActivityKindsIden::Table)
            .to_owned()
            .to_string(SqliteQueryBuilder)
    }
}
