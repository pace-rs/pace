//! The checks of times against an entry: its length, its breaks, and a
//! moment before another one.

use std::cmp::Ordering;

use jiff::ZonedDateTime;
use pace_error::{ErrorKind, PaceError, PaceResult};

use crate::{
    Entry, EntryChanges,
    rules::{BREAK_BEGAN, ENTRY_BEGAN, LAST_BREAK_ENDED, NOW_HINT, Rules},
};

impl Rules<'_> {
    /// A new hold or pause may not begin before the entry, before the open
    /// break it replaces, or before the end of an earlier one.
    pub(super) fn check_break_start(&self, entry: &Entry, at: &ZonedDateTime) -> PaceResult<()> {
        if *at < entry.start {
            return Err(self.after(ENTRY_BEGAN, &entry.start, at));
        }
        if let Some(open_since) = open_break_start(entry)
            && at < open_since
        {
            return Err(self.after(BREAK_BEGAN, open_since, at));
        }
        if let Some(last) = last_break_end(entry)
            && at < last
        {
            return Err(self.after(LAST_BREAK_ENDED, last, at));
        }
        Ok(())
    }

    /// The switch of `begin` ends the running entry at the new entry's
    /// start, so the start comes after the open break began and after the
    /// closed breaks.
    pub(super) fn check_switch_after_breaks(
        &self,
        running: &Entry,
        start: &ZonedDateTime,
    ) -> PaceResult<()> {
        if let Some(open_since) = open_break_start(running)
            && start < open_since
        {
            return Err(self.after(BREAK_BEGAN, open_since, start));
        }
        if let Some(last) = last_break_end(running)
            && start < last
        {
            return Err(PaceError::new(
                ErrorKind::InvalidInput,
                "the running entry has a break until {at}, after the new entry's start at {start}",
            )
            .attach_context("at", self.clock(last))
            .attach_context("start", self.clock(start))
            .attach_hint(format!(
                "begin at {} or later with `--at`",
                self.clock(last)
            )));
        }
        Ok(())
    }

    /// An end at `at` closes the open break, so it may not come before the
    /// break began, and the closed breaks lie before it.
    pub(super) fn check_end_after_breaks(
        &self,
        entry: &Entry,
        at: &ZonedDateTime,
    ) -> PaceResult<()> {
        if let Some(open_since) = open_break_start(entry)
            && at < open_since
        {
            return Err(self.after(BREAK_BEGAN, open_since, at));
        }
        self.check_breaks_inside(entry, &entry.start, Some(at))
    }

    /// The checks of an adjustment that moves the start or the end; a time
    /// it leaves alone is not checked again.
    pub(super) fn check_moved_times(
        &self,
        entry: &Entry,
        changes: &EntryChanges,
        start: &ZonedDateTime,
        end: Option<&ZonedDateTime>,
    ) -> PaceResult<()> {
        if let Some(end) = end {
            self.check_length(start, end)?;
        }
        if let Some(start) = &changes.start {
            self.refuse_future("start", start, NOW_HINT)?;
        }
        if let Some(end) = &changes.end {
            self.refuse_future("end", end, NOW_HINT)?;
        }
        self.check_breaks_inside(entry, start, end)
    }

    /// After an adjustment, holds and pauses still lie inside the entry.
    fn check_breaks_inside(
        &self,
        entry: &Entry,
        start: &ZonedDateTime,
        end: Option<&ZonedDateTime>,
    ) -> PaceResult<()> {
        let breaks = entry
            .holds
            .iter()
            .map(|hold| (&hold.start, hold.end.as_ref()))
            .chain(
                entry
                    .pauses
                    .iter()
                    .map(|pause| (&pause.start, pause.end.as_ref())),
            );
        for (break_start, break_end) in breaks {
            if break_start < start {
                return Err(PaceError::new(
                    ErrorKind::InvalidInput,
                    "the entry has a break at {at}, before the new start at {start}",
                )
                .attach_context("at", self.clock(break_start))
                .attach_context("start", self.clock(start))
                .attach_hint("pick a start before its first break"));
            }
            if let (Some(end), Some(break_end)) = (end, break_end)
                && break_end > end
            {
                return Err(PaceError::new(
                    ErrorKind::InvalidInput,
                    "the entry has a break until {at}, after the new end at {end}",
                )
                .attach_context("at", self.clock(break_end))
                .attach_context("end", self.clock(end))
                .attach_hint("pick an end after its last break"));
            }
        }
        Ok(())
    }

    /// `at` comes before `earlier`; `guidance` names both, as `{earlier}`
    /// and `{at}`.
    pub(super) fn after(
        &self,
        guidance: &'static str,
        earlier: &ZonedDateTime,
        at: &ZonedDateTime,
    ) -> Box<PaceError> {
        PaceError::new(ErrorKind::InvalidInput, guidance)
            .attach_context("earlier", self.clock(earlier))
            .attach_context("at", self.clock(at))
            .attach_hint(format!("give a time after {}", self.clock(earlier)))
    }

    /// An entry ends after it starts: `end`, `log` and `adjust` refuse an
    /// entry without length alike.
    pub(super) fn check_length(
        &self,
        start: &ZonedDateTime,
        end: &ZonedDateTime,
    ) -> PaceResult<()> {
        let error = match end.cmp(start) {
            Ordering::Greater => return Ok(()),
            Ordering::Equal => PaceError::new(
                ErrorKind::InvalidInput,
                "the entry would end at {end}, the moment it starts",
            ),
            Ordering::Less => PaceError::new(
                ErrorKind::InvalidInput,
                "the entry would end at {end}, before it starts at {start}",
            )
            .attach_context("start", self.clock(start)),
        };
        Err(error
            .attach_context("end", self.clock(end))
            .attach_hint("give an end after the start"))
    }
}

/// The latest end of the entry's closed holds and pauses, if any.
fn last_break_end(entry: &Entry) -> Option<&ZonedDateTime> {
    entry
        .holds
        .iter()
        .filter_map(|hold| hold.end.as_ref())
        .chain(entry.pauses.iter().filter_map(|pause| pause.end.as_ref()))
        .max()
}

/// The start of the entry's open hold or pause, if any.
pub(super) fn open_break_start(entry: &Entry) -> Option<&ZonedDateTime> {
    entry
        .holds
        .iter()
        .find(|hold| hold.end.is_none())
        .map(|hold| &hold.start)
        .or_else(|| {
            entry
                .pauses
                .iter()
                .find(|pause| pause.end.is_none())
                .map(|pause| &pause.start)
        })
}
