use sea_query::{ColumnDef, SqliteQueryBuilder, Table};

use crate::{entities::activity_status::ActivityStatusIden, migration::SQLiteMigration};

pub struct Migration;

impl SQLiteMigration for Migration {
    fn version(&self) -> String {
        "20240326125819".to_string()
    }

    fn up(&self) -> String {
        Table::create()
            .if_not_exists()
            .table(ActivityStatusIden::Table)
            .col(
                ColumnDef::new(ActivityStatusIden::Guid)
                    .text()
                    .not_null()
                    .primary_key(),
            )
            .col(ColumnDef::new(ActivityStatusIden::Status).text().not_null())
            .build(SqliteQueryBuilder)
    }

    fn down(&self) -> String {
        Table::drop()
            .table(ActivityStatusIden::Table)
            .build(SqliteQueryBuilder)
    }
}
