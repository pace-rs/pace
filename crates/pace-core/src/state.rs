//! The state of the entries, built from the events in merged order.

use std::collections::{BTreeMap, BTreeSet};

use jiff::ZonedDateTime;

use crate::{
    BatchId, Change, DeviceId, Entry, EntryChanges, EntryId, EntryOrigin, EventKind, HoldSource,
    PauseKind, ProjectRef, RecordedEvent, Tag,
    entry::{Catch, Hold, Pause},
};

/// The current state: the result of applying all events in merged order.
///
/// Building it never fails. Events in an undone batch count as never
/// written, a forgotten entry is gone, and an event about an entry that
/// doesn't exist, or that can't apply any more, changes nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    entries: BTreeMap<EntryId, Entry>,
    batches: Vec<BatchRecord>,
}

/// A batch as far as `undo` cares.
#[derive(Debug, Clone, PartialEq, Eq)]
struct BatchRecord {
    id: BatchId,
    device: DeviceId,
    undone: bool,
    /// Undo and forget can't be undone.
    undoable: bool,
    /// The entries its events concern; undoing a batch whose entries are all
    /// gone for good would change nothing.
    entries: BTreeSet<EntryId>,
    /// The moments its events are filed under, each once.
    abouts: Vec<ZonedDateTime>,
}

/// What `undo` reverts: a batch and the moments its events are filed under,
/// so the undo is filed with each of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoTarget {
    /// The batch.
    pub batch: BatchId,
    /// The moments its events are filed under.
    pub abouts: Vec<ZonedDateTime>,
}

impl State {
    /// Apply `events`, from any number of devices and in any order, in
    /// merged order: by [`Hlc`](crate::Hlc), ties broken by device ID, then
    /// by event ID ([`RecordedEvent::merge_key`]).
    #[must_use]
    pub fn from_events(events: &[RecordedEvent]) -> Self {
        let mut ordered: Vec<&RecordedEvent> = events.iter().collect();
        ordered.sort_by_key(|recorded| recorded.merge_key());

        let undone: BTreeSet<BatchId> = ordered
            .iter()
            .filter_map(|recorded| match recorded.event.kind {
                EventKind::BatchUndone { batch } => Some(batch),
                _ => None,
            })
            .collect();
        let forgotten: BTreeSet<EntryId> = ordered
            .iter()
            .filter_map(|recorded| match recorded.event.kind {
                EventKind::EntryForgotten { entry } => Some(entry),
                _ => None,
            })
            .collect();

        let mut state = Self::default();
        for recorded in ordered {
            state.record_batch(recorded, undone.contains(&recorded.batch));
            if !undone.contains(&recorded.batch) {
                state.apply(recorded, &forgotten);
            }
        }
        state
    }

    fn record_batch(&mut self, recorded: &RecordedEvent, undone: bool) {
        let undoable = !matches!(
            recorded.event.kind,
            EventKind::BatchUndone { .. } | EventKind::EntryForgotten { .. }
        );
        let entry = recorded.event.kind.entry();
        let about = &recorded.event.about;
        if let Some(batch) = self
            .batches
            .iter_mut()
            .find(|batch| batch.id == recorded.batch)
        {
            batch.undoable &= undoable;
            batch.entries.extend(entry);
            if !batch.abouts.contains(about) {
                batch.abouts.push(about.clone());
            }
        } else {
            self.batches.push(BatchRecord {
                id: recorded.batch,
                device: recorded.device,
                undone,
                undoable,
                entries: entry.into_iter().collect(),
                abouts: vec![about.clone()],
            });
        }
    }

    /// The last batch of `device` that can be undone and isn't yet; a batch
    /// whose entries are all gone for good, forgotten or begun in an undone
    /// batch, is left out.
    #[must_use]
    pub fn last_undoable(&self, device: DeviceId) -> Option<UndoTarget> {
        self.batches
            .iter()
            .rev()
            .find(|batch| {
                batch.device == device
                    && batch.undoable
                    && !batch.undone
                    && batch.entries.iter().any(|id| self.entries.contains_key(id))
            })
            .map(|batch| UndoTarget {
                batch: batch.id,
                abouts: batch.abouts.clone(),
            })
    }

