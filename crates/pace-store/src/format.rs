//! The TOML format of event files.
//!
//! Each event is one `[[event]]` table with its envelope (`id`, `type`, `v`,
//! `batch`, `at`, `about` if it differs from `at`, `hlc`) and the fields of
//! its type. pace reads strictly: an unknown key, a missing or foreign field,
//! an unknown type or version stops it with the file and the line.

use std::{collections::BTreeSet, ops::Range, path::Path, str::FromStr};

use jiff::{Timestamp, ZonedDateTime};
use pace_core::{
    BatchId, Change, DeviceId, EntryChanges, EntryId, Event, EventId, EventKind, Hlc, HoldSource,
    PauseKind, ProjectId, ProjectKey, ProjectRef, RecordedEvent, Tag,
};
use pace_error::{ErrorKind, PaceError, PaceResult};
use serde::{Deserialize, Serialize};

use crate::toml_file::{TomlFile, closest, in_file_order};

/// The version of the event file format.
pub const FILE_VERSION: u32 = 1;

/// The version of every event type this pace writes.
const EVENT_VERSION: u32 = 1;

/// One `[[event]]` table. The field order is the order in the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct Record {
    id: EventId,
    #[serde(rename = "type")]
    kind: String,
    v: u32,
    batch: BatchId,
    at: ZonedDateTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    about: Option<ZonedDateTime>,
    hlc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    entry: Option<EntryId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    project: Option<ProjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    project_key: Option<ProjectKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tags: Option<BTreeSet<Tag>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source: Option<HoldSource>,
    #[serde(default, rename = "kind", skip_serializing_if = "Option::is_none")]
    pause_kind: Option<PauseKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    start: Option<ZonedDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    end: Option<ZonedDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cleared: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    undoes: Option<BatchId>,
}

/// The keys an `[[event]]` table may have.
const KEYS: [&str; 22] = [
    "id",
    "type",
    "v",
    "batch",
    "at",
    "about",
    "hlc",
    "entry",
    "project",
    "project-key",
    "note",
    "tags",
    "source",
    "kind",
    "reason",
    "text",
    "start",
    "end",
    "cleared",
    "undoes",
    // Not keys of an event, but known in the file.
    "version",
    "event",
];

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    version: u32,
    #[serde(default)]
    event: Vec<Record>,
}

#[derive(Debug, Serialize)]
struct Chunk<'a> {
    event: &'a [Record],
}

/// The header of a new event file.
#[must_use]
pub fn header() -> String {
    format!("version = {FILE_VERSION}\n")
}

/// The text to append to an event file for `events`.
///
/// # Errors
///
/// If an event cannot be written as TOML, which is a bug.
pub fn serialize(events: &[RecordedEvent]) -> PaceResult<String> {
    let records: Vec<Record> = events.iter().map(record).collect();
    toml::to_string(&Chunk { event: &records }).map_err(|error| {
        PaceError::with_source(ErrorKind::Internal, "cannot write an event as TOML", error)
            .ask_report()
    })
}

/// The events of an event file of `device`.
///
/// # Errors
///
/// If the file is no valid event file; the error names the file and the line.
pub fn parse(path: &Path, text: &str, device: DeviceId) -> PaceResult<Vec<RecordedEvent>> {
    let file: File = match toml::from_str(text) {
        Ok(file) => file,
        Err(error) => return Err(explain(path, text, &error)),
    };
    if file.version != FILE_VERSION {
        return Err(newer_or_broken(path, text, "version", file.version));
    }
    file.event
        .into_iter()
        .enumerate()
        .map(|(index, record)| {
            recorded(record, device).map_err(|invalid| {
                let file = event_file(path, text);
                let span = event_span(text, index).unwrap_or(0..0);
                file.error_at(span, invalid.message, invalid.hint, "this event")
                    .attach_error_code("pace::store::invalid-event")
            })
        })
        .collect()
}

fn event_file<'a>(path: &'a Path, text: &'a str) -> TomlFile<'a> {
    TomlFile {
        path,
        text,
        what: "an event file",
        noun: "key",
        code: "pace::store",
        version: i64::from(FILE_VERSION),
    }
}

const READ_ONLY: &str = "event files are for reading; change an entry with `pace edit`";

/// A precise error for a file that serde rejected: the unknown key, the
/// broken TOML, or the value at fault.
fn explain(path: &Path, text: &str, error: &toml::de::Error) -> Box<PaceError> {
    let file = event_file(path, text);
    if let Ok(table) = file.parse() {
        for (key, value) in in_file_order(table.get_ref()) {
            if !KEYS.contains(&key.get_ref().as_ref()) {
                return unknown_key(&file, key.get_ref(), key.span());
            }
            if let Some(events) = value.get_ref().as_array() {
                for event in events {
                    let Some(event) = event.get_ref().as_table() else {
                        continue;
                    };
                    for (key, _) in in_file_order(event) {
                        if !KEYS.contains(&key.get_ref().as_ref()) {
                            return unknown_key(&file, key.get_ref(), key.span());
                        }
                    }
                }
            }
        }
    }
    file.error_at(
        error.span().unwrap_or(0..0),
        format!("{} has an event pace cannot read", path.display()),
        READ_ONLY,
        error.message().trim(),
    )
    .attach_error_code("pace::store::invalid-event")
}

