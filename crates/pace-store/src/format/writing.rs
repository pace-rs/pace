//! An event written as its `[[event]]` record.

use pace_core::{Change, EntryChanges, EventKind, ProjectRef, RecordedEvent};

use crate::format::{EVENT_VERSION, EventType, Record};
use crate::hlc::hlc_text;

pub(super) fn record_from_event(recorded: &RecordedEvent) -> Record {
    let event = &recorded.event;
    let mut record = Record {
        id: event.id,
        kind: EventType::of(&event.kind).name().to_owned(),
        v: EVENT_VERSION,
        batch: recorded.batch,
        at: event.at.clone(),
        about: (event.about != event.at).then(|| event.about.clone()),
        hlc: hlc_text(recorded.hlc),
        entry: None,
        project: None,
        project_key: None,
        note: None,
        tags: None,
        role: None,
        determined_by: None,
        billable: None,
        allow_overlap: None,
        source: None,
        pause_kind: None,
        reason: None,
        text: None,
        start: None,
        end: None,
        cleared: None,
        undoes: None,
    };
    match &event.kind {
        EventKind::EntryBegun { .. } | EventKind::EntryLogged { .. } => {
            new_entry_record(&mut record, &event.kind);
        }
        EventKind::EntryHeld {
            entry,
            source,
            reason,
        } => {
            record.entry = Some(*entry);
            record.source = Some(*source);
            record.reason.clone_from(reason);
        }
        EventKind::EntryPaused {
            entry,
            kind,
            reason,
        } => {
            record.entry = Some(*entry);
            record.pause_kind = Some(*kind);
            record.reason.clone_from(reason);
        }
        EventKind::EntryResumed { entry }
        | EventKind::EntryEnded { entry }
        | EventKind::EntryForgotten { entry } => record.entry = Some(*entry),
        EventKind::CatchNoted { entry, text } => {
            record.entry = Some(*entry);
            record.text = Some(text.clone());
        }
        EventKind::EntryAdjusted { entry, changes } => {
            record.entry = Some(*entry);
            adjusted_record(&mut record, changes);
        }
        EventKind::BatchUndone { batch } => record.undoes = Some(*batch),
    }
    record
}

/// The fields of an `entry.begun` or an `entry.logged` event; the four
/// fields of phase 3 only when the event has them.
fn new_entry_record(record: &mut Record, kind: &EventKind) {
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
        return;
    };
    if let EventKind::EntryLogged { end, .. } = kind {
        record.end = Some(end.clone());
    }
    record.entry = Some(*entry);
    set_project(record, project.as_ref());
    record.note.clone_from(note);
    record.tags = (!tags.is_empty()).then(|| tags.clone());
    record.role = *role;
    record.determined_by = *determined_by;
    record.billable = *billable;
    record.allow_overlap = allow_overlap.then_some(true);
}

fn set_project(record: &mut Record, project: Option<&ProjectRef>) {
    match project {
        Some(ProjectRef::Id(id)) => record.project = Some(*id),
        Some(ProjectRef::Key(key)) => record.project_key = Some(key.clone()),
        None => {}
    }
}

/// The fields of an `entry.adjusted` event; emptied fields go to `cleared`.
fn adjusted_record(record: &mut Record, changes: &EntryChanges) {
    record.start.clone_from(&changes.start);
    record.end.clone_from(&changes.end);
    let mut cleared = Vec::new();
    match &changes.note {
        Change::Set(note) => record.note = Some(note.clone()),
        Change::Clear => cleared.push("note".to_owned()),
        Change::Keep => {}
    }
    match &changes.project {
        Change::Set(project) => set_project(record, Some(project)),
        Change::Clear => cleared.push("project".to_owned()),
        Change::Keep => {}
    }
    record.tags.clone_from(&changes.tags);
    match changes.role {
        Change::Set(role) => record.role = Some(role),
        Change::Clear => cleared.push("role".to_owned()),
        Change::Keep => {}
    }
    match changes.determined_by {
        Change::Set(by) => record.determined_by = Some(by),
        Change::Clear => cleared.push("determined-by".to_owned()),
        Change::Keep => {}
    }
    match changes.billable {
        Change::Set(billable) => record.billable = Some(billable),
        Change::Clear => cleared.push("billable".to_owned()),
        Change::Keep => {}
    }
    record.allow_overlap = changes.allow_overlap.then_some(true);
    record.cleared = (!cleared.is_empty()).then_some(cleared);
}
