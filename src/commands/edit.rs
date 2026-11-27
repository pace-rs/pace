//! `pace edit`: an entry as TOML in the user's editor; the changes become an
//! adjustment.

use std::collections::BTreeSet;

use abscissa_core::{Command, Runnable};
use jiff::ZonedDateTime;
use pace_core::{
    Change, Command as UseCase, DeviceId, Entry, EntryOrigin, ProjectId, ProjectKey, ProjectRef,
    Tag, Text, command::Adjust,
};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult, Snippet};
use pace_time::parse_at;
use serde::{Deserialize, Serialize};
use toml::Spanned;

use crate::commands::tracking::{chosen_moment, track};
use crate::editor::{edit_in_editor, editor_command};
use crate::prompt::Ask;
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
        exit_on_error(track(|session, context, notes| {
            let asker = &context.terminal;
            let editor = editor_command(context.editor.as_deref(), asker.can_ask())?;
            let chosen = chosen_entry(session, asker, self.entry.as_deref(), self.last, "edit")?;
            let now = session.now();
            let adjust = edit_until_read(
                &write(&chosen, session.device(), &now)?,
                &chosen.entry,
                &now,
                asker,
                notes,
                |text| edit_in_editor(&editor, text, ".toml"),
                print_error,
            )?;
            Ok(UseCase::Adjust(Adjust {
                allow_overlap: self.allow_overlap,
                ..adjust
            }))
        }));
    }
}

