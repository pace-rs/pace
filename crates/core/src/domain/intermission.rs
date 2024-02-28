//! Intermission entity and business logic

use serde_derive::{Deserialize, Serialize};

pub type IntermissionReason = Option<String>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum IntermissionAction {
    /// Extends the ongoing intermission
    #[default]
    Extend,
    /// Starts a new intermission
    New,
}

impl IntermissionAction {
    /// Returns `true` if the intermission action is [`Extend`].
    ///
    /// [`Extend`]: IntermissionAction::Extend
    #[must_use]
    pub fn is_extend(&self) -> bool {
        matches!(self, Self::Extend)
    }

    /// Returns `true` if the intermission action is [`New`].
    ///
    /// [`New`]: IntermissionAction::New
    #[must_use]
    pub fn is_new(&self) -> bool {
        matches!(self, Self::New)
    }
}
