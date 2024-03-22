//! Intermission entity and business logic

use serde_derive::{Deserialize, Serialize};

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, Default, displaydoc::Display,
)]
#[serde(rename_all = "snake_case")]
pub enum IntermissionAction {
    /// Extend the current intermission
    #[default]
    Extend,
    /// Start a new intermission
    New,
}

impl From<bool> for IntermissionAction {
    fn from(new: bool) -> Self {
        if new {
            Self::New
        } else {
            Self::Extend
        }
    }
}

impl IntermissionAction {
    /// Returns `true` if the intermission action is [`Extend`].
    ///
    /// [`Extend`]: IntermissionAction::Extend
    #[must_use]
    pub const fn is_extend(&self) -> bool {
        matches!(self, Self::Extend)
    }

    /// Returns `true` if the intermission action is [`New`].
    ///
    /// [`New`]: IntermissionAction::New
    #[must_use]
    pub const fn is_new(&self) -> bool {
        matches!(self, Self::New)
    }
}
