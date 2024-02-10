//! Task entity and business logic

use chrono::NaiveDateTime;
use serde_derive::{Deserialize, Serialize};
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::domain::{priority::ItemPriority, status::ItemStatus};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
pub struct TaskId(Uuid);

impl Default for TaskId {
    fn default() -> Self {
        Self(Uuid::now_v7())
    }
}

#[derive(Debug, TypedBuilder, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Task {
    created_at: NaiveDateTime,
    description: String,
    finished_at: Option<NaiveDateTime>,
    #[builder(default, setter(strip_option))]
    id: Option<TaskId>,
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
