/// A type of storage that can be synced to a persistent medium - a file
pub mod file;

/// An in-memory storage backend for activities.
pub mod in_memory;

pub mod sqlite;

use rusqlite::{Error, Row};

/// Trait for `SQLite` entities.
///
/// This trait is used to convert a `SQLite` row into a struct.
pub trait SQLiteEntity {
    /// Convert a `SQLite` row into a struct.
    ///
    /// # Arguments
    ///
    /// * `row` - A [`rusqlite::Row`].
    ///
    /// # Errors
    ///
    /// Returns an error if the conversion fails.
    ///
    /// # Returns
    ///
    /// Returns a struct.
    fn from_row(row: Row<'_>) -> Result<Self, Error>
    where
        Self: Sized;
}
