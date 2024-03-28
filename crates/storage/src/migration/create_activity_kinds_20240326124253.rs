use sea_query::{ColumnDef, SqliteQueryBuilder, Table};

use crate::{entities::activity_kinds::ActivityKindsIden, migration::SQLiteMigration};

pub struct Migration;

impl SQLiteMigration for Migration {
    fn version(&self) -> String {
        "20240326124253".to_string()
    }

    fn up(&self) -> String {
        Table::create()
            .if_not_exists()
            .table(ActivityKindsIden::Table)
            .col(
                ColumnDef::new(ActivityKindsIden::Guid)
                    .text()
                    .not_null()
                    .primary_key(),
            )
            .col(ColumnDef::new(ActivityKindsIden::Kind).text().not_null())
            .build(SqliteQueryBuilder)
    }

    fn down(&self) -> String {
        Table::drop()
            .table(ActivityKindsIden::Table)
            .build(SqliteQueryBuilder)
    }
}