fn unknown_key(file: &TomlFile<'_>, name: &str, span: Range<usize>) -> Box<PaceError> {
    let hint = closest(name, KEYS.iter().copied()).map_or_else(
        || READ_ONLY.to_owned(),
        |known| format!("did you mean `{known}`? {READ_ONLY}"),
    );
    file.error_at(
        span,
        format!("`{name}` is not a key pace knows in an event"),
        hint,
        "unknown key",
    )
    .attach_error_code("pace::store::unknown-key")
}

fn newer_or_broken(path: &Path, text: &str, key: &str, version: u32) -> Box<PaceError> {
    let file = event_file(path, text);
    let span = text
        .find(key)
        .map_or(0..0, |start| start..start + key.len());
    let hint = if version > FILE_VERSION {
        "a newer pace may have written it on another device; update pace"
    } else {
        READ_ONLY
    };
    file.error_at(
        span,
        format!("this pace reads version {FILE_VERSION} of event files, not {version}"),
        hint,
        "this version",
    )
    .attach_error_code("pace::store::version")
}

/// The span of the header of the `index`-th `[[event]]` table.
fn event_span(text: &str, index: usize) -> Option<Range<usize>> {
    let mut offset = 0;
    let mut seen = 0;
    for line in text.split_inclusive('\n') {
        if line.trim() == "[[event]]" {
            if seen == index {
                return Some(offset..offset + line.trim_end().len());
            }
            seen += 1;
        }
        offset += line.len();
    }
    None
}

/// Why a record is no valid event.
struct Invalid {
    message: String,
    hint: String,
}

impl Invalid {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            hint: READ_ONLY.to_owned(),
        }
    }
}

fn hlc_text(hlc: Hlc) -> String {
    format!(
        "{}/{}",
        hlc.wall().strftime("%Y-%m-%dT%H:%M:%S%.3fZ"),
        hlc.counter()
    )
}

fn parse_hlc(text: &str) -> Option<Hlc> {
    let (wall, counter) = text.split_once('/')?;
    Some(Hlc::new(
        Timestamp::from_str(wall).ok()?,
        counter.parse().ok()?,
    ))
}

fn record(recorded: &RecordedEvent) -> Record {
    let event = &recorded.event;
    let mut record = Record {
        id: event.id,
        kind: type_name(&event.kind).to_owned(),
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
        _ => {}
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

const fn type_name(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::EntryBegun { .. } => "entry.begun",
        EventKind::EntryHeld { .. } => "entry.held",
        EventKind::EntryPaused { .. } => "entry.paused",
        EventKind::EntryResumed { .. } => "entry.resumed",
        EventKind::EntryEnded { .. } => "entry.ended",
        EventKind::CatchNoted { .. } => "catch.noted",
        EventKind::EntryLogged { .. } => "entry.logged",
        EventKind::EntryAdjusted { .. } => "entry.adjusted",
        EventKind::EntryForgotten { .. } => "entry.forgotten",
        EventKind::BatchUndone { .. } => "batch.undone",
        _ => "unknown",
    }
}

/// The fields of a record besides the envelope, by their names in the file.
fn present(record: &Record) -> Vec<&'static str> {
    [
        ("entry", record.entry.is_some()),
        ("project", record.project.is_some()),
        ("project-key", record.project_key.is_some()),
        ("note", record.note.is_some()),
        ("tags", record.tags.is_some()),
        ("source", record.source.is_some()),
        ("kind", record.pause_kind.is_some()),
        ("reason", record.reason.is_some()),
        ("text", record.text.is_some()),
        ("start", record.start.is_some()),
        ("end", record.end.is_some()),
        ("cleared", record.cleared.is_some()),
        ("undoes", record.undoes.is_some()),
    ]
    .into_iter()
    .filter_map(|(name, present)| present.then_some(name))
    .collect()
}

/// The fields each type allows besides the envelope.
fn allowed(kind: &str) -> Option<&'static [&'static str]> {
    Some(match kind {
        "entry.begun" => &["entry", "project", "project-key", "note", "tags"],
        "entry.held" => &["entry", "source", "reason"],
        "entry.paused" => &["entry", "kind", "reason"],
        "entry.resumed" | "entry.ended" | "entry.forgotten" => &["entry"],
        "catch.noted" => &["entry", "text"],
        "entry.logged" => &["entry", "end", "project", "project-key", "note", "tags"],
        "entry.adjusted" => &[
            "entry",
            "start",
            "end",
            "note",
            "project",
            "project-key",
            "tags",
            "cleared",
        ],
        "batch.undone" => &["undoes"],
        _ => return None,
    })
}

