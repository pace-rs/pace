use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Default, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum ItemPriority {
    High,
    #[default]
    Medium,
    Low,
}
