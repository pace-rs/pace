//! Task entity and business logic

use chrono::NaiveDateTime;
use serde_derive::{Deserialize, Serialize};
use typed_builder::TypedBuilder;
use ulid::Ulid;

use crate::domain::{priority::ItemPriority, status::ItemStatus};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
pub struct TaskId(Ulid);

impl Default for TaskId {
    fn default() -> Self {
        Self(Ulid::new())
    }
}

#[derive(Debug, TypedBuilder, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Task {
    created_at: NaiveDateTime,

    description: String,

    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    finished_at: Option<NaiveDateTime>,

    #[builder(default, setter(strip_option))]
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    guid: Option<TaskId>,

    priority: ItemPriority,

    status: ItemStatus,

    tags: Vec<String>,

    title: String,
    // TODO: It would be nice to have a way to track the number of pomodoro cycles for each task
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskList {
    tasks: Vec<Task>,
}

impl rusqlite::types::FromSql for TaskId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let bytes = <[u8; 16]>::column_result(value)?;
        Ok(Self(Ulid::from(u128::from_be_bytes(bytes))))
    }
}

impl rusqlite::types::ToSql for TaskId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.0.to_string()))
    }
}