fn recorded(record: Record, device: DeviceId) -> Result<RecordedEvent, Invalid> {
    let kind = record.kind.clone();
    let allowed = allowed(&kind).ok_or_else(|| Invalid {
        message: format!("pace doesn't know the event type `{kind}`"),
        hint: "a newer pace may have written it on another device; update pace".to_owned(),
    })?;
    if record.v != EVENT_VERSION {
        return Err(Invalid {
            message: format!(
                "pace reads version {EVENT_VERSION} of `{kind}`, not {}",
                record.v
            ),
            hint: if record.v > EVENT_VERSION {
                "a newer pace may have written it on another device; update pace".to_owned()
            } else {
                READ_ONLY.to_owned()
            },
        });
    }
    if let Some(foreign) = present(&record)
        .into_iter()
        .find(|field| !allowed.contains(field))
    {
        return Err(Invalid::new(format!(
            "an `{kind}` event has no `{foreign}`"
        )));
    }
    if record.project.is_some() && record.project_key.is_some() {
        return Err(Invalid::new(format!(
            "an `{kind}` event has `project` and `project-key`; it needs one"
        )));
    }
    let hlc = parse_hlc(&record.hlc).ok_or_else(|| {
        Invalid::new(format!(
            "`{}` is no HLC like `2026-11-26T08:02:11.204Z/0`",
            record.hlc
        ))
    })?;

    let needs = |field: &str| Invalid::new(format!("an `{kind}` event needs `{field}`"));
    let entry = record.entry.ok_or_else(|| needs("entry"));
    let project = record
        .project
        .map(ProjectRef::Id)
        .or_else(|| record.project_key.clone().map(ProjectRef::Key));
    let tags = record.tags.clone().unwrap_or_default();

    let event_kind = match kind.as_str() {
        "entry.begun" => EventKind::EntryBegun {
            entry: entry?,
            project,
            note: record.note.clone(),
            tags,
        },
        "entry.held" => EventKind::EntryHeld {
            entry: entry?,
            source: record.source.ok_or_else(|| needs("source"))?,
            reason: record.reason.clone(),
        },
        "entry.paused" => EventKind::EntryPaused {
            entry: entry?,
            kind: record.pause_kind.ok_or_else(|| needs("kind"))?,
            reason: record.reason.clone(),
        },
        "entry.resumed" => EventKind::EntryResumed { entry: entry? },
        "entry.ended" => EventKind::EntryEnded { entry: entry? },
        "entry.forgotten" => EventKind::EntryForgotten { entry: entry? },
        "catch.noted" => EventKind::CatchNoted {
            entry: entry?,
            text: record.text.clone().ok_or_else(|| needs("text"))?,
        },
        "entry.logged" => EventKind::EntryLogged {
            entry: entry?,
            end: record.end.clone().ok_or_else(|| needs("end"))?,
            project,
            note: record.note.clone(),
            tags,
        },
        "entry.adjusted" => EventKind::EntryAdjusted {
            entry: entry?,
            changes: adjusted(&record, project)?,
        },
        _ => EventKind::BatchUndone {
            batch: record.undoes.ok_or_else(|| needs("undoes"))?,
        },
    };

    Ok(RecordedEvent {
        device,
        batch: record.batch,
        hlc,
        event: Event {
            id: record.id,
            about: record.about.unwrap_or_else(|| record.at.clone()),
            at: record.at,
            kind: event_kind,
        },
    })
}

fn adjusted(record: &Record, project: Option<ProjectRef>) -> Result<EntryChanges, Invalid> {
    let cleared = record.cleared.clone().unwrap_or_default();
    if let Some(unknown) = cleared
        .iter()
        .find(|field| !["note", "project"].contains(&field.as_str()))
    {
        return Err(Invalid::new(format!(
            "`cleared` names `{unknown}`; only `note` and `project` can be cleared"
        )));
    }
    Ok(EntryChanges {
        start: record.start.clone(),
        end: record.end.clone(),
        note: change("note", record.note.clone(), &cleared)?,
        project: change("project", project, &cleared)?,
        tags: record.tags.clone(),
    })
}

fn change<T>(field: &str, value: Option<T>, cleared: &[String]) -> Result<Change<T>, Invalid> {
    match (value, cleared.iter().any(|name| name == field)) {
        (Some(_), true) => Err(Invalid::new(format!(
            "`{field}` is set and cleared at once"
        ))),
        (Some(value), false) => Ok(Change::Set(value)),
        (None, true) => Ok(Change::Clear),
        (None, false) => Ok(Change::Keep),
    }
}
