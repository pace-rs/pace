//! An `[[event]]` record read back as an event, checked against its type.

use std::str::FromStr;

use jiff::Timestamp;
use pace_core::{Change, DeviceId, EntryChanges, Event, EventKind, Hlc, ProjectRef, RecordedEvent};

use crate::format::{EVENT_VERSION, EventType, NEWER, READ_ONLY, Record};

/// Why a record is no valid event: a guidance with its context, and how to
/// go on.
pub(super) struct InvalidRecord {
    pub(super) guidance: &'static str,
    pub(super) context: Vec<(&'static str, String)>,
    pub(super) hint: &'static str,
}

impl InvalidRecord {
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

fn parse_hlc(text: &str) -> Option<Hlc> {
    let (wall, counter) = text.split_once('/')?;
    Some(Hlc::new(
        Timestamp::from_str(wall).ok()?,
        counter.parse().ok()?,
    ))
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
pub(super) fn event_from_record(
    record: Record,
    device: DeviceId,
) -> Result<RecordedEvent, InvalidRecord> {
    let event_type = checked_type(&record)?;
    let hlc = parse_hlc(&record.hlc).ok_or_else(|| {
        InvalidRecord::new("`{hlc}` is no HLC like `2026-11-26T08:02:11.204Z/0`")
            .with("hlc", &record.hlc)
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
fn checked_type(record: &Record) -> Result<EventType, InvalidRecord> {
    let kind = &record.kind;
    let event_type = EventType::parse(kind).ok_or_else(|| {
        InvalidRecord::newer("pace doesn't know the event type `{type}`").with("type", kind)
    })?;
    let allowed = event_type.allowed();
    if record.v != EVENT_VERSION {
        let guidance = "pace reads version {reads} of `{type}`, not {found}";
        let invalid = if record.v > EVENT_VERSION {
            InvalidRecord::newer(guidance)
        } else {
            InvalidRecord::new(guidance)
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
        return Err(
            InvalidRecord::new("an event of type `{type}` has no `{field}`")
                .with("type", kind)
                .with("field", foreign),
        );
    }
    if record.project.is_some() && record.project_key.is_some() {
        return Err(InvalidRecord::new(
            "an event of type `{type}` has `project` and `project-key`; it needs one",
        )
        .with("type", kind));
    }
    Ok(event_type)
}

/// The event of a checked record of `event_type`, or the field its type
/// needs and misses.
fn event_kind(record: &Record, event_type: EventType) -> Result<EventKind, InvalidRecord> {
    let kind = &record.kind;
    let needs = |field: &str| {
        InvalidRecord::new("an event of type `{type}` needs `{field}`")
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

fn adjusted(record: &Record, project: Option<ProjectRef>) -> Result<EntryChanges, InvalidRecord> {
    let cleared = record.cleared.clone().unwrap_or_default();
    if let Some(unknown) = cleared
        .iter()
        .find(|field| !["note", "project"].contains(&field.as_str()))
    {
        return Err(InvalidRecord::new(
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

fn change<T>(
    field: &str,
    value: Option<T>,
    cleared: &[String],
) -> Result<Change<T>, InvalidRecord> {
    match (value, cleared.iter().any(|name| name == field)) {
        (Some(_), true) => {
            Err(InvalidRecord::new("`{field}` is set and cleared at once").with("field", field))
        }
        (Some(value), false) => Ok(Change::Set(value)),
        (None, true) => Ok(Change::Clear),
        (None, false) => Ok(Change::Keep),
    }
}
