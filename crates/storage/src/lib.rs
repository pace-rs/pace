use std::sync::Arc;

use pace_core::prelude::{ActivityLogStorageKind, ActivityStorage, DatabaseEngineKind, PaceConfig};
use tracing::debug;

use crate::{
    error::{DatabaseStorageErrorKind, PaceStorageErrorKind, PaceStorageResult},
    file::TomlActivityStorage,
    in_memory::InMemoryActivityStorage,
    sqlite::SqliteActivityStorage,
};

pub mod error;

/// A type of storage that can be synced to a persistent medium - a file
pub mod file;

/// An in-memory storage backend for activities.
pub mod in_memory;

#[cfg(feature = "rusqlite")]
pub mod sqlite;

/// Get the storage backend from the configuration.
///
/// # Arguments
///
/// * `config` - The application configuration.
///
/// # Errors
///
/// This function should return an error if the storage backend cannot be created or is not supported.
///
/// # Returns
///
/// The storage backend.
pub fn get_storage_from_config(config: &PaceConfig) -> PaceStorageResult<Arc<dyn ActivityStorage>> {
    let storage: dyn ActivityStorage = match config.general().activity_log_options().storage_kind()
    {
        ActivityLogStorageKind::File => {
            TomlActivityStorage::new(config.general().activity_log_options().path())?.into()
        }
        ActivityLogStorageKind::Database => {
            if config.database().is_some() {
                let Some(db_config) = config.database() else {
                    return Err(DatabaseStorageErrorKind::NoConfigSettings.into());
                };

                match db_config.engine() {
                    DatabaseEngineKind::Sqlite => {
                        #[cfg(feature = "rusqlite")]
                        {
                            let connection_string = config
                                .database()
                                .as_ref()
                                .ok_or(DatabaseStorageErrorKind::NoConnectionString)?
                                .connection_string();

                            debug!("Connecting to database: {}", &connection_string);

                            SqliteActivityStorage::new(connection_string.clone())?.into()
                        }
                        #[cfg(not(feature = "rusqlite"))]
                        return Err(PaceErrorKind::DatabaseStorageNotImplemented.into());
                    }
                    engine => {
                        return Err(
                            DatabaseStorageErrorKind::UnsupportedDatabaseEngine(*engine).into()
                        )
                    }
                }
            }

            return Err(PaceStorageErrorKind::DatabaseStorageNotConfigured.into());
        }
        ActivityLogStorageKind::InMemory => InMemoryActivityStorage::new().into(),
    };

    debug!("Using storage backend: {}", storage);

    Ok(Arc::new(storage))
}
