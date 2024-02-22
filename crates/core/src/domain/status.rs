use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum ItemStatus {
    Completed,
    #[serde(rename = "wip")]
    WorkInProgress,
    Paused,
    Pending,
    Scheduled,
    Started,
    Stopped,
    #[default]
    Todo,
    Waiting,
}
