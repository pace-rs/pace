//! The machine-local index of the event files.
//!
//! It lives in the machine directory as `index.json`, is never synced, and
//! can always be deleted: pace rebuilds it. For each event file it keeps a
//! small summary, so that a read only parses the files it needs:
//!
//! - size and modification time, to notice a changed file
//! - the latest HLC, for the next event's clock and to find the files with
//!   events recorded since a moment
//! - the envelope of all moments in the file (`at`, `about`, `start`,
//!   `end`), to find the files that meet a time range
//! - the range of `about` moments, to find every file with events about the
//!   same entries: all events of an entry are filed under its first start
//! - the entries begun but not ended in the file, and the entries ended in
//!   the file but begun elsewhere; together they give the open entries,
//!   since entry IDs are unique across devices

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use jiff::Timestamp;
use pace_core::{EntryId, EventKind, Hlc, ReadScope, RecordedEvent};
use pace_error::PaceResult;
use serde::{Deserialize, Serialize};

use crate::hlc::{HlcParts, hlc_from_parts, hlc_parts};

/// The version of `index.json`; another version is rebuilt. Version 2
/// added `forgotten`.
pub(crate) const INDEX_VERSION: u32 = 2;

/// The index: summaries by file, relative to `PACE_HOME/events`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Index {
    pub version: u32,
    pub files: BTreeMap<String, FileSummary>,
}

/// What pace needs to know about an event file without parsing it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FileSummary {
    pub len: u64,
    pub modified: Option<u128>,
    /// The latest HLC as milliseconds and counter.
    pub max_hlc: Option<HlcParts>,
    /// The earliest and latest moment in the file, in milliseconds.
    pub envelope: Option<(i64, i64)>,
    /// The earliest and latest `about`, in milliseconds.
    pub abouts: Option<(i64, i64)>,
    /// Entries begun here without an end here.
    pub unended: Vec<EntryId>,
    /// Entries ended here but begun in another file.
    pub ends_elsewhere: Vec<EntryId>,
    /// Tombstones in the file: the forgotten entry and the moment its events
    /// are filed under, in milliseconds.
    pub forgotten: Vec<(EntryId, i64)>,
}

impl FileSummary {
    /// The latest HLC in the file, as the index at `index_path` holds it.
    ///
    /// # Errors
    ///
    /// If the index holds a reading outside the range of a timestamp.
    pub(crate) fn max_hlc(&self, index_path: &Path) -> PaceResult<Option<Hlc>> {
        self.max_hlc
            .map(|parts| hlc_from_parts(parts, index_path))
            .transpose()
    }
}

/// The summary of the events of one file.
pub(crate) fn summarize(events: &[RecordedEvent]) -> FileSummary {
    let undone: BTreeSet<_> = events
        .iter()
        .filter_map(|recorded| match recorded.event.kind {
            EventKind::BatchUndone { batch } => Some(batch),
            _ => None,
        })
        .collect();
    let live = events
        .iter()
        .filter(|recorded| !undone.contains(&recorded.batch));

    let mut begun: BTreeSet<EntryId> = BTreeSet::new();
    let mut ended: BTreeSet<EntryId> = BTreeSet::new();
    let mut logged: BTreeSet<EntryId> = BTreeSet::new();
    for recorded in live {
        match recorded.event.kind {
            EventKind::EntryBegun { entry, .. } => _ = begun.insert(entry),
            EventKind::EntryLogged { entry, .. } => _ = logged.insert(entry),
            EventKind::EntryEnded { entry } => _ = ended.insert(entry),
            EventKind::EntryHeld { .. }
            | EventKind::EntryPaused { .. }
            | EventKind::EntryResumed { .. }
            | EventKind::CatchNoted { .. }
            | EventKind::EntryAdjusted { .. }
            | EventKind::EntryForgotten { .. }
            | EventKind::BatchUndone { .. } => {}
        }
    }

    let mut moments = Vec::new();
    let mut abouts = Vec::new();
    for recorded in events {
        let event = &recorded.event;
        moments.push(event.at.timestamp().as_millisecond());
        moments.push(event.about.timestamp().as_millisecond());
        abouts.push(event.about.timestamp().as_millisecond());
        match &event.kind {
            EventKind::EntryLogged { end, .. } => moments.push(end.timestamp().as_millisecond()),
            EventKind::EntryAdjusted { changes, .. } => {
                moments.extend(
                    changes
                        .start
                        .iter()
                        .chain(&changes.end)
                        .map(|moment| moment.timestamp().as_millisecond()),
                );
            }
            // Their only moments are `at` and `about`.
            EventKind::EntryBegun { .. }
            | EventKind::EntryHeld { .. }
            | EventKind::EntryPaused { .. }
            | EventKind::EntryResumed { .. }
            | EventKind::EntryEnded { .. }
            | EventKind::CatchNoted { .. }
            | EventKind::EntryForgotten { .. }
            | EventKind::BatchUndone { .. } => {}
        }
    }

    FileSummary {
        len: 0,
        modified: None,
        max_hlc: events
            .iter()
            .map(|recorded| recorded.hlc)
            .max()
            .map(hlc_parts),
        envelope: span(&moments),
        abouts: span(&abouts),
        unended: begun.difference(&ended).copied().collect(),
        ends_elsewhere: ended
            .iter()
            .filter(|entry| !begun.contains(entry) && !logged.contains(entry))
            .copied()
            .collect(),
        forgotten: events
            .iter()
            .filter_map(|recorded| match recorded.event.kind {
                EventKind::EntryForgotten { entry } => {
                    Some((entry, recorded.event.about.timestamp().as_millisecond()))
                }
                _ => None,
            })
            .collect(),
    }
}

