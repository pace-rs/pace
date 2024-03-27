use rusqlite::{Error, Row};

pub trait SQLiteEntity {
    fn from_row(row: Row<'_>) -> Result<Self, Error>
    where
        Self: Sized;
}
