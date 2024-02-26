//! Task entity and business logic

use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
};

use chrono::NaiveDateTime;
use serde_derive::{Deserialize, Serialize};
use typed_builder::TypedBuilder;
use ulid::Ulid;

use crate::domain::{priority::ItemPriorityKind, status::TaskStatus};

#[derive(Debug, TypedBuilder, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Task {
    created_at: NaiveDateTime,

    description: String,

    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    finished_at: Option<NaiveDateTime>,

    priority: ItemPriorityKind,

    status: TaskStatus,

    tags: Vec<String>,

    title: String,
    // TODO: It would be nice to have a way to track the number of pomodoro cycles for each task
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskList {
    /// The tasks in the list
    #[serde(flatten)]
    tasks: BTreeMap<TaskGuid, Task>,
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::FromSql for TaskGuid {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let bytes = <[u8; 16]>::column_result(value)?;
        Ok(Self(Ulid::from(u128::from_be_bytes(bytes))))
    }
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::ToSql for TaskGuid {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.0.to_string()))
    }
}

/// The unique identifier of an activity
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialEq, PartialOrd, Eq, Copy, Hash)]
pub struct TaskGuid(Ulid);

impl Display for TaskGuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for TaskGuid {
    fn default() -> Self {
        Self(Ulid::new())
    }
}

#[cfg(test)]
mod tests {

    use crate::error::TestResult;

    use super::*;
    use rstest::*;
    use std::{fs, path::PathBuf};

    #[rstest]
    fn test_parse_tasks_file_passes(
        #[files("../../config/tasks.pace.toml")] config_path: PathBuf,
    ) -> TestResult<()> {
        let toml_string = fs::read_to_string(config_path)?;
        let _ = toml::from_str::<TaskList>(&toml_string)?;

        Ok(())
    }
}
