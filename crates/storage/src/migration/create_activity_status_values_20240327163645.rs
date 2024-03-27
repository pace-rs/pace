use itertools::Itertools;
use pace_core::prelude::ActivityStatusKind;
use sea_query::{Query, SqliteQueryBuilder};
use strum::IntoEnumIterator;
use ulid::Ulid;

use crate::{entities::activity_status::ActivityStatusIden, migration::SQLiteMigration};

pub struct Migration;

impl SQLiteMigration for Migration {
    fn version(&self) -> String {
        "20240327163645".to_string()
    }

    fn up(&self) -> String {
        let activity_kinds = ActivityStatusKind::iter()
            .map(|kind| {
                Query::insert()
                    .into_table(ActivityStatusIden::Table)
                    .columns([ActivityStatusIden::Guid, ActivityStatusIden::Status])
                    .values_panic([Ulid::new().to_string().into(), kind.to_string().into()])
                    .to_owned()
                    .to_string(SqliteQueryBuilder)
            })
            .collect_vec();

        activity_kinds.join("; ")
    }

    fn down(&self) -> String {
        Query::delete()
            .from_table(ActivityStatusIden::Table)
            .to_owned()
            .to_string(SqliteQueryBuilder)
    }
}
