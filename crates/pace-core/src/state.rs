//! The state of the entries, built from the events in merged order.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use jiff::ZonedDateTime;

use crate::{
    BatchId, Billing, Catalog, Change, DeviceId, Entry, EntryChanges, EntryId, EntryOrigin,
    EventId, EventKind, Hlc, HoldSource, PauseKind, ProjectRef, RecordedEvent,
    entry::{EntryCatch, EntryHold, EntryPause},
    short_id::short_id_among,
};

/// The current state: the result of applying all events in merged order.
///
/// Building it never fails. Events in an undone batch count as never
/// written, a forgotten entry is gone, and an event about an entry that
/// doesn't exist, or that can't apply any more, changes nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    entries: BTreeMap<EntryId, Entry>,
    /// The billing of each billed entry; the first marking counts.
    billings: BTreeMap<EntryId, Billing>,
    /// The batches in merged order of their first event.
    batches: Vec<BatchRecord>,
    /// Where each batch is in `batches`, so an event finds its batch without
    /// a scan.
    batch_positions: HashMap<BatchId, usize>,
    /// The entries forgotten for good.
    forgotten: BTreeSet<EntryId>,
    /// The merge key of the last event applied, so a batch on top knows
    /// whether it comes after all of them.
    last: Option<MergeKey>,
}

/// Where an event stands in merged order; see [`RecordedEvent::merge_key`].
type MergeKey = (Hlc, DeviceId, EventId);

/// Whether a later batch took a batch back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Standing {
    /// No batch undid it: its events count.
    Kept,
    /// A batch undid it: its events count as never written.
    Undone,
}

/// A batch as far as `undo` cares.
#[derive(Debug, Clone, PartialEq, Eq)]
struct BatchRecord {
    id: BatchId,
    device: DeviceId,
    /// When its first event was recorded.
    recorded: Hlc,
    standing: Standing,
    /// Undo and forget can't be undone.
    undoable: bool,
    /// Whether its events are all `billing.marked`: the batch of
    /// `bill --mark-billed`, whose undo takes the marking back.
    marks_billing: bool,
    /// The entries its events concern; undoing a batch whose entries are all
    /// gone for good would change nothing.
    entries: BTreeSet<EntryId>,
    /// The moments its events are filed under, each once.
    abouts: Vec<ZonedDateTime>,
}

/// Two entries whose time overlaps, and the time they share.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overlap<'state> {
    /// The entry that starts first; of two with one start, the one with
    /// the lower ID.
    pub first: &'state Entry,
    /// The other entry.
    pub second: &'state Entry,
    /// When the shared time begins.
    pub start: ZonedDateTime,
    /// When the shared time ends.
    pub end: ZonedDateTime,
}

