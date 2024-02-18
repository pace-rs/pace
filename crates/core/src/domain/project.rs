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
pub struct ProjectId(Ulid);

impl Default for ProjectId {
    fn default() -> Self {
        Self(Ulid::new())
    }
}

impl rusqlite::types::FromSql for ProjectId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let bytes = <[u8; 16]>::column_result(value)?;
        Ok(Self(Ulid::from(u128::from_be_bytes(bytes))))
    }
}

impl rusqlite::types::ToSql for ProjectId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.0.to_string()))
    }
}

#[derive(Debug, TypedBuilder, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Project {
    #[builder(default, setter(strip_option))]
    id: Option<ProjectId>,

    name: String,

    description: Option<String>,

    // TODO: Broken Eq impl
    // #[serde(skip)]
    // next_actions: BinaryHeap<Task>,
    finished: Option<Vec<Task>>,

    archived: Option<Vec<Task>>,

    root_tasks_file: String,
}

#[derive(Serialize, Deserialize, Debug, TypedBuilder)]
struct Subproject {
    #[builder(default, setter(strip_option))]
    id: Option<ProjectId>,
    name: String,
    description: String,
    tasks_file: String,
}
