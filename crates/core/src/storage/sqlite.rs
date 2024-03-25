use crate::{
    domain::activity::Activity,
    error::{PaceResult, SqliteDatabaseStoreErrorKind},
    storage::ActivityStorage,
};
use rusqlite::{Connection, Result};

use serde::{Deserialize, Serialize};
use std::env;

struct SqliteActivityStorage {
    connection: Connection,
}

impl SqliteActivityStorage {
    pub fn new(connection_string: String) -> PaceResult<Self> {
        let connection = Connection::open(connection_string.as_str()).map_err(
            SqliteDatabaseStoreErrorKind::ConnectionFailed(connection_string),
        )?;

        Ok(Self { connection })
    }
}

impl ActivityStorage for SqliteActivityStorage {
    fn setup_storage(&self) -> PaceResult<()> {
        // TODO!: Check if the needed tables are existing or if we
        // are dealing with a fresh database, so we need to create
        // the tables

        //

        Ok(())
    }
}
