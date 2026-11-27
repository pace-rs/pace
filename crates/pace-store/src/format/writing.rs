//! An event written as its `[[event]]` record.

use std::collections::BTreeSet;

use pace_core::{Change, EntryChanges, EventKind, ProjectRef, RecordedEvent, Tag};

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
        source: None,
        pause_kind: None,
        reason: None,
        text: None,
        start: None,
        end: None,
        cleared: None,
        undoes: None,
    };
    let non_empty = |tags: &BTreeSet<Tag>| (!tags.is_empty()).then(|| tags.clone());
    match &event.kind {
        EventKind::EntryBegun {
            entry,
            project,
            note,
            tags,
        } => {
            record.entry = Some(*entry);
            set_project(&mut record, project.as_ref());
            record.note.clone_from(note);
            record.tags = non_empty(tags);
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
        EventKind::EntryLogged {
            entry,
            end,
            project,
            note,
            tags,
        } => {
            record.entry = Some(*entry);
            record.end = Some(end.clone());
            set_project(&mut record, project.as_ref());
            record.note.clone_from(note);
            record.tags = non_empty(tags);
        }
        EventKind::EntryAdjusted { entry, changes } => {
            record.entry = Some(*entry);
            adjusted_record(&mut record, changes);
        }
        EventKind::BatchUndone { batch } => record.undoes = Some(*batch),
    }
    record
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
    record.cleared = (!cleared.is_empty()).then_some(cleared);
}
