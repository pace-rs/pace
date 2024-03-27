use sea_query::{ColumnDef, SqliteQueryBuilder, Table};

use crate::{entities::categories::CategoriesIden, migration::SQLiteMigration};

pub struct Migration;

impl SQLiteMigration for Migration {
    fn version(&self) -> String {
        "20240326125937".to_string()
    }

    fn up(&self) -> String {
        Table::create()
            .if_not_exists()
            .table(CategoriesIden::Table)
            .col(
                ColumnDef::new(CategoriesIden::Guid)
                    .text()
                    .not_null()
                    .primary_key(),
            )
            .col(ColumnDef::new(CategoriesIden::Category).text().not_null())
            .col(ColumnDef::new(CategoriesIden::Description).text().null())
            .build(SqliteQueryBuilder)
    }

    fn down(&self) -> String {
        Table::drop()
            .table(CategoriesIden::Table)
            .build(SqliteQueryBuilder)
    }
}