/// The adjustment from `written` once `edit` changed it. A mistake is
/// shown with `report`; on a terminal the user may open the entry again to
/// fix it, without one the mistake is the error. What pace chose for the
/// user goes to `notes`.
fn edit_until_read(
    written: &str,
    entry: &Entry,
    now: &ZonedDateTime,
    asker: &dyn Ask,
    notes: &mut Vec<String>,
    mut edit: impl FnMut(&str) -> PaceResult<String>,
    report: impl Fn(&PaceError),
) -> PaceResult<Adjust> {
    let mut text = edit(written)?;
    loop {
        match read(&text, entry, now, asker, notes) {
            Ok(adjust) => return Ok(adjust),
            Err(error) if error.exit_code() != ExitCode::NothingToDo && asker.can_ask() => {
                report(&error);
                if !asker.confirm("Open the entry again to fix it?")? {
                    return Err(PaceError::new(
                        ErrorKind::InvalidInput,
                        "the entry stays as it was",
                    )
                    .attach_hint("`pace edit` opens it again")
                    .attach_exit_code(ExitCode::NothingToDo));
                }
                text = edit(&text)?;
            }
            Err(error) => return Err(error),
        }
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
        chosen.state.short_id(entry.id)
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
        lines.push("# project-key = \"your-key\"".to_owned());
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

/// The adjustment the saved `text` asks for; a time that happened twice is
/// chosen as [`chosen_moment`] says.
fn read(
    text: &str,
    entry: &Entry,
    now: &ZonedDateTime,
    asker: &dyn Ask,
    notes: &mut Vec<String>,
) -> PaceResult<Adjust> {
    let edited: Edited = toml::from_str(text).map_err(|error| {
        let mistake = PaceError::new(
            ErrorKind::InvalidInput,
            "the edited entry has a mistake: {message}",
        )
        .attach_context("message", error.message().trim())
        .attach_hint("fix the marked spot; nothing was changed");
        match error.span() {
            Some(span) if !text.trim().is_empty() => {
                mistake.attach_snippet(Snippet::new(EDITED, text, span, "here"))
            }
            _ => mistake,
        }
    })?;

    let mut edited_moment = |value: &Spanned<String>| moment(text, value, now, asker, notes);
    let start = edited_moment(&edited.start)?;
    let end = match (&edited.end, &entry.end) {
        (Some(end), _) => Some(edited_moment(end)?),
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
    let note = edited.note.and_then(|note| Text::try_from(note).ok());
    let current_note = entry
        .note
        .clone()
        .and_then(|note| Text::try_from(note).ok());

    let adjust = Adjust {
        entry: Some(entry.id),
        start: (start != entry.start).then_some(start),
        end: end.filter(|end| entry.end.as_ref() != Some(end)),
        note: change(note, current_note.as_ref()),
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

/// A time as pace wrote it, or in the `--at` grammar; a mistake points at
/// it in `text`.
fn moment(
    text: &str,
    value: &Spanned<String>,
    now: &ZonedDateTime,
    asker: &dyn Ask,
    notes: &mut Vec<String>,
) -> PaceResult<ZonedDateTime> {
    if let Ok(moment) = value.get_ref().parse::<ZonedDateTime>() {
        return Ok(moment);
    }
    parse_at(value.get_ref(), now)
        .and_then(|at| chosen_moment(value.get_ref(), at, asker, notes))
        .map_err(|error| {
            error.attach_snippet(Snippet::new(EDITED, text, value.span(), "this time"))
        })
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

#[cfg(test)]
mod tests {
    use std::{cell::Cell, collections::VecDeque};

    use super::*;
    use crate::prompt::Scripted;

    fn at(time: &str) -> ZonedDateTime {
        format!("2026-11-26T{time}[Europe/Berlin]").parse().unwrap()
    }

    fn entry() -> Entry {
        Entry {
            id: "0193a8d0-000a-7000-8000-000000000001".parse().unwrap(),
            device: "0193a8d0-000d-7000-8000-000000000001".parse().unwrap(),
            start: at("09:00"),
            end: Some(at("10:00")),
            project: None,
            note: Some("Draft".to_owned()),
            tags: BTreeSet::new(),
            holds: Vec::new(),
            pauses: Vec::new(),
            catches: Vec::new(),
            origin: EntryOrigin::Live,
            filed: at("09:00"),
        }
    }

    fn saved(note: &str) -> String {
        format!(
            "start = \"{}\"\nend = \"{}\"\n{note}\n",
            at("09:00"),
            at("10:00")
        )
    }

    /// Run [`edit_until_read`] with an editor that saves `texts` in turn;
    /// also the number of mistakes reported.
    fn edited(asker: &Scripted, texts: &[String]) -> (PaceResult<Adjust>, usize) {
        let mut texts: VecDeque<String> = texts.iter().cloned().collect();
        let reported = Cell::new(0);
        let result = edit_until_read(
            &saved("note = \"Draft\""),
            &entry(),
            &at("10:05"),
            asker,
            &mut Vec::new(),
            |_| Ok(texts.pop_front().unwrap()),
            |_| reported.set(reported.get() + 1),
        );
        (result, reported.get())
    }

    /// The labels of the error's snippet: where it points in the file.
    fn labels(error: &PaceError) -> Vec<(usize, String)> {
        use miette::Diagnostic;
        error
            .labels()
            .into_iter()
            .flatten()
            .map(|label| (label.offset(), label.label().unwrap_or_default().to_owned()))
            .collect()
    }

    #[test]
    fn readable_time_in_future_says_so_and_points_at_it() {
        let text = format!("start = \"{}\"\nend = \"2026-11-26 23:00\"\n", at("09:00"));
        let mut notes = Vec::new();

        let error = read(
            &text,
            &entry(),
            &at("10:05"),
            &Scripted::without_terminal(),
            &mut notes,
        )
        .unwrap_err();

        assert!(error.to_string().contains("lies in the future"), "{error}");
        assert_eq!(
            labels(&error),
            [(
                text.find("2026-11-26 23:00").unwrap() - 1,
                "this time".to_owned()
            )]
        );
    }

    #[test]
    fn emptied_file_names_the_mistake_in_guidance() {
        let error = read(
            "",
            &entry(),
            &at("10:05"),
            &Scripted::without_terminal(),
            &mut Vec::new(),
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "the edited entry has a mistake: missing field `start`"
        );
    }

    #[test]
    fn time_that_happened_twice_without_terminal_is_first_with_note() {
        let now: ZonedDateTime = "2026-10-25T12:00[Europe/Berlin]".parse().unwrap();
        let entry = Entry {
            start: "2026-10-25T01:00[Europe/Berlin]".parse().unwrap(),
            end: Some("2026-10-25T04:00[Europe/Berlin]".parse().unwrap()),
            ..entry()
        };
        let text = format!(
            "start = \"02:30\"\nend = \"{}\"\nnote = \"Draft\"\n",
            entry.end.as_ref().unwrap()
        );
        let mut notes = Vec::new();

        let adjust = read(
            &text,
            &entry,
            &now,
            &Scripted::without_terminal(),
            &mut notes,
        )
        .unwrap();

        assert_eq!(
            adjust.start.map(|start| start.to_string()),
            Some("2026-10-25T02:30:00+02:00[Europe/Berlin]".to_owned())
        );
        assert_eq!(notes.len(), 1, "{notes:?}");
    }

    #[test]
    fn missing_project_offers_key_that_works_once_uncommented() {
        let chosen = Chosen {
            state: pace_core::State::default(),
            entry: entry(),
        };
        let written = write(&chosen, entry().device, &at("10:05")).unwrap();
        let line = written
            .lines()
            .find(|line| line.starts_with("# project-key"))
            .unwrap();

        assert!(
            line.trim_start_matches("# ").parse::<toml::Table>().is_ok(),
            "{line}"
        );
        let uncommented = written.replace(line, line.trim_start_matches("# "));
        assert!(
            read(
                &uncommented,
                &entry(),
                &at("10:05"),
                &Scripted::without_terminal(),
                &mut Vec::new()
            )
            .is_ok()
        );
    }

    #[test]
    fn mistake_declined_leaves_entry_as_it_was() {
        let asker = Scripted::confirming([false]);

        let (result, reported) = edited(&asker, &[saved("notes = \"Write offer\"")]);

        let error = result.unwrap_err();
        assert_eq!(error.to_string(), "the entry stays as it was");
        assert_eq!(error.exit_code(), ExitCode::NothingToDo);
        assert_eq!(reported, 1);
        assert_eq!(asker.questions(), ["Open the entry again to fix it?"]);
    }

    #[test]
    fn mistake_fixed_after_opening_again_adjusts_entry() {
        let asker = Scripted::confirming([true]);

        let (result, reported) = edited(
            &asker,
            &[
                saved("notes = \"Write offer\""),
                saved("note = \"Write offer\""),
            ],
        );

        assert_eq!(
            result.unwrap().note,
            Change::Set("Write offer".parse().unwrap())
        );
        assert_eq!(reported, 1);
    }

    #[test]
    fn mistake_without_terminal_is_error() {
        let asker = Scripted::without_terminal();

        let (result, reported) = edited(&asker, &[saved("notes = \"Write offer\"")]);

        assert!(
            result
                .unwrap_err()
                .to_string()
                .starts_with("the edited entry has a mistake: unknown field `notes`")
        );
        assert_eq!(reported, 0);
        assert_eq!(asker.questions(), Vec::<String>::new());
    }
}
