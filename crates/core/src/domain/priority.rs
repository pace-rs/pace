use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Default, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ItemPriorityKind {
    High,
    #[default]
    Medium,
    Low,
}
