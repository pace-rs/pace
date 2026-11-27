//! The event types as the log names them, and the fields each allows.

use pace_core::EventKind;

/// The type of an event as the log names it: one per [`EventKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EventType {
    EntryBegun,
    EntryHeld,
    EntryPaused,
    EntryResumed,
    EntryEnded,
    CatchNoted,
    EntryLogged,
    EntryAdjusted,
    EntryForgotten,
    BatchUndone,
}

impl EventType {
    /// Every type, for [`EventType::parse`]. A type missing here can't be
    /// read: the test `every_event_kind_reads_back_as_written` writes one
    /// event of each kind and reads it back.
    const ALL: [Self; 10] = [
        Self::EntryBegun,
        Self::EntryHeld,
        Self::EntryPaused,
        Self::EntryResumed,
        Self::EntryEnded,
        Self::CatchNoted,
        Self::EntryLogged,
        Self::EntryAdjusted,
        Self::EntryForgotten,
        Self::BatchUndone,
    ];

    pub(super) const fn of(kind: &EventKind) -> Self {
        match kind {
            EventKind::EntryBegun { .. } => Self::EntryBegun,
            EventKind::EntryHeld { .. } => Self::EntryHeld,
            EventKind::EntryPaused { .. } => Self::EntryPaused,
            EventKind::EntryResumed { .. } => Self::EntryResumed,
            EventKind::EntryEnded { .. } => Self::EntryEnded,
            EventKind::CatchNoted { .. } => Self::CatchNoted,
            EventKind::EntryLogged { .. } => Self::EntryLogged,
            EventKind::EntryAdjusted { .. } => Self::EntryAdjusted,
            EventKind::EntryForgotten { .. } => Self::EntryForgotten,
            EventKind::BatchUndone { .. } => Self::BatchUndone,
        }
    }

    /// The value of `type` in the log.
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::EntryBegun => "entry.begun",
            Self::EntryHeld => "entry.held",
            Self::EntryPaused => "entry.paused",
            Self::EntryResumed => "entry.resumed",
            Self::EntryEnded => "entry.ended",
            Self::CatchNoted => "catch.noted",
            Self::EntryLogged => "entry.logged",
            Self::EntryAdjusted => "entry.adjusted",
            Self::EntryForgotten => "entry.forgotten",
            Self::BatchUndone => "batch.undone",
        }
    }

    /// The type that `type = "name"` names, if pace knows it.
    pub(super) fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|event_type| event_type.name() == name)
    }

    /// The fields the type allows besides the envelope.
    pub(super) const fn allowed(self) -> &'static [&'static str] {
        match self {
            Self::EntryBegun => &[
                "entry",
                "project",
                "project-key",
                "note",
                "tags",
                "role",
                "determined-by",
                "billable",
                "allow-overlap",
            ],
            Self::EntryHeld => &["entry", "source", "reason"],
            Self::EntryPaused => &["entry", "kind", "reason"],
            Self::EntryResumed | Self::EntryEnded | Self::EntryForgotten => &["entry"],
            Self::CatchNoted => &["entry", "text"],
            Self::EntryLogged => &[
                "entry",
                "end",
                "project",
                "project-key",
                "note",
                "tags",
                "role",
                "determined-by",
                "billable",
                "allow-overlap",
            ],
            Self::EntryAdjusted => &[
                "entry",
                "start",
                "end",
                "note",
                "project",
                "project-key",
                "tags",
                "role",
                "determined-by",
                "billable",
                "allow-overlap",
                "cleared",
            ],
            Self::BatchUndone => &["undoes"],
        }
    }
}
