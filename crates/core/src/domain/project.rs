use serde_derive::{Deserialize, Serialize};
use typed_builder::TypedBuilder;
use ulid::Ulid;

use crate::domain::task::Task;

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectConfig {
    project: Project,
    subprojects: Vec<Subproject>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProjectGuid(Ulid);

impl Default for ProjectGuid {
    fn default() -> Self {
        Self(Ulid::new())
    }
}

impl rusqlite::types::FromSql for ProjectGuid {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let bytes = <[u8; 16]>::column_result(value)?;
        Ok(Self(Ulid::from(u128::from_be_bytes(bytes))))
    }
}

impl rusqlite::types::ToSql for ProjectGuid {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.0.to_string()))
    }
}

#[derive(Debug, TypedBuilder, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Project {
    #[builder(default, setter(strip_option))]
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    guid: Option<ProjectGuid>,

    name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,

    // TODO: Broken Eq impl
    // #[serde(skip)]
    // next_actions: BinaryHeap<Task>,
    #[serde(skip_serializing_if = "Option::is_none")]
    finished: Option<Vec<Task>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    archived: Option<Vec<Task>>,

    root_tasks_file: String,
}

#[derive(Serialize, Deserialize, Debug, TypedBuilder)]
struct Subproject {
    #[builder(default, setter(strip_option))]
    id: Option<ProjectGuid>,
    name: String,
    description: String,
    tasks_file: String,
}
