use std::collections::BinaryHeap;

use crate::domain::task::Task;

/// Inbox entity
#[derive(Debug, Default)]
pub struct Inbox {
    items: BinaryHeap<Task>,
}
