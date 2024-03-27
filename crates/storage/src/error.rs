use std::path::PathBuf;

use displaydoc::Display;
use pace_core::prelude::{Activity, ActivityGuid, DatabaseEngineKind, PaceError};
use thiserror::Error;

pub type PaceStorageResult<T> = Result<T, PaceStorageErrorKind>;

/// [`PaceTimeErrorKind`] describes the errors that can happen while dealing with time.
#[non_exhaustive]
#[derive(Error, Debug, Display)]
pub enum PaceStorageErrorKind {
    /// Pace error: {0}
    #[error(transparent)]
    PaceError(#[from] PaceError),

    // /// SQLite error: {0}
    // #[error(transparent)]
    // #[cfg(feature = "rusqlite")]
    // SQLite(#[from] rusqlite::Error),
    /// Database error: {0}
    #[error(transparent)]
    Database(#[from] DatabaseStorageErrorKind),

    /// Toml file error: {0}
    #[error(transparent)]
    TomlFile(#[from] TomlFileStorageErrorKind),

    /// Database storage not configured
    DatabaseStorageNotConfigured,

    /// Database storage not implemented, yet!
    StorageNotImplemented,
}

/// [`DatabaseErrorKind`] describes the errors that can happen while dealing with the SQLite database.
#[non_exhaustive]
#[derive(Error, Debug, Display)]
pub enum DatabaseStorageErrorKind {
    /// Error connecting to database: {0} - {1}
    ConnectionFailed(String, String),

    /// No connection string provided
    NoConnectionString,

    /// This database engine is currently not supported: {0}
    UnsupportedDatabaseEngine(DatabaseEngineKind),

    /// Activity with id {0} not found
    ActivityNotFound(ActivityGuid),

    /// Failed to create activity: {0}
    ActivityCreationFailed(Activity),

    /// Failed to delete activity: {0}
    ActivityDeletionFailed(ActivityGuid),
}

/// [`TomlFileStorageErrorKind`] describes the errors that can happen while dealing with the Toml file storage.
#[non_exhaustive]
#[derive(Error, Debug, Display)]
pub enum TomlFileStorageErrorKind {
    /// Parent directory not found: {0}
    ParentDirNotFound(PathBuf),
}
