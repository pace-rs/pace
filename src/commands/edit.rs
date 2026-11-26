//! `pace edit`: an entry as TOML in the user's editor; the changes become an
//! adjustment.

use std::collections::BTreeSet;

use abscissa_core::{Command, Runnable};
use jiff::ZonedDateTime;
use pace_core::{
    Change, Command as UseCase, DeviceId, Entry, EntryOrigin, ProjectId, ProjectKey, ProjectRef,
    Tag, command::Adjust,
};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult, Snippet};
use pace_time::parse_at;
use serde::{Deserialize, Serialize};
use toml::Spanned;

use crate::commands::tracking::track;
use crate::editor::edit_in_editor;
use crate::output::short_id;
use crate::prompt::{confirm, interactive};
use crate::select::chosen_entry;
use crate::{
    application::{exit_on_error, print_error},
    output::clock,
    select::Chosen,
};

/// What the snippet of a mistake in the edited file is called.
const EDITED: &str = "the edited entry";

const HELP: &str = "\
# Change what you like and save; pace records the changes as one adjustment,
# which `pace undo` takes back. Delete the note, project-key or tags line to
# remove it. A time can also be written as 2026-11-26 09:00.
";

/// `pace edit`
#[derive(clap::Args, Command, Debug)]
pub struct EditCmd {
    #[arg(
        value_name = "ENTRY",
        conflicts_with = "last",
        help = "The entry, by its short ID; the open one if left out"
    )]
    entry: Option<String>,

    #[arg(long, help = "The entry that ended last")]
    last: bool,

    #[arg(long, help = "Allow the entry to overlap another one")]
    allow_overlap: bool,
}

impl Runnable for EditCmd {
    fn run(&self) {
        exit_on_error(track(|session, _| {
            let chosen = chosen_entry(session, self.entry.as_deref(), self.last, "edit")?;
            let now = session.now();
            let mut text = edit_in_editor(&write(&chosen, session.device(), &now)?, ".toml")?;
            loop {
                match read(&text, &chosen.entry, &now) {
                    Ok(adjust) => {
                        return Ok(UseCase::Adjust(Adjust {
                            allow_overlap: self.allow_overlap,
                            ..adjust
                        }));
                    }
                    Err(error) if error.exit_code() != ExitCode::NothingToDo && interactive() => {
                        print_error(&error);
                        if !confirm("Open the entry again to fix it?")? {
                            return Err(PaceError::new(
                                ErrorKind::InvalidInput,
                                "the entry stays as it was",
                            )
                            .attach_hint("`pace edit` opens it again")
                            .attach_exit_code(ExitCode::NothingToDo));
                        }
                        text = edit_in_editor(&text, ".toml")?;
                    }
                    Err(error) => return Err(error),
                }
            }
        }));
    }
}

/// The entry's fields as the user edits them.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct Written<'a> {
    start: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<&'a ProjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_key: Option<&'a ProjectKey>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<&'a str>,
}

/// The fields as the user saved them, with spans for errors.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct Edited {
    start: Spanned<String>,
    #[serde(default)]
    end: Option<Spanned<String>>,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    project: Option<ProjectId>,
    #[serde(default)]
    project_key: Option<ProjectKey>,
    #[serde(default)]
    tags: BTreeSet<Tag>,
}

/// The entry as a commented TOML file.
fn write(chosen: &Chosen, device: DeviceId, now: &ZonedDateTime) -> PaceResult<String> {
    let entry = &chosen.entry;
    let how = match entry.origin {
        EntryOrigin::Live => "begun",
        EntryOrigin::Logged => "logged",
    };
    let place = if entry.device == device {
        "this device"
    } else {
        "another device"
    };
    let mut lines = vec![format!(
        "# pace edit: entry {}, {how} on {place}.",
        short_id(&chosen.state, entry.id)
    )];
    lines.push(HELP.trim_end().to_owned());
    let breaks = breaks(entry, now);
    if !breaks.is_empty() {
        lines.push("#".to_owned());
        lines.extend(breaks);
        lines.push("# Holds, pauses and catches are shown here, not edited.".to_owned());
    }
    lines.push(String::new());

    let (project, project_key) = match &entry.project {
        Some(ProjectRef::Id(id)) => (Some(id), None),
        Some(ProjectRef::Key(key)) => (None, Some(key)),
        None => (None, None),
    };
    let written = Written {
        start: entry.start.to_string(),
        end: entry.end.as_ref().map(ToString::to_string),
        note: entry.note.as_deref(),
        project,
        project_key,
        tags: entry.tags.iter().map(Tag::as_str).collect(),
    };
    let fields = toml::to_string(&written).map_err(|error| {
        PaceError::with_source(
            ErrorKind::Internal,
            "pace can't write the entry as TOML",
            error,
        )
        .ask_report()
    })?;
    lines.push(fields.trim_end().to_owned());
    if entry.note.is_none() {
        lines.push("# note = \"\"".to_owned());
    }
    if entry.project.is_none() {
        lines.push("# project-key = \"\"".to_owned());
    }
    if entry.tags.is_empty() {
        lines.push("# tags = []".to_owned());
    }
    lines.push(String::new());
    Ok(lines.join("\n"))
}

