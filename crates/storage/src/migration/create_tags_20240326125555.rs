use sea_query::{ColumnDef, SqliteQueryBuilder, Table};

use crate::{entities::tags::TagsIden, migration::SQLiteMigration};

pub struct Migration;

impl SQLiteMigration for Migration {
    fn version(&self) -> String {
        "20240326125555".to_string()
    }

    fn up(&self) -> String {
        Table::create()
            .if_not_exists()
            .table(TagsIden::Table)
            .col(
                ColumnDef::new(TagsIden::Guid)
                    .text()
                    .not_null()
                    .primary_key(),
            )
            .col(ColumnDef::new(TagsIden::Tag).text().not_null())
            .build(SqliteQueryBuilder)
    }

    fn down(&self) -> String {
        Table::drop()
            .table(TagsIden::Table)
            .build(SqliteQueryBuilder)
    }
}
