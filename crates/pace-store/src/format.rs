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
use toml::Spanned;

use crate::hlc::hlc_text;
use crate::toml_file::{TomlCause, TomlFile, closest, in_file_order};

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
    /// Each event with its span in the file, to point at it.
    #[serde(default)]
    event: Vec<Spanned<Record>>,
}

#[derive(Debug, Serialize)]
struct Chunk<'a> {
    event: &'a [Record],
}

/// The header of a new event file.
#[must_use]
pub fn file_header() -> String {
    format!("version = {FILE_VERSION}\n")
}

/// The text to append to an event file for `events`.
///
/// # Errors
///
/// If an event cannot be written as TOML, which is a bug.
pub fn serialize_events(events: &[RecordedEvent]) -> PaceResult<String> {
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
pub fn parse_events(path: &Path, text: &str, device: DeviceId) -> PaceResult<Vec<RecordedEvent>> {
    let file: File = match toml::from_str(text) {
        Ok(file) => file,
        Err(error) => return Err(explain(path, text, error)),
    };
    if file.version != FILE_VERSION {
        return Err(newer_or_broken(path, text, "version", file.version));
    }
    file.event
        .into_iter()
        .map(|record| {
            let span = record.span();
            recorded(record.into_inner(), device).map_err(|invalid| {
                event_file(path, text)
                    .error_at(
                        span,
                        invalid.guidance,
                        invalid.context,
                        invalid.hint,
                        "this event",
                    )
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
        kind: ErrorKind::InputOutput,
        version: i64::from(FILE_VERSION),
    }
}

const READ_ONLY: &str = "event files are for reading; change an entry with `pace edit`";

/// How to go on from an event file that is no valid TOML or misses its
/// `version`: `pace edit` reads the same file, so it can't help.
const REPAIR: &str = "repair the file, or restore it from a backup";

/// A precise error for a file that serde rejected: the broken TOML, the
/// unknown key, or the value at fault. The TOML error is its cause.
fn explain(path: &Path, text: &str, error: toml::de::Error) -> Box<PaceError> {
    let file = event_file(path, text);
    let Ok(table) = file.parse() else {
        return broken(&file, error, "the event file {file} is not valid TOML")
            .attach_error_code("pace::store::syntax");
    };
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
    broken(&file, error, "pace cannot read the event file {file}")
        .attach_error_code("pace::store::invalid-file")
}

/// The error `guidance` about the event file `file`, caused by `error`.
fn broken(file: &TomlFile<'_>, error: toml::de::Error, guidance: &'static str) -> Box<PaceError> {
    let span = error.span().unwrap_or(0..0);
    let label = error.message().trim().to_owned();
    let error = PaceError::with_source(file.kind, guidance, TomlCause(error))
        .attach_context("file", file.path.display().to_string())
        .attach_hint(REPAIR);
    file.point_at(error, span, &label)
}

fn unknown_key(file: &TomlFile<'_>, name: &str, span: Range<usize>) -> Box<PaceError> {
    let hint = closest(name, KEYS.iter().copied()).map_or_else(
        || READ_ONLY.to_owned(),
        |known| format!("did you mean `{known}`? {READ_ONLY}"),
    );
    file.error_at(
        span,
        "`{key}` is not a key pace knows in an event",
        [("key", name.to_owned())],
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
        NEWER
    } else {
        READ_ONLY
    };
    file.error_at(
        span,
        "this pace reads version {reads} of event files, not {found}",
        [
            ("reads", FILE_VERSION.to_string()),
            ("found", version.to_string()),
        ],
        hint,
        "this version",
    )
    .attach_error_code("pace::store::version")
}

/// Why a record is no valid event: a guidance with its context, and how to
/// go on.
struct Invalid {
    guidance: &'static str,
    context: Vec<(&'static str, String)>,
    hint: &'static str,
}

impl Invalid {
    /// The record is wrong; event files are for reading.
    const fn new(guidance: &'static str) -> Self {
        Self {
            guidance,
            context: Vec::new(),
            hint: READ_ONLY,
        }
    }

    /// A newer pace may know the record.
    fn newer(guidance: &'static str) -> Self {
        Self {
            hint: NEWER,
            ..Self::new(guidance)
        }
    }

    fn with(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.context.push((key, value.into()));
        self
    }
}

const NEWER: &str = "a newer pace may have written it on another device; update pace";

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

/// The type of an event as the log names it: one per [`EventKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventType {
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

    const fn of(kind: &EventKind) -> Self {
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
    const fn name(self) -> &'static str {
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
    fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|event_type| event_type.name() == name)
    }

    /// The fields the type allows besides the envelope.
    const fn allowed(self) -> &'static [&'static str] {
        match self {
            Self::EntryBegun => &["entry", "project", "project-key", "note", "tags"],
            Self::EntryHeld => &["entry", "source", "reason"],
            Self::EntryPaused => &["entry", "kind", "reason"],
            Self::EntryResumed | Self::EntryEnded | Self::EntryForgotten => &["entry"],
            Self::CatchNoted => &["entry", "text"],
            Self::EntryLogged => &["entry", "end", "project", "project-key", "note", "tags"],
            Self::EntryAdjusted => &[
                "entry",
                "start",
                "end",
                "note",
                "project",
                "project-key",
                "tags",
                "cleared",
            ],
            Self::BatchUndone => &["undoes"],
        }
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

/// The event a record holds: its type parsed and its version and fields
/// checked ([`checked_type`]), its HLC parsed and its fields turned into an
/// [`EventKind`] ([`event_kind`]).
fn recorded(record: Record, device: DeviceId) -> Result<RecordedEvent, Invalid> {
    let event_type = checked_type(&record)?;
    let hlc = parse_hlc(&record.hlc).ok_or_else(|| {
        Invalid::new("`{hlc}` is no HLC like `2026-11-26T08:02:11.204Z/0`").with("hlc", &record.hlc)
    })?;

    let event_kind = event_kind(&record, event_type)?;

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

/// The type of a record of a type and version pace knows, with only the
/// fields of its type, and a project either by ID or by key.
fn checked_type(record: &Record) -> Result<EventType, Invalid> {
    let kind = &record.kind;
    let event_type = EventType::parse(kind).ok_or_else(|| {
        Invalid::newer("pace doesn't know the event type `{type}`").with("type", kind)
    })?;
    let allowed = event_type.allowed();
    if record.v != EVENT_VERSION {
        let guidance = "pace reads version {reads} of `{type}`, not {found}";
        let invalid = if record.v > EVENT_VERSION {
            Invalid::newer(guidance)
        } else {
            Invalid::new(guidance)
        };
        return Err(invalid
            .with("reads", EVENT_VERSION.to_string())
            .with("type", kind)
            .with("found", record.v.to_string()));
    }
    if let Some(foreign) = present(record)
        .into_iter()
        .find(|field| !allowed.contains(field))
    {
        return Err(Invalid::new("an event of type `{type}` has no `{field}`")
            .with("type", kind)
            .with("field", foreign));
    }
    if record.project.is_some() && record.project_key.is_some() {
        return Err(Invalid::new(
            "an event of type `{type}` has `project` and `project-key`; it needs one",
        )
        .with("type", kind));
    }
    Ok(event_type)
}

/// The event of a checked record of `event_type`, or the field its type
/// needs and misses.
fn event_kind(record: &Record, event_type: EventType) -> Result<EventKind, Invalid> {
    let kind = &record.kind;
    let needs = |field: &str| {
        Invalid::new("an event of type `{type}` needs `{field}`")
            .with("type", kind)
            .with("field", field)
    };
    let entry = record.entry.ok_or_else(|| needs("entry"));
    let project = record
        .project
        .map(ProjectRef::Id)
        .or_else(|| record.project_key.clone().map(ProjectRef::Key));
    let tags = record.tags.clone().unwrap_or_default();

    Ok(match event_type {
        EventType::EntryBegun => EventKind::EntryBegun {
            entry: entry?,
            project,
            note: record.note.clone(),
            tags,
        },
        EventType::EntryHeld => EventKind::EntryHeld {
            entry: entry?,
            source: record.source.ok_or_else(|| needs("source"))?,
            reason: record.reason.clone(),
        },
        EventType::EntryPaused => EventKind::EntryPaused {
            entry: entry?,
            kind: record.pause_kind.ok_or_else(|| needs("kind"))?,
            reason: record.reason.clone(),
        },
        EventType::EntryResumed => EventKind::EntryResumed { entry: entry? },
        EventType::EntryEnded => EventKind::EntryEnded { entry: entry? },
        EventType::EntryForgotten => EventKind::EntryForgotten { entry: entry? },
        EventType::CatchNoted => EventKind::CatchNoted {
            entry: entry?,
            text: record.text.clone().ok_or_else(|| needs("text"))?,
        },
        EventType::EntryLogged => EventKind::EntryLogged {
            entry: entry?,
            end: record.end.clone().ok_or_else(|| needs("end"))?,
            project,
            note: record.note.clone(),
            tags,
        },
        EventType::EntryAdjusted => EventKind::EntryAdjusted {
            entry: entry?,
            changes: adjusted(record, project)?,
        },
        EventType::BatchUndone => EventKind::BatchUndone {
            batch: record.undoes.ok_or_else(|| needs("undoes"))?,
        },
    })
}

fn adjusted(record: &Record, project: Option<ProjectRef>) -> Result<EntryChanges, Invalid> {
    let cleared = record.cleared.clone().unwrap_or_default();
    if let Some(unknown) = cleared
        .iter()
        .find(|field| !["note", "project"].contains(&field.as_str()))
    {
        return Err(Invalid::new(
            "`cleared` names `{field}`; only `note` and `project` can be cleared",
        )
        .with("field", unknown));
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
        (Some(_), true) => {
            Err(Invalid::new("`{field}` is set and cleared at once").with("field", field))
        }
        (Some(value), false) => Ok(Change::Set(value)),
        (None, true) => Ok(Change::Clear),
        (None, false) => Ok(Change::Keep),
    }
}

#[cfg(test)]
mod tests;