/// The holds, pauses and catches, as comment lines.
fn breaks(entry: &Entry, now: &ZonedDateTime) -> Vec<String> {
    let span = |start: &ZonedDateTime, end: Option<&ZonedDateTime>| {
        end.map_or_else(
            || format!("since {}", clock(start, now)),
            |end| format!("{}–{}", clock(start, now), clock(end, now)),
        )
    };
    let why =
        |reason: Option<&String>| reason.map_or_else(String::new, |reason| format!(": {reason}"));
    let holds = entry.holds.iter().map(|hold| {
        format!(
            "# Held {}{}.",
            span(&hold.start, hold.end.as_ref()),
            why(hold.reason.as_ref())
        )
    });
    let pauses = entry.pauses.iter().map(|pause| {
        format!(
            "# Paused {}{}.",
            span(&pause.start, pause.end.as_ref()),
            why(pause.reason.as_ref())
        )
    });
    let catches = entry
        .catches
        .iter()
        .map(|catch| format!("# Caught at {}: {}", clock(&catch.at, now), catch.text));
    holds.chain(pauses).chain(catches).collect()
}

/// The adjustment the saved `text` asks for.
fn read(text: &str, entry: &Entry, now: &ZonedDateTime) -> PaceResult<Adjust> {
    let edited: Edited = toml::from_str(text).map_err(|error| {
        let span = error.span().unwrap_or(0..0);
        mistake(
            text,
            span,
            "the edited entry has a mistake",
            error.message().trim(),
        )
        .attach_hint("fix the marked spot; nothing was changed")
    })?;

    let start = moment(text, &edited.start, now)?;
    let end = match (&edited.end, &entry.end) {
        (Some(end), _) => Some(moment(text, end, now)?),
        (None, Some(_)) => {
            return Err(PaceError::new(
                ErrorKind::InvalidInput,
                "the entry has ended, so it keeps an end",
            )
            .attach_hint("move the end instead of removing it; nothing was changed"));
        }
        (None, None) => None,
    };
    let project = match (edited.project, edited.project_key) {
        (Some(_), Some(_)) => {
            return Err(PaceError::new(
                ErrorKind::InvalidInput,
                "the edited entry has `project` and `project-key`; it needs one",
            )
            .attach_hint("keep one of them; nothing was changed"));
        }
        (Some(id), None) => Some(ProjectRef::Id(id)),
        (None, Some(key)) => Some(ProjectRef::Key(key)),
        (None, None) => None,
    };
    let note = edited.note.filter(|note| !note.trim().is_empty());

    let adjust = Adjust {
        entry: Some(entry.id),
        start: (start != entry.start).then_some(start),
        end: end.filter(|end| entry.end.as_ref() != Some(end)),
        note: change(note, entry.note.as_ref()),
        project: change(project, entry.project.as_ref()),
        tag: edited.tags.difference(&entry.tags).cloned().collect(),
        untag: entry.tags.difference(&edited.tags).cloned().collect(),
        allow_overlap: false,
    };
    if adjust
        == (Adjust {
            entry: Some(entry.id),
            ..Adjust::default()
        })
    {
        return Err(
            PaceError::new(ErrorKind::InvalidInput, "the entry is unchanged")
                .attach_hint("change a value and save the file to adjust the entry")
                .attach_exit_code(ExitCode::NothingToDo),
        );
    }
    Ok(adjust)
}

/// A time as pace wrote it, or in the `--at` grammar.
fn moment(text: &str, value: &Spanned<String>, now: &ZonedDateTime) -> PaceResult<ZonedDateTime> {
    if let Ok(moment) = value.get_ref().parse::<ZonedDateTime>() {
        return Ok(moment);
    }
    parse_at(value.get_ref(), now)
        .map(pace_time::At::earlier)
        .map_err(|_| {
            mistake(
                text,
                value.span(),
                "`{value}` is no time pace reads",
                "this time",
            )
            .attach_context("value", value.get_ref().clone())
            .attach_hint("write it as pace did, or as 2026-11-26 09:00; nothing was changed")
        })
}

fn mistake(text: &str, span: std::ops::Range<usize>, message: &str, label: &str) -> Box<PaceError> {
    PaceError::new(ErrorKind::InvalidInput, message)
        .attach_snippet(Snippet::new(EDITED, text, span, label))
}

/// The change from `current` to `new`.
fn change<T: PartialEq>(new: Option<T>, current: Option<&T>) -> Change<T> {
    match (new, current) {
        (Some(new), Some(current)) if new == *current => Change::Keep,
        (Some(new), _) => Change::Set(new),
        (None, Some(_)) => Change::Clear,
        (None, None) => Change::Keep,
    }
}