    /// The entries other than `except` whose time overlaps `[start, end)`;
    /// an open entry lasts until `now`.
    pub fn overlapping<'s>(
        &'s self,
        start: &ZonedDateTime,
        end: &ZonedDateTime,
        except: &[EntryId],
        now: &ZonedDateTime,
    ) -> impl Iterator<Item = &'s Entry> {
        let (start, end, now) = (start.clone(), end.clone(), now.clone());
        let except = except.to_vec();
        self.entries().filter(move |entry| {
            let entry_end = entry.end.as_ref().unwrap_or(&now);
            !except.contains(&entry.id) && entry.start < end && start < *entry_end
        })
    }

    fn apply(&mut self, recorded: &RecordedEvent, forgotten: &BTreeSet<EntryId>) {
        let at = &recorded.event.at;
        match &recorded.event.kind {
            EventKind::EntryBegun {
                entry,
                project,
                note,
                tags,
            } => self.insert(
                new_entry(
                    recorded,
                    *entry,
                    None,
                    project.as_ref(),
                    note.as_ref(),
                    tags,
                ),
                forgotten,
            ),
            EventKind::EntryLogged {
                entry,
                end,
                project,
                note,
                tags,
            } => self.insert(
                new_entry(
                    recorded,
                    *entry,
                    Some(end),
                    project.as_ref(),
                    note.as_ref(),
                    tags,
                ),
                forgotten,
            ),
            EventKind::EntryHeld {
                entry,
                source,
                reason,
            } => {
                if let Some(entry) = self.open_entry(*entry) {
                    hold(entry, at, *source, reason.as_ref());
                }
            }
            EventKind::EntryPaused {
                entry,
                kind,
                reason,
            } => {
                if let Some(entry) = self.open_entry(*entry) {
                    pause(entry, at, *kind, reason.as_ref());
                }
            }
            EventKind::EntryResumed { entry } => {
                if let Some(entry) = self.open_entry(*entry) {
                    entry.close_breaks(at);
                }
            }
            EventKind::EntryEnded { entry } => {
                if let Some(entry) = self.entries.get_mut(entry) {
                    entry.close_breaks(at);
                    entry.end = Some(at.clone());
                }
            }
            EventKind::CatchNoted { entry, text } => {
                if let Some(entry) = self.entries.get_mut(entry) {
                    entry.catches.push(Catch {
                        at: at.clone(),
                        text: text.clone(),
                    });
                }
            }
            EventKind::EntryAdjusted { entry, changes } => {
                if let Some(entry) = self.entries.get_mut(entry) {
                    adjust(entry, changes);
                }
            }
            EventKind::EntryForgotten { .. } | EventKind::BatchUndone { .. } => {}
        }
    }

    fn insert(&mut self, entry: Entry, forgotten: &BTreeSet<EntryId>) {
        if !forgotten.contains(&entry.id) {
            _ = self.entries.entry(entry.id).or_insert(entry);
        }
    }

    fn open_entry(&mut self, id: EntryId) -> Option<&mut Entry> {
        self.entries.get_mut(&id).filter(|entry| entry.is_open())
    }

    /// An entry by its ID.
    #[must_use]
    pub fn entry(&self, id: EntryId) -> Option<&Entry> {
        self.entries.get(&id)
    }

    /// All entries, by start.
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        let mut entries: Vec<&Entry> = self.entries.values().collect();
        entries.sort_by(|a, b| a.start.cmp(&b.start).then(a.id.cmp(&b.id)));
        entries.into_iter()
    }

    /// The entries without an end: running, held or paused, by start.
    pub fn open(&self) -> impl Iterator<Item = &Entry> {
        self.entries().filter(|entry| entry.is_open())
    }
}

fn new_entry(
    recorded: &RecordedEvent,
    id: EntryId,
    end: Option<&ZonedDateTime>,
    project: Option<&ProjectRef>,
    note: Option<&String>,
    tags: &BTreeSet<Tag>,
) -> Entry {
    Entry {
        id,
        device: recorded.device,
        start: recorded.event.at.clone(),
        end: end.cloned(),
        project: project.cloned(),
        note: note.cloned(),
        tags: tags.clone(),
        holds: Vec::new(),
        pauses: Vec::new(),
        catches: Vec::new(),
        origin: if end.is_some() {
            EntryOrigin::Logged
        } else {
            EntryOrigin::Live
        },
        filed: recorded.event.about.clone(),
    }
}

/// A hold while held only updates the reason; a hold while paused turns the
/// pause into a hold from `at`.
fn hold(entry: &mut Entry, at: &ZonedDateTime, source: HoldSource, reason: Option<&String>) {
    if let Some(open) = entry.open_hold() {
        if let Some(reason) = reason {
            open.reason = Some(reason.clone());
        }
        return;
    }
    entry.close_breaks(at);
    entry.holds.push(Hold {
        start: at.clone(),
        end: None,
        source,
        reason: reason.cloned(),
    });
}

/// A pause while paused only updates the reason; a pause while held turns the
/// hold into a pause from `at`.
fn pause(entry: &mut Entry, at: &ZonedDateTime, kind: PauseKind, reason: Option<&String>) {
    if let Some(open) = entry.open_pause() {
        if let Some(reason) = reason {
            open.reason = Some(reason.clone());
        }
        return;
    }
    entry.close_breaks(at);
    entry.pauses.push(Pause {
        start: at.clone(),
        end: None,
        kind,
        reason: reason.cloned(),
    });
}

fn adjust(entry: &mut Entry, changes: &EntryChanges) {
    if let Some(start) = &changes.start {
        entry.start = start.clone();
    }
    if let Some(end) = &changes.end {
        entry.end = Some(end.clone());
    }
    match &changes.note {
        Change::Keep => {}
        Change::Set(note) => entry.note = Some(note.clone()),
        Change::Clear => entry.note = None,
    }
    match &changes.project {
        Change::Keep => {}
        Change::Set(project) => entry.project = Some(project.clone()),
        Change::Clear => entry.project = None,
    }
    if let Some(tags) = &changes.tags {
        entry.tags.clone_from(tags);
    }
}