fn span(moments: &[i64]) -> Option<(i64, i64)> {
    Some((*moments.iter().min()?, *moments.iter().max()?))
}

/// The files to read for `scope`: those whose moments meet its range, those
/// where an open entry began, those with an event recorded since the
/// scope's moment, and every file with events about the same entries as
/// a selected file.
pub(crate) fn select(files: &BTreeMap<String, FileSummary>, scope: &ReadScope) -> BTreeSet<String> {
    let ended_elsewhere: BTreeSet<EntryId> = files
        .values()
        .flat_map(|summary| summary.ends_elsewhere.iter().copied())
        .collect();
    let range = scope.range();
    let start = range
        .start()
        .map(|start| start.timestamp().as_millisecond());
    let end = range.end().map(|end| end.timestamp().as_millisecond());
    let recorded_since = scope.recorded_since().map(Timestamp::as_millisecond);
    let meets = |(min, max): (i64, i64)| {
        start.is_none_or(|start| max >= start) && end.is_none_or(|end| min <= end)
    };

    let first: BTreeSet<&str> = files
        .iter()
        .filter(|(_, summary)| {
            summary.envelope.is_some_and(meets)
                || recorded_since
                    .is_some_and(|since| summary.max_hlc.is_some_and(|(latest, _)| latest >= since))
                || summary
                    .unended
                    .iter()
                    .any(|entry| !ended_elsewhere.contains(entry))
        })
        .map(|(name, _)| name.as_str())
        .collect();

    with_same_entries(files, first)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

/// `chosen` and every file whose `about` range meets one of theirs, until
/// no file comes along: a file that came along can bring an entry whose
/// other events lie in a file that meets none of the first ones, when
/// devices split their logs differently.
fn with_same_entries<'a>(
    files: &'a BTreeMap<String, FileSummary>,
    mut chosen: BTreeSet<&'a str>,
) -> BTreeSet<&'a str> {
    let abouts = |name: &str| files.get(name).and_then(|summary| summary.abouts);
    let mut frontier: Vec<(i64, i64)> = chosen.iter().filter_map(|name| abouts(name)).collect();
    while !frontier.is_empty() {
        let new: Vec<&str> = files
            .iter()
            .filter(|(name, summary)| {
                !chosen.contains(name.as_str())
                    && summary.abouts.is_some_and(|(min, max)| {
                        frontier
                            .iter()
                            .any(|&(chosen_min, chosen_max)| min <= chosen_max && chosen_min <= max)
                    })
            })
            .map(|(name, _)| name.as_str())
            .collect();
        frontier = new.iter().filter_map(|name| abouts(name)).collect();
        chosen.extend(new);
    }
    chosen
}

#[cfg(test)]
mod tests;
