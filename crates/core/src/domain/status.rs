use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum ActivityStatus {
    Active,
    Archived,
    Ended,
    #[default]
    Inactive,
    Held,
    Unarchived, // TODO: Do we need this or can be unarchiving done without it?
}

#[allow(clippy::trivially_copy_pass_by_ref)]
impl ActivityStatus {
    /// Returns `true` if the activity status is [`Active`].
    ///
    /// [`Active`]: ActivityStatus::Active
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns `true` if the activity status is [`Archived`].
    ///
    /// [`Archived`]: ActivityStatus::Archived
    #[must_use]
    pub const fn is_archived(self) -> bool {
        matches!(self, Self::Archived)
    }

    /// Returns `true` if the activity status is [`Ended`].
    ///
    /// [`Ended`]: ActivityStatus::Ended
    #[must_use]
    pub const fn is_ended(self) -> bool {
        matches!(self, Self::Ended)
    }

    /// Returns `true` if the activity status is [`Inactive`].
    ///
    /// [`Inactive`]: ActivityStatus::Inactive
    #[must_use]
    pub const fn is_inactive(self) -> bool {
        matches!(self, Self::Inactive)
    }

    /// Returns `true` if the activity status is [`Held`].
    ///
    /// [`Held`]: ActivityStatus::Held
    #[must_use]
    pub const fn is_held(self) -> bool {
        matches!(self, Self::Held)
    }

    /// Returns `true` if the activity status is [`Unarchived`].
    ///
    /// [`Unarchived`]: ActivityStatus::Unarchived
    #[must_use]
    pub const fn is_unarchived(&self) -> bool {
        matches!(self, Self::Unarchived)
    }
}
