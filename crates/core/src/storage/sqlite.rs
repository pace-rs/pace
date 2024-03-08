use serde::{Deserialize, Serialize};

use crate::{
    domain::activity::{Activity, ActivityId, ActivityLog},
    error::PaceResult,
    storage::ActivityStorage,
};

struct SqliteActivityStorage {
    conn: Connection,
}
