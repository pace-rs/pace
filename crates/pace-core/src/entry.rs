//! Entries: a stretch of tracked time with its note, project and tags, its
//! holds, pauses and catches, and what it is doing now.

use std::collections::BTreeSet;

use jiff::{SignedDuration, ZonedDateTime};
use serde::{Deserialize, Serialize};

use crate::{DeviceId, EntryId, HoldSource, PauseKind, ProjectId, ProjectKey, Tag};

/// A reference to a project: its ID, or its key while it has no ID yet.
///
/// A project gets its ID from `pace project init`; until then entries record
/// the key, and pace resolves it to the ID once the project has one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectRef {
    /// The project's ID.
    Id(ProjectId),
    /// The project's key, for a project without an ID.
    Key(ProjectKey),
}

/// How an entry came to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryOrigin {
    /// Recorded live with `pace begin`.
    Live,
    /// Entered afterwards with `pace log`.
    Logged,
}

/// What an entry is doing; derived, never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryState {
    /// No end, no open hold, no open pause.
    Running,
    /// An open hold.
    Held,
    /// An open pause.
    Paused,
    /// An end is set.
    Ended,
}

/// An interruption inside an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryHold {
    /// When it began.
    pub start: ZonedDateTime,
    /// When the work went on; `None` while held.
    pub end: Option<ZonedDateTime>,
    /// Where it came from.
    pub source: HoldSource,
    /// Why.
    pub reason: Option<String>,
}

/// A planned break inside an entry; never an interruption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryPause {
    /// When it began.
    pub start: ZonedDateTime,
    /// When the work went on; `None` while paused.
    pub end: Option<ZonedDateTime>,
    /// The kind of break.
    pub kind: PauseKind,
    /// Why.
    pub reason: Option<String>,
}

/// A distraction written down instead of followed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryCatch {
    /// When it was written down.
    pub at: ZonedDateTime,
    /// The thought.
    pub text: String,
}

/// A span of actual time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its identity.
    pub id: EntryId,
    /// The device it was begun or logged on.
    pub device: DeviceId,
    /// When it began, in the zone it was recorded in.
    pub start: ZonedDateTime,
    /// When it ended; `None` while running, held or paused.
    pub end: Option<ZonedDateTime>,
    /// Its project.
    pub project: Option<ProjectRef>,
    /// What it is about.
    pub note: Option<String>,
    /// Its tags.
    pub tags: BTreeSet<Tag>,
    /// Its interruptions.
    pub holds: Vec<EntryHold>,
    /// Its planned breaks.
    pub pauses: Vec<EntryPause>,
    /// The distractions written down during it.
    pub catches: Vec<EntryCatch>,
    /// Recorded live or entered afterwards.
    pub origin: EntryOrigin,
    /// The moment the entry's events are filed under: its first start,
    /// which an adjusted start doesn't change.
    pub filed: ZonedDateTime,
}

impl Entry {
    /// What the entry is doing.
    #[must_use]
    pub fn state(&self) -> EntryState {
        if self.end.is_some() {
            EntryState::Ended
        } else if self.open_hold().is_some() {
            EntryState::Held
        } else if self.open_pause().is_some() {
            EntryState::Paused
        } else {
            EntryState::Running
        }
    }

    /// Whether the entry has no end yet.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.end.is_none()
    }

    /// The end, or `now` for an open entry.
    pub(crate) fn until<'moment>(
        &'moment self,
        now: &'moment ZonedDateTime,
    ) -> &'moment ZonedDateTime {
        self.end.as_ref().unwrap_or(now)
    }

    /// End minus start; an open entry counts up to `now`.
    #[must_use]
    pub fn gross(&self, now: &ZonedDateTime) -> SignedDuration {
        span(&self.start, self.until(now))
    }

    /// The time spent in holds, inside the entry.
    #[must_use]
    pub fn hold_time(&self, now: &ZonedDateTime) -> SignedDuration {
        self.holds
            .iter()
            .map(|hold| self.break_time(&hold.start, hold.end.as_ref(), now))
            .sum()
    }

    /// The time spent in pauses, inside the entry.
    #[must_use]
    pub fn pause_time(&self, now: &ZonedDateTime) -> SignedDuration {
        self.pauses
            .iter()
            .map(|pause| self.break_time(&pause.start, pause.end.as_ref(), now))
            .sum()
    }

    /// The part of a break that lies inside the entry; an open break lasts
    /// until the entry's end or `now`.
    ///
    /// The rules keep every break inside its entry, but a merge of two
    /// devices can move the entry's start or end into a break: an
    /// adjustment from a device that never saw the hold.
    fn break_time(
        &self,
        start: &ZonedDateTime,
        end: Option<&ZonedDateTime>,
        now: &ZonedDateTime,
    ) -> SignedDuration {
        let until = self.until(now);
        let end = end.map_or(until, |end| end.min(until));
        span(start.max(&self.start), end)
    }

    /// Gross time minus holds and pauses; what billing and target vs. actual
    /// use. Never negative: breaks merged from two devices can overlap.
    #[must_use]
    pub fn net(&self, now: &ZonedDateTime) -> SignedDuration {
        (self.gross(now) - self.hold_time(now) - self.pause_time(now)).max(SignedDuration::ZERO)
    }

    /// How often the entry was interrupted; pauses and catches don't count.
    #[must_use]
    pub const fn interruptions(&self) -> usize {
        self.holds.len()
    }

    /// The hold without an end, while the entry is held.
    #[must_use]
    pub fn open_hold(&self) -> Option<&EntryHold> {
        self.holds.iter().find(|hold| hold.end.is_none())
    }

    /// The pause without an end, while the entry is paused.
    #[must_use]
    pub fn open_pause(&self) -> Option<&EntryPause> {
        self.pauses.iter().find(|pause| pause.end.is_none())
    }

    /// When the open hold or pause began, while the entry is held or
    /// paused.
    #[must_use]
    pub fn open_break_start(&self) -> Option<&ZonedDateTime> {
        self.open_hold()
            .map(|hold| &hold.start)
            .or_else(|| self.open_pause().map(|pause| &pause.start))
    }

    pub(crate) fn open_hold_mut(&mut self) -> Option<&mut EntryHold> {
        self.holds.iter_mut().find(|hold| hold.end.is_none())
    }

    pub(crate) fn open_pause_mut(&mut self) -> Option<&mut EntryPause> {
        self.pauses.iter_mut().find(|pause| pause.end.is_none())
    }

    /// Close an open hold or pause at `at`.
    pub(crate) fn close_breaks(&mut self, at: &ZonedDateTime) {
        if let Some(hold) = self.open_hold_mut() {
            hold.end = Some(at.clone());
        }
        if let Some(pause) = self.open_pause_mut() {
            pause.end = Some(at.clone());
        }
    }
}

/// The time from `start` to `end`, never negative: events merged from two
/// devices can close a break before it began.
fn span(start: &ZonedDateTime, end: &ZonedDateTime) -> SignedDuration {
    start.duration_until(end).max(SignedDuration::ZERO)
}
