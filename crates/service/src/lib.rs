/// An activity store service
///
/// This module contains the domain logic for tracking activities and their intermissions.
///
pub mod activity_store;

pub mod activity_tracker;

use std::sync::Arc;

use pace_core::prelude::{ActivityLogStorageKind, ActivityStorage, PaceConfig};
use pace_error::{DatabaseStorageErrorKind, PaceResult};
use tracing::debug;

use pace_storage::storage::{
    file::TomlActivityStorage, in_memory::InMemoryActivityStorage, sqlite::DatabaseActivityStorage,
};

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
pub fn get_storage_from_config(config: &PaceConfig) -> PaceResult<Arc<dyn ActivityStorage>> {
    let storage: Arc<dyn ActivityStorage> = match config.storage().storage() {
        ActivityLogStorageKind::File { location } => Arc::new(TomlActivityStorage::new(location)?),
        ActivityLogStorageKind::Database { kind, url } => {
            debug!("Connecting to SQLite database: {url}");

            Arc::new(DatabaseActivityStorage::new(*kind, url)?)
        }
        ActivityLogStorageKind::InMemory => Arc::new(InMemoryActivityStorage::new()),
        _ => return Err(DatabaseStorageErrorKind::StorageNotImplemented.into()),
    };

    debug!("Using storage backend: {storage:?}");

    Ok(storage)
}
