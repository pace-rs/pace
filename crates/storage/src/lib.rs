use std::sync::Arc;

use pace_core::prelude::{
    ActivityLogStorageKind, ActivityStorage, DatabaseEngineKind, PaceConfig, PaceStorageResult,
};
use tracing::debug;

use crate::{
    error::{DatabaseStorageErrorKind, PaceStorageErrorKind},
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

pub mod storage_types;

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
    let storage: Arc<dyn ActivityStorage> =
        match config.general().activity_log_options().storage_kind() {
            ActivityLogStorageKind::File => Arc::new(TomlActivityStorage::new(
                config.general().activity_log_options().path(),
            )?),
            ActivityLogStorageKind::Database => {
                if config.database().is_some() {
                    let Some(db_config) = config.database() else {
                        return Err(DatabaseStorageErrorKind::NoConfigSettings.into());
                    };

                    let storage: Arc<dyn ActivityStorage> = match db_config.engine() {
                        DatabaseEngineKind::Sqlite => {
                            #[cfg(feature = "rusqlite")]
                            {
                                let connection_string = config
                                    .database()
                                    .as_ref()
                                    .ok_or(DatabaseStorageErrorKind::NoConnectionString)?
                                    .connection_string();

                                debug!("Connecting to database: {}", &connection_string);

                                Arc::new(SqliteActivityStorage::new(connection_string.clone())?)
                            }

                            #[cfg(not(feature = "rusqlite"))]
                            return Err(PaceErrorKind::DatabaseStorageNotImplemented.into());
                        }
                        engine => {
                            return Err(DatabaseStorageErrorKind::UnsupportedDatabaseEngine(
                                *engine,
                            )
                            .into())
                        }
                    };

                    storage
                }

                return Err(PaceStorageErrorKind::DatabaseStorageNotConfigured.into());
            }
            ActivityLogStorageKind::InMemory => Arc::new(InMemoryActivityStorage::new()),
            _ => return Err(PaceStorageErrorKind::StorageNotImplemented.into()),
        };

    debug!("Using storage backend: {:?}", storage);

    Ok(storage)
}
