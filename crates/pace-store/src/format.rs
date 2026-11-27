//! The TOML format of event files.
//!
//! Each event is one `[[event]]` table with its envelope (`id`, `type`, `v`,
//! `batch`, `at`, `about` if it differs from `at`, `hlc`) and the fields of
//! its type. pace reads strictly: an unknown key, a missing or foreign field,
//! an unknown type or version stops it with the file and the line.

use std::{collections::BTreeSet, ops::Range, path::Path};

use jiff::ZonedDateTime;
use pace_core::{
    BatchId, DeterminedBy, DeviceId, EntryId, EventId, HoldSource, PauseKind, ProjectId,
    ProjectKey, RecordedEvent, RoleId, Tag,
};
use pace_error::{ErrorKind, PaceError, PaceResult, closest_name};
use serde::{Deserialize, Serialize};
use toml::Spanned;

use crate::toml_file::{TomlCause, TomlFile};

mod event_type;
mod reading;
mod writing;

use event_type::EventType;
use reading::{InvalidRecord, event_from_record};
use writing::record_from_event;

/// The version of the event file format.
pub(crate) const FILE_VERSION: u32 = 1;

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
    role: Option<RoleId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    determined_by: Option<DeterminedBy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    billable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    allow_overlap: Option<bool>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    invoice: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    net_seconds: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    billed_seconds: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rate: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    currency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    amount: Option<String>,
}

/// The keys an `[[event]]` table may have.
const KEYS: [&str; 32] = [
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
    "role",
    "determined-by",
    "billable",
    "allow-overlap",
    "source",
    "kind",
    "reason",
    "text",
    "start",
    "end",
    "cleared",
    "undoes",
    "invoice",
    "net-seconds",
    "billed-seconds",
    "rate",
    "currency",
    "amount",
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
struct Chunk<'records> {
    event: &'records [Record],
}

/// The header of a new event file.
#[must_use]
pub(crate) fn file_header() -> String {
    format!("version = {FILE_VERSION}\n")
}

/// The text to append to an event file for `events`.
///
/// # Errors
///
/// If an event cannot be written as TOML, which is a bug.
pub(crate) fn serialize_events(events: &[RecordedEvent]) -> PaceResult<String> {
    let records: Vec<Record> = events.iter().map(record_from_event).collect();
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
pub(crate) fn parse_events(
    path: &Path,
    text: &str,
    device: DeviceId,
) -> PaceResult<Vec<RecordedEvent>> {
    let file: File = match toml::from_str(text) {
        Ok(file) => file,
        Err(error) => return Err(explain(path, text, &error)),
    };
    if file.version != FILE_VERSION {
        return Err(newer_or_broken(path, text, "version", file.version));
    }
    file.event
        .into_iter()
        .map(|record| {
            let span = record.span();
            event_from_record(record.into_inner(), device)
                .map_err(|invalid| invalid_event(path, text, span, invalid))
        })
        .collect()
}

/// What [`parse_events_collecting`] found in an event file: the events
/// that read, and a mistake for each that doesn't.
#[derive(Debug, Default)]
pub struct EventFileRead {
    /// The valid events, in the order of the file.
    pub events: Vec<RecordedEvent>,
    /// Each mistake, in the order of the file, with the file and the line.
    pub mistakes: Vec<Box<PaceError>>,
}

/// An event file read without its events' types, so that one invalid event
/// leaves the others.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LenientFile {
    version: u32,
    #[serde(default)]
    event: Vec<Spanned<toml::Table>>,
}