/// What `undo` reverts: a batch and the moments its events are filed under,
/// so the undo is filed with each of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoTarget {
    /// The batch.
    pub batch: BatchId,
    /// When its first event was recorded.
    pub recorded: Hlc,
    /// The moments its events are filed under.
    pub abouts: Vec<ZonedDateTime>,
    /// The entries its events concern.
    pub entries: BTreeSet<EntryId>,
    /// Whether its events are all `billing.marked`: an undo takes a
    /// marking back.
    pub marks_billing: bool,
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

        let mut state = Self {
            forgotten,
            last: ordered.last().map(|recorded| recorded.merge_key()),
            ..Self::default()
        };
        for recorded in ordered {
            let standing = if undone.contains(&recorded.batch) {
                Standing::Undone
            } else {
                Standing::Kept
            };
            state.record_batch(recorded, standing);
            if standing == Standing::Kept {
                state.apply(recorded);
            }
        }
        state
    }

    /// This state with each entry recorded with a project's key moved to
    /// the project of the record with that key, if `catalog` holds one:
    /// entries recorded before the project had its record count for it.
    #[must_use]
    pub fn resolve_projects(mut self, catalog: &Catalog) -> Self {
        for entry in self.entries.values_mut() {
            if let Some(project @ ProjectRef::Key(_)) = &entry.project
                && let Some(record) = catalog.resolve(project)
            {
                entry.project = Some(ProjectRef::Id(record.id));
            }
        }
        self
    }

    /// This state, built from the events of `read`, followed by `batch`:
    /// the state of all their events, as [`from_events`](Self::from_events)
    /// builds it.
    ///
    /// A batch whose events all merge after the read ones applies on top,
    /// and an undo builds only the entries of the batch it undoes again. A
    /// batch that merges between the read events, such as one from a device
    /// whose clock runs behind, builds the state from all events.
    #[must_use]
    pub fn followed_by(mut self, batch: &[RecordedEvent], read: &[RecordedEvent]) -> Self {
        let mut ordered: Vec<&RecordedEvent> = batch.iter().collect();
        ordered.sort_by_key(|recorded| recorded.merge_key());
        let on_top = match (self.last, ordered.first()) {
            (Some(last), Some(first)) => first.merge_key() > last,
            _ => true,
        };
        if !on_top {
            let all: Vec<RecordedEvent> = read.iter().chain(batch).cloned().collect();
            return Self::from_events(&all);
        }
        for recorded in ordered {
            self.record_batch(recorded, Standing::Kept);
            match recorded.event.kind {
                EventKind::BatchUndone { batch: undone } => {
                    self.undo(undone, read.iter().chain(batch));
                }
                EventKind::EntryForgotten { entry } => {
                    _ = self.forgotten.insert(entry);
                    _ = self.entries.remove(&entry);
                    _ = self.billings.remove(&entry);
                }
                _ => self.apply(recorded),
            }
            self.last = Some(recorded.merge_key());
        }
        self
    }

    /// Take back the batch `undone`: its events count as never written, so
    /// the entries they concern are built again from the `events` that
    /// still count, up to the last one applied.
    fn undo<'events>(
        &mut self,
        undone: BatchId,
        events: impl Iterator<Item = &'events RecordedEvent>,
    ) {
        let Some(record) = self
            .batch_positions
            .get(&undone)
            .and_then(|&position| self.batches.get_mut(position))
        else {
            return;
        };
        if record.standing == Standing::Undone {
            return;
        }
        record.standing = Standing::Undone;
        let concerned = record.entries.clone();
        for id in &concerned {
            _ = self.entries.remove(id);
            _ = self.billings.remove(id);
        }
        let mut again: Vec<&RecordedEvent> = events
            .filter(|recorded| {
                recorded
                    .event
                    .kind
                    .entry()
                    .is_some_and(|id| concerned.contains(&id))
                    && self.last.is_none_or(|last| recorded.merge_key() <= last)
                    && self.standing(recorded.batch) == Some(Standing::Kept)
            })
            .collect();
        again.sort_by_key(|recorded| recorded.merge_key());
        for recorded in again {
            self.apply(recorded);
        }
    }

    /// Whether the batch `id` still counts, if the state knows it.
    fn standing(&self, id: BatchId) -> Option<Standing> {
        self.batch_positions
            .get(&id)
            .and_then(|&position| self.batches.get(position))
            .map(|batch| batch.standing)
    }

    fn record_batch(&mut self, recorded: &RecordedEvent, standing: Standing) {
        let undoable = !matches!(
            recorded.event.kind,
            EventKind::BatchUndone { .. } | EventKind::EntryForgotten { .. }
        );
        let marks_billing = matches!(recorded.event.kind, EventKind::BillingMarked { .. });
        let entry = recorded.event.kind.entry();
        let about = &recorded.event.about;
        if let Some(batch) = self
            .batch_positions
            .get(&recorded.batch)
            .and_then(|&position| self.batches.get_mut(position))
        {
            batch.undoable &= undoable;
            batch.marks_billing &= marks_billing;
            batch.entries.extend(entry);
            if !batch.abouts.contains(about) {
                batch.abouts.push(about.clone());
            }
        } else {
            _ = self
                .batch_positions
                .insert(recorded.batch, self.batches.len());
            self.batches.push(BatchRecord {
                id: recorded.batch,
                device: recorded.device,
                recorded: recorded.hlc,
                standing,
                undoable,
                marks_billing,
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
                    && batch.standing == Standing::Kept
                    && batch.entries.iter().any(|id| self.entries.contains_key(id))
            })
            .map(|batch| UndoTarget {
                batch: batch.id,
                recorded: batch.recorded,
                abouts: batch.abouts.clone(),
                entries: batch.entries.clone(),
                marks_billing: batch.marks_billing,
            })
    }

    /// The entries other than `except` whose time overlaps `[start, end)`;
    /// an open entry lasts until `now`.
    pub fn overlapping<'state>(
        &'state self,
        start: &ZonedDateTime,
        end: &ZonedDateTime,
        except: &[EntryId],
        now: &ZonedDateTime,
    ) -> impl Iterator<Item = &'state Entry> {
        let (start, end, now) = (start.clone(), end.clone(), now.clone());
        let except = except.to_vec();
        self.entries().filter(move |entry| {
            !except.contains(&entry.id) && entry.start < end && start < *entry.until(&now)
        })
    }

    /// Every pair of entries whose time overlaps, with the time they
    /// share; an open entry lasts until `now`. The pairs come by the start
    /// of their first entry, then of their second.
    #[must_use]
    pub fn overlaps(&self, now: &ZonedDateTime) -> Vec<Overlap<'_>> {
        let entries: Vec<&Entry> = self.entries().collect();
        let mut overlaps = Vec::new();
        for (index, first) in entries.iter().enumerate() {
            let first_end = first.until(now);
            // The entries come by start: once one begins at or after the
            // end of `first`, no later one can overlap it.
            for second in entries[index + 1..]
                .iter()
                .take_while(|second| second.start < *first_end)
            {
                let start = (&first.start).max(&second.start);
                let end = first_end.min(second.until(now));
                if start < end {
                    overlaps.push(Overlap {
                        first,
                        second,
                        start: start.clone(),
                        end: end.clone(),
                    });
                }
            }
        }
        overlaps
    }

    fn apply(&mut self, recorded: &RecordedEvent) {
        let at = &recorded.event.at;
        match &recorded.event.kind {
            EventKind::EntryBegun { .. } | EventKind::EntryLogged { .. } => {
                if let Some(entry) = new_entry(recorded) {
                    self.insert(entry);
                }
            }
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
                    entry.catches.push(EntryCatch {
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
            EventKind::BillingMarked { entry, billing } => {
                if self.entries.contains_key(entry) {
                    _ = self
                        .billings
                        .entry(*entry)
                        .or_insert_with(|| billing.clone());
                }
            }
            EventKind::EntryForgotten { .. } | EventKind::BatchUndone { .. } => {}
        }
    }

    fn insert(&mut self, entry: Entry) {
        if !self.forgotten.contains(&entry.id) {
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

    /// Every billed entry with its billing, by entry ID.
    pub fn billings(&self) -> impl Iterator<Item = (EntryId, &Billing)> {
        self.billings.iter().map(|(id, billing)| (*id, billing))
    }

    /// The billing of the entry `id`, if it is billed: the invoice and the
    /// values its `billing.marked` event froze. Of two markings, the first
    /// in merged order counts.
    #[must_use]
    pub fn billing(&self, id: EntryId) -> Option<&Billing> {
        self.billings.get(&id)
    }

    /// The short ID of the entry `id`: its last hex characters, at least
    /// four, that no other entry of the state ends with. An ID that names no
    /// entry of the state is shown whole.
    #[must_use]
    pub fn short_id(&self, id: EntryId) -> String {
        if self.entries.contains_key(&id) {
            short_id_among(id, self.entries.keys().copied())
        } else {
            id.to_string()
        }
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

/// The entry an `entry.begun` or `entry.logged` event creates; `None` for
/// any other event.
fn new_entry(recorded: &RecordedEvent) -> Option<Entry> {
    let kind = &recorded.event.kind;
    let (EventKind::EntryBegun {
        entry,
        project,
        note,
        tags,
        role,
        determined_by,
        billable,
        allow_overlap,
    }
    | EventKind::EntryLogged {
        entry,
        project,
        note,
        tags,
        role,
        determined_by,
        billable,
        allow_overlap,
        ..
    }) = kind
    else {
        return None;
    };
    let end = match kind {
        EventKind::EntryLogged { end, .. } => Some(end.clone()),
        _ => None,
    };
    Some(Entry {
        id: *entry,
        device: recorded.device,
        start: recorded.event.at.clone(),
        origin: if end.is_some() {
            EntryOrigin::Logged
        } else {
            EntryOrigin::Live
        },
        end,
        project: project.clone(),
        note: note.clone(),
        tags: tags.clone(),
        role: *role,
        determined_by: *determined_by,
        billable: *billable,
        allow_overlap: *allow_overlap,
        holds: Vec::new(),
        pauses: Vec::new(),
        catches: Vec::new(),
        filed: recorded.event.about.clone(),
    })
}

/// A hold while held only updates the reason; a hold while paused turns the
/// pause into a hold from `at`.
fn hold(entry: &mut Entry, at: &ZonedDateTime, source: HoldSource, reason: Option<&String>) {
    if let Some(open) = entry.open_hold_mut() {
        if let Some(reason) = reason {
            open.reason = Some(reason.clone());
        }
        return;
    }
    entry.close_breaks(at);
    entry.holds.push(EntryHold {
        start: at.clone(),
        end: None,
        source,
        reason: reason.cloned(),
    });
}

/// A pause while paused only updates the reason; a pause while held turns the
/// hold into a pause from `at`.
fn pause(entry: &mut Entry, at: &ZonedDateTime, kind: PauseKind, reason: Option<&String>) {
    if let Some(open) = entry.open_pause_mut() {
        if let Some(reason) = reason {
            open.reason = Some(reason.clone());
        }
        return;
    }
    entry.close_breaks(at);
    entry.pauses.push(EntryPause {
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
    apply_change(&mut entry.note, &changes.note);
    apply_change(&mut entry.project, &changes.project);
    if let Some(tags) = &changes.tags {
        entry.tags.clone_from(tags);
    }
    apply_change(&mut entry.role, &changes.role);
    apply_change(&mut entry.determined_by, &changes.determined_by);
    apply_change(&mut entry.billable, &changes.billable);
    entry.allow_overlap |= changes.allow_overlap;
}

/// Keep, set or clear one field of an entry.
fn apply_change<T: Clone>(field: &mut Option<T>, change: &Change<T>) {
    match change {
        Change::Keep => {}
        Change::Set(value) => *field = Some(value.clone()),
        Change::Clear => *field = None,
    }
}
