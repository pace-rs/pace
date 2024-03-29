use sea_query::{ColumnDef, SqliteQueryBuilder, Table};

use crate::{entities::description::DescriptionsIden, migration::SQLiteMigration};

pub struct Migration;

impl SQLiteMigration for Migration {
    fn version(&self) -> String {
        "20240329131712".to_string()
    }

    fn up(&self) -> String {
        Table::create()
            .if_not_exists()
            .table(DescriptionsIden::Table)
            .col(
                ColumnDef::new(DescriptionsIden::Guid)
                    .text()
                    .not_null()
                    .primary_key(),
            )
            .col(
                ColumnDef::new(DescriptionsIden::Description)
                    .text()
                    .not_null(),
            )
            .build(SqliteQueryBuilder)
    }

    fn down(&self) -> String {
        Table::drop()
            .table(DescriptionsIden::Table)
            .build(SqliteQueryBuilder)
    }
}