/// The events of an event file of `device`, and a mistake for each
/// invalid event.
///
/// `parse_events`, which commands use, stops at the first invalid event;
/// this reader names every one, for `pace doctor`. A file that is no valid
/// TOML, has another `version` or an unknown key outside its events has
/// one mistake and no events.
#[must_use]
pub fn parse_events_collecting(path: &Path, text: &str, device: DeviceId) -> EventFileRead {
    let only = |mistake| EventFileRead {
        events: Vec::new(),
        mistakes: vec![mistake],
    };
    let file: LenientFile = match toml::from_str(text) {
        Ok(file) => file,
        Err(error) => return only(explain(path, text, &error)),
    };
    if file.version != FILE_VERSION {
        return only(newer_or_broken(path, text, "version", file.version));
    }
    let mut read = EventFileRead::default();
    for (index, table) in file.event.into_iter().enumerate() {
        let span = table.span();
        let event = Record::deserialize(toml::Value::Table(table.into_inner()))
            .map_err(|error| unreadable_event(path, text, index, span.clone(), &error))
            .and_then(|record| {
                event_from_record(record, device)
                    .map_err(|invalid| invalid_event(path, text, span, invalid))
            });
        match event {
            Ok(event) => read.events.push(event),
            Err(mistake) => read.mistakes.push(mistake),
        }
    }
    read
}

/// The error for the event at `span` that is no valid event, as
/// `invalid` says.
fn invalid_event(
    path: &Path,
    text: &str,
    span: Range<usize>,
    invalid: InvalidRecord,
) -> Box<PaceError> {
    event_file(path, text)
        .error_at(
            span,
            invalid.guidance,
            invalid.context,
            invalid.hint,
            "this event",
        )
        .attach_error_code("pace::store::invalid-event")
}

/// The error for the event number `index` at `span`, which serde refused
/// as a record: its unknown key, or else the value at fault.
fn unreadable_event(
    path: &Path,
    text: &str,
    index: usize,
    span: Range<usize>,
    error: &toml::de::Error,
) -> Box<PaceError> {
    let file = event_file(path, text);
    let document = file.parse().ok();
    let unknown = document.as_ref().and_then(|document| {
        let event = document
            .table()
            .get("event")?
            .tables()
            .into_iter()
            .nth(index)?;
        event
            .entries()
            .into_iter()
            .map(|(key, _)| key)
            .find(|key| !KEYS.contains(&key.name()))
            .map(|key| unknown_key(&file, key.name(), key.span()))
    });
    unknown.unwrap_or_else(|| {
        file.error_at(
            span,
            "pace cannot read this event: {reason}",
            [("reason", error.message().trim().to_owned())],
            REPAIR,
            "this event",
        )
        .attach_error_code("pace::store::invalid-event")
    })
}

fn event_file<'file>(path: &'file Path, text: &'file str) -> TomlFile<'file> {
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
fn explain(path: &Path, text: &str, error: &toml::de::Error) -> Box<PaceError> {
    let file = event_file(path, text);
    let Ok(document) = file.parse() else {
        return broken(&file, error, "the event file {file} is not valid TOML")
            .attach_error_code("pace::store::syntax");
    };
    for (key, value) in document.table().entries() {
        let keys = std::iter::once(key).chain(
            value
                .tables()
                .into_iter()
                .flat_map(|event| event.entries().into_iter().map(|(key, _)| key)),
        );
        for key in keys {
            if !KEYS.contains(&key.name()) {
                return unknown_key(&file, key.name(), key.span());
            }
        }
    }
    broken(&file, error, "pace cannot read the event file {file}")
        .attach_error_code("pace::store::invalid-file")
}

/// The error `guidance` about the event file `file`, caused by `error`.
fn broken(file: &TomlFile<'_>, error: &toml::de::Error, guidance: &'static str) -> Box<PaceError> {
    let span = error.span().unwrap_or(0..0);
    let label = error.message().trim().to_owned();
    let error = PaceError::with_source(file.kind, guidance, TomlCause(label.clone()))
        .attach_context("file", file.path.display().to_string())
        .attach_hint(REPAIR);
    file.point_at(error, span, &label)
}

fn unknown_key(file: &TomlFile<'_>, name: &str, span: Range<usize>) -> Box<PaceError> {
    let hint = closest_name(name, KEYS.iter().copied()).map_or_else(
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

const NEWER: &str = "a newer pace may have written it on another device; update pace";

#[cfg(test)]
mod tests;
