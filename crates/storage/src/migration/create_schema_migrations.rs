use sea_query::{ColumnDef, SqliteQueryBuilder, Table};

use crate::{entities::schema_migrations::SchemaMigrationsIden, migration::SQLiteMigration};

pub struct Migration;

impl SQLiteMigration for Migration {
    fn version(&self) -> String {
        "".to_string()
    }

    fn up(&self) -> String {
        Table::create()
            .table(SchemaMigrationsIden::Table)
            .col(
                ColumnDef::new(SchemaMigrationsIden::Guid)
                    .text()
                    .not_null()
                    .primary_key(),
            )
            .col(
                ColumnDef::new(SchemaMigrationsIden::Version)
                    .integer()
                    .not_null(),
            )
            .build(SqliteQueryBuilder)
    }

    fn down(&self) -> String {
        Table::drop()
            .table(SchemaMigrationsIden::Table)
            .build(SqliteQueryBuilder)
    }
}
