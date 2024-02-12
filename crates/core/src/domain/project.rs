use std::collections::BinaryHeap;

use serde_derive::{Deserialize, Serialize};
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::domain::task::Task;

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectConfig {
    project: Project,
    subprojects: Vec<Subproject>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProjectId(Uuid);

impl Default for ProjectId {
    fn default() -> Self {
        Self(Uuid::now_v7())
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
