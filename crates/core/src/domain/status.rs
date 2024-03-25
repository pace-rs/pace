use serde_derive::{Deserialize, Serialize};
use strum::EnumString;

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

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumString,
    strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum ActivityStatusKind {
    /// The initial state of an activity once it's created in the system but not yet started.
    #[default]
    Created,

    /// The activity is scheduled to start at a specific time.
    /// It remains in this state until the activity begins.
    Scheduled,

    /// The active state of an activity. It transitions to this state from "Scheduled" when
    /// the activity begins or from "Paused" when it's resumed. The start time is recorded
    /// upon entering this state for the first time, and the resume time is noted for
    /// subsequent entries.
    InProgress,

    /// Represents an activity that has been temporarily halted.
    /// This could apply to tasks being paused for a break or intermission.
    /// The activity can move back to "InProgress" when work on it resumes.
    Paused,

    /// The final state of an activity, indicating it has been finished.
    /// The end time of the activity is recorded, marking its completion.
    Completed,

    Archived,
    Unarchived, // TODO: Do we need this or can be unarchiving done without it?
}

#[allow(clippy::trivially_copy_pass_by_ref)]
impl ActivityStatusKind {
    /// Returns `true` if the activity status is [`InProgress`].
    ///
    /// [`InProgress`]: ActivityStatusKind::InProgress
    #[must_use]
    pub const fn is_in_progress(self) -> bool {
        matches!(self, Self::InProgress)
    }

    /// Returns `true` if the activity status is [`Archived`].
    ///
    /// [`Archived`]: ActivityStatusKind::Archived
    #[must_use]
    pub const fn is_archived(self) -> bool {
        matches!(self, Self::Archived)
    }

    /// Returns `true` if the activity status is [`Completed`].
    ///
    /// [`Completed`]: ActivityStatusKind::Completed
    #[must_use]
    pub const fn is_completed(self) -> bool {
        matches!(self, Self::Completed)
    }

    /// Returns `true` if the activity status is [`Created`].
    ///
    /// [`Created`]: ActivityStatusKind::Created
    #[must_use]
    pub const fn is_created(self) -> bool {
        matches!(self, Self::Created)
    }

    /// Returns `true` if the activity status is [`Paused`].
    ///
    /// [`Paused`]: ActivityStatusKind::Paused
    #[must_use]
    pub const fn is_paused(self) -> bool {
        matches!(self, Self::Paused)
    }

    /// Returns `true` if the activity status is [`Unarchived`].
    ///
    /// [`Unarchived`]: ActivityStatusKind::Unarchived
    #[must_use]
    pub const fn is_unarchived(&self) -> bool {
        matches!(self, Self::Unarchived)
    }
}
