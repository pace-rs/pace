use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ItemStatus {
    Completed,
    InProgress,
    Paused,
    Pending,
    Scheduled,
}
