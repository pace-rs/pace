use std::collections::BinaryHeap;

use crate::domain::task::Task;

pub struct Inbox {
    items: BinaryHeap<Task>,
}
