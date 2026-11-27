//! `pace edit`: an entry as TOML in the user's editor; the changes become an
//! adjustment.

use std::collections::BTreeSet;

use abscissa_core::{Command, Runnable};
use jiff::ZonedDateTime;
use pace_core::{
    Change, Command as UseCase, DeviceId, Entry, EntryOrigin, EntryVerb, OVERLAP, ProjectId,
    ProjectKey, ProjectRef, Tag, Text, command::Adjust,
};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult, Snippet};
use pace_time::{NOT_UNDERSTOOD, format_moment, parse_at};
use serde::{Deserialize, Serialize};
use toml::Spanned;

use crate::commands::tracking::{chosen_moment, track_run};
use crate::editor::{edit_in_editor, editor_command};
use crate::prompt::Ask;
use crate::select::{Target, chosen_entry};
use crate::{
    application::{exit_on_error, print_error},
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
        exit_on_error(track_run(|session, context, notes| {
            let asker = &context.terminal;
            let editor = editor_command(context.editor.as_deref(), asker.can_ask())?;
            let chosen = chosen_entry(
                session,
                asker,
                Target::new(self.entry.as_deref(), self.last),
                EntryVerb::Edit,
            )?;
            let now = session.now();
            edit_until_applied(
                &write_entry_text(&chosen, session.device(), &now)?,
                asker,
                self.allow_overlap,
                |text| edit_in_editor(&editor, text, ".toml"),
                |text, allow_overlap| {
                    let adjust = read_adjustment(text, &chosen.entry, &now, asker, notes)?;
                    session.run(&UseCase::Adjust(Adjust {
                        allow_overlap,
                        ..adjust
                    }))
                },
                print_error,
            )
        }));
    }
}

/// What the user does with an edited entry that pace refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Next {
    /// Open the edited text again.
    EditAgain,
    /// Apply it again, with the overlap allowed.
    AllowOverlap,
    /// Leave the entry as it was.
    Leave,
}

/// What `apply` makes of `written` once `edit` changed it; `apply` reads
/// the text and runs the adjustment, with the overlap allowed or not. A
/// mistake or a refusal of the rules is shown with `report`: on a terminal
/// the user may open the edited text again to fix it, or allow an
/// overlap; without one the mistake is the error.
fn edit_until_applied<T>(
    written: &str,
    asker: &dyn Ask,
    allow_overlap: bool,
    mut edit: impl FnMut(&str) -> PaceResult<String>,
    mut apply: impl FnMut(&str, bool) -> PaceResult<T>,
    report: impl Fn(&PaceError),
) -> PaceResult<T> {
    let mut text = edit(written)?;
    let mut allow_overlap = allow_overlap;
    loop {
        let error = match apply(&text, allow_overlap) {
            Ok(applied) => return Ok(applied),
            Err(error) if can_fix(&error) && asker.can_ask() => error,
            Err(error) => return Err(error),
        };
        report(&error);
        match next_step(&error, asker)? {
            Next::EditAgain => text = edit(&text)?,
            Next::AllowOverlap => allow_overlap = true,
            Next::Leave => {
                return Err(
                    PaceError::new(ErrorKind::InvalidInput, "the entry stays as it was")
                        .attach_hint("`pace edit` opens it again")
                        .attach_exit_code(ExitCode::NothingToDo),
                );
            }
        }
    }
}

/// Whether the user can fix `error` in the edited text: a mistake in it
/// or a refusal of the rules, but not an unchanged entry or a failing
/// store.
fn can_fix(error: &PaceError) -> bool {
    error.exit_code() != ExitCode::NothingToDo
        && matches!(error.kind(), ErrorKind::InvalidInput | ErrorKind::Time)
}

/// What the user does after `error`: an overlap may also be allowed.
fn next_step(error: &PaceError, asker: &dyn Ask) -> PaceResult<Next> {
    if !error.is_code(OVERLAP) {
        let again = asker.confirm("Open the entry again to fix it?")?;
        return Ok(if again { Next::EditAgain } else { Next::Leave });
    }
    let steps = [Next::EditAgain, Next::AllowOverlap, Next::Leave];
    let choice = asker.choose(
        "What should pace do with the edited entry?",
        &[
            "open the entry again to fix it".to_owned(),
            "allow the overlap".to_owned(),
            "leave the entry as it was".to_owned(),
        ],
    )?;
    Ok(steps.get(choice).copied().unwrap_or(Next::Leave))
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
fn write_entry_text(chosen: &Chosen, device: DeviceId, now: &ZonedDateTime) -> PaceResult<String> {
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
            || format!("since {}", format_moment(start, now)),
            |end| format!("{}–{}", format_moment(start, now), format_moment(end, now)),
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
    let catches = entry.catches.iter().map(|catch| {
        format!(
            "# Caught at {}: {}",
            format_moment(&catch.at, now),
            catch.text
        )
    });
    holds.chain(pauses).chain(catches).collect()
}

/// The adjustment the saved `text` asks for; a time that happened twice is
/// chosen as [`chosen_moment`] says.
fn read_adjustment(
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
        .attach_context("message", error.message().trim());
        // A missing field has no spot in the file, though toml gives it
        // an empty span at 0; a syntax error's empty span marks its spot,
        // and so does every other span, also in a file without `start`.
        match error.span() {
            Some(span) if !(span.is_empty() && lacks_start(text)) => mistake
                .attach_snippet(Snippet::new(EDITED, text, span, "here"))
                .attach_hint("fix the marked spot; nothing was changed"),
            _ if lacks_start(text) => {
                mistake.attach_hint("write the fields back, at least `start`; nothing was changed")
            }
            _ => mistake.attach_hint("fix the named mistake; nothing was changed"),
        }
    })?;

    let mut edited_moment =
        |value: &Spanned<String>| parse_edited_moment(text, value, now, asker, notes);
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

    // the whole entry as saved; the core leaves out what didn't change
    let adjust = Adjust {
        entry: Some(entry.id),
        start: Some(start),
        end,
        note: note.map_or(Change::Clear, Change::Set),
        project: project.map_or(Change::Clear, Change::Set),
        untag: entry.tags.difference(&edited.tags).cloned().collect(),
        tag: edited.tags,
        allow_overlap: false,
    };
    if adjust.changes(entry).is_empty() {
        return Err(
            PaceError::new(ErrorKind::InvalidInput, "the entry is unchanged")
                .attach_hint("change a value and save the file to adjust the entry")
                .attach_exit_code(ExitCode::NothingToDo),
        );
    }
    Ok(adjust)
}

/// A time as pace wrote it, or in the `--at` grammar; a mistake points at
/// it in `text` and says that nothing was changed.
fn parse_edited_moment(
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
            let hint = if error.is_code(NOT_UNDERSTOOD) {
                time_forms(now)
            } else {
                error.hint().unwrap_or("give another time").to_owned()
            };
            error
                .attach_hint(unchanged(&hint))
                .attach_snippet(Snippet::new(EDITED, text, value.span(), "this time"))
        })
}

/// Whether the edited `text` is TOML without a `start`: a missing field
/// has no spot in the file to mark.
fn lacks_start(text: &str) -> bool {
    toml::from_str::<toml::Table>(text).is_ok_and(|table| !table.contains_key("start"))
}

/// `hint` with the words that nothing was changed: after it, or before
/// it when it asks, so no punctuation follows the question mark.
fn unchanged(hint: &str) -> String {
    if hint.ends_with('?') {
        format!("nothing was changed; {hint}")
    } else {
        format!("{hint}; nothing was changed")
    }
}

/// The two forms of a time the edited file takes, shown with `now`.
fn time_forms(now: &ZonedDateTime) -> String {
    let minute = now
        .with()
        .second(0)
        .subsec_nanosecond(0)
        .build()
        .unwrap_or_else(|_| now.clone());
    format!(
        "write it as pace does, like `{minute}`, or as `{}`",
        minute.strftime("%Y-%m-%d %H:%M")
    )
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, collections::VecDeque};

    use pace_core::{HoldSource, PauseKind};

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

    /// What [`edit_until_applied`] did with an editor that saved `texts`
    /// in turn.
    struct Edited {
        result: PaceResult<Adjust>,
        /// The number of mistakes reported.
        reported: usize,
        /// The text the editor opened each time.
        opened: Vec<String>,
    }

    /// Run [`edit_until_applied`] with an editor that saves `texts` in
    /// turn; `refuse` stands in for the core's rules.
    fn edited_refusing(
        asker: &Scripted,
        texts: &[String],
        mut refuse: impl FnMut(&Adjust) -> PaceResult<()>,
    ) -> Edited {
        let mut texts: VecDeque<String> = texts.iter().cloned().collect();
        let mut opened = Vec::new();
        let reported = Cell::new(0);
        let result = edit_until_applied(
            &saved("note = \"Draft\""),
            asker,
            false,
            |text| {
                opened.push(text.to_owned());
                Ok(texts.pop_front().unwrap())
            },
            |text, allow_overlap| {
                let adjust = Adjust {
                    allow_overlap,
                    ..read_adjustment(text, &entry(), &at("10:05"), asker, &mut Vec::new())?
                };
                refuse(&adjust).map(|()| adjust)
            },
            |_| reported.set(reported.get() + 1),
        );
        Edited {
            result,
            reported: reported.get(),
            opened,
        }
    }

    /// Run [`edit_until_applied`] with an editor that saves `texts` in
    /// turn and rules that take every adjustment; also the number of
    /// mistakes reported.
    fn edited(asker: &Scripted, texts: &[String]) -> (PaceResult<Adjust>, usize) {
        let run = edited_refusing(asker, texts, |_| Ok(()));
        (run.result, run.reported)
    }

    /// The rules' refusal of an entry moved into another one, unless the
    /// overlap is allowed.
    fn refuse_overlap(adjust: &Adjust) -> PaceResult<()> {
        if adjust.allow_overlap {
            return Ok(());
        }
        Err(
            PaceError::new(ErrorKind::InvalidInput, "the entry would overlap 0193")
                .attach_error_code(OVERLAP),
        )
    }

    #[test]
    fn refused_change_opens_edited_text_again() {
        let asker = Scripted::confirming([true]);
        let refused = saved("note = \"Write offer\"");
        let mut refusals = 1;

        let run = edited_refusing(&asker, &[refused.clone(), refused.clone()], |_| {
            if refusals == 0 {
                return Ok(());
            }
            refusals -= 1;
            Err(PaceError::new(
                ErrorKind::InvalidInput,
                "the entry would end before it starts",
            ))
        });

        assert!(run.result.is_ok());
        assert_eq!(run.reported, 1);
        assert_eq!(run.opened, [saved("note = \"Draft\""), refused]);
        assert_eq!(asker.questions(), ["Open the entry again to fix it?"]);
    }

    #[test]
    fn overlap_allowed_after_refusal_keeps_changes() {
        let asker = Scripted::choosing([1]);

        let run = edited_refusing(&asker, &[saved("note = \"Write offer\"")], refuse_overlap);

        let adjust = run.result.unwrap();
        assert!(adjust.allow_overlap);
        assert_eq!(adjust.note, Change::Set("Write offer".parse().unwrap()));
        assert_eq!(run.opened.len(), 1);
        assert_eq!(
            asker.offered(),
            [[
                "open the entry again to fix it",
                "allow the overlap",
                "leave the entry as it was",
            ]]
        );
    }

    #[test]
    fn overlap_left_after_refusal_leaves_entry_as_it_was() {
        let asker = Scripted::choosing([2]);

        let run = edited_refusing(&asker, &[saved("note = \"Write offer\"")], refuse_overlap);

        let error = run.result.unwrap_err();
        assert_eq!(error.to_string(), "the entry stays as it was");
        assert_eq!(error.exit_code(), ExitCode::NothingToDo);
    }

    #[test]
    fn refusal_without_terminal_is_error() {
        let asker = Scripted::without_terminal();

        let run = edited_refusing(&asker, &[saved("note = \"Write offer\"")], refuse_overlap);

        assert!(run.result.unwrap_err().is_code(OVERLAP));
        assert_eq!(run.reported, 0);
        assert_eq!(asker.questions(), Vec::<String>::new());
    }

    #[test]
    fn store_failure_opens_nothing_again() {
        let asker = Scripted::confirming([true]);

        let run = edited_refusing(&asker, &[saved("note = \"Write offer\"")], |_| {
            Err(PaceError::new(
                ErrorKind::InputOutput,
                "pace can't write the event log",
            ))
        });

        assert_eq!(
            run.result.unwrap_err().to_string(),
            "pace can't write the event log"
        );
        assert_eq!(run.opened.len(), 1);
        assert_eq!(asker.questions(), Vec::<String>::new());
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

        let error = read_adjustment(
            &text,
            &entry(),
            &at("10:05"),
            &Scripted::without_terminal(),
            &mut notes,
        )
        .unwrap_err();

        assert!(error.to_string().contains("lies in the future"), "{error}");
        assert!(
            error
                .hint()
                .is_some_and(|hint| hint.ends_with("; nothing was changed")),
            "{:?}",
            error.hint()
        );
        assert_eq!(
            labels(&error),
            [(
                text.find("2026-11-26 23:00").unwrap() - 1,
                "this time".to_owned()
            )]
        );
    }

    #[test]
    fn emptied_file_names_mistake_in_guidance_and_hint() {
        let error = read_adjustment(
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
        assert_eq!(
            error.hint(),
            Some("write the fields back, at least `start`; nothing was changed")
        );
    }

    #[test]
    fn missing_start_marks_no_spot_and_hints_at_field() {
        let text = format!(
            "# pace edit: entry 0193, begun on this device.\n# Started at 09:00.\nend = \"{}\"\n",
            at("10:00")
        );

        let error = read_adjustment(
            &text,
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
        assert_eq!(labels(&error), []);
        assert_eq!(
            error.hint(),
            Some("write the fields back, at least `start`; nothing was changed")
        );
    }

    #[test]
    fn time_in_future_by_offset_asks_after_saying_nothing_changed() {
        let text = "start = \"+15m\"\n";

        let error = read_adjustment(
            text,
            &entry(),
            &at("10:05"),
            &Scripted::without_terminal(),
            &mut Vec::new(),
        )
        .unwrap_err();

        assert_eq!(
            error.hint(),
            Some("nothing was changed; did you mean `-15m`?")
        );
    }

    #[test]
    fn marked_mistake_hints_at_marked_spot() {
        let text = format!("start = \"{}\"\nend = 10\n", at("09:00"));

        let error = read_adjustment(
            &text,
            &entry(),
            &at("10:05"),
            &Scripted::without_terminal(),
            &mut Vec::new(),
        )
        .unwrap_err();

        assert_eq!(labels(&error).len(), 1);
        assert_eq!(
            error.hint(),
            Some("fix the marked spot; nothing was changed")
        );
    }

    #[test]
    fn mistyped_start_key_marks_its_spot() {
        let text = format!(
            "# pace edit: entry 7049, begun on this device.\nstrat = \"{}\"\n",
            at("09:00")
        );

        let error = read_adjustment(
            &text,
            &entry(),
            &at("10:05"),
            &Scripted::without_terminal(),
            &mut Vec::new(),
        )
        .unwrap_err();

        assert_eq!(
            labels(&error),
            [(text.find("strat").unwrap(), "here".to_owned())]
        );
        assert_eq!(
            error.hint(),
            Some("fix the marked spot; nothing was changed")
        );
    }

    #[test]
    fn wrong_type_without_start_marks_its_spot() {
        let text = "# Started at 09:00.\nend = 10\n";

        let error = read_adjustment(
            text,
            &entry(),
            &at("10:05"),
            &Scripted::without_terminal(),
            &mut Vec::new(),
        )
        .unwrap_err();

        assert_eq!(labels(&error).len(), 1, "{error}");
        assert_eq!(labels(&error)[0].1, "here");
        assert_eq!(
            error.hint(),
            Some("fix the marked spot; nothing was changed")
        );
    }

    #[test]
    fn deleted_value_marks_its_spot() {
        let text = format!("start = \nend = \"{}\"\n", at("10:00"));

        let error = read_adjustment(
            &text,
            &entry(),
            &at("10:05"),
            &Scripted::without_terminal(),
            &mut Vec::new(),
        )
        .unwrap_err();

        assert_eq!(labels(&error), [(8, "here".to_owned())]);
        assert_eq!(
            error.hint(),
            Some("fix the marked spot; nothing was changed")
        );
    }

    #[test]
    fn unclosed_string_marks_its_spot() {
        let text = "# Started at 09:00.\nstart = \"09:00\nend = \"10:00\"\n";

        let error = read_adjustment(
            text,
            &entry(),
            &at("10:05"),
            &Scripted::without_terminal(),
            &mut Vec::new(),
        )
        .unwrap_err();

        assert_eq!(labels(&error).len(), 1, "{error}");
        assert_eq!(labels(&error)[0].1, "here");
        assert_eq!(
            error.hint(),
            Some("fix the marked spot; nothing was changed")
        );
    }

    #[test]
    fn unreadable_time_hints_at_pace_form_and_keeps_entry() {
        let value = "2026-11-26T09:00:00+01:00[Europe/Berln]";
        let text = format!("start = \"{value}\"\n");

        let error = read_adjustment(
            &text,
            &entry(),
            &at("10:05"),
            &Scripted::without_terminal(),
            &mut Vec::new(),
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            format!("`{value}` is not a time pace understands")
        );
        assert_eq!(
            error.hint(),
            Some(
                "write it as pace does, like `2026-11-26T10:05:00+01:00[Europe/Berlin]`, \
                 or as `2026-11-26 10:05`; nothing was changed"
            )
        );
        assert_eq!(
            labels(&error),
            [(text.find(value).unwrap() - 1, "this time".to_owned())]
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

        let adjust = read_adjustment(
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
        let written = write_entry_text(&chosen, entry().device, &at("10:05")).unwrap();
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
            read_adjustment(
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

    /// The file pace writes for `entry`, seen from device 2.
    fn written(entry: Entry) -> String {
        let chosen = Chosen {
            state: pace_core::State::default(),
            entry,
        };
        write_entry_text(
            &chosen,
            "0193a8d0-000d-7000-8000-000000000002".parse().unwrap(),
            &at("10:05"),
        )
        .unwrap()
    }

    #[test]
    fn written_entry_shows_its_origin_device_and_breaks_as_comments() {
        let entry = Entry {
            origin: EntryOrigin::Logged,
            holds: vec![pace_core::EntryHold {
                start: at("09:10"),
                end: Some(at("09:20")),
                source: HoldSource::Outer,
                reason: Some("phone call".to_owned()),
            }],
            pauses: vec![pace_core::EntryPause {
                start: at("09:40"),
                end: None,
                kind: PauseKind::Break,
                reason: None,
            }],
            catches: vec![pace_core::EntryCatch {
                at: at("09:30"),
                text: "call Anna".to_owned(),
            }],
            ..entry()
        };

        let text = written(entry);

        let lines: Vec<&str> = text.lines().collect();
        let breaks = lines.iter().position(|line| *line == "#").unwrap() + 1;
        assert_eq!(
            lines[0],
            "# pace edit: entry 0193a8d0-000a-7000-8000-000000000001, logged on another device."
        );
        assert_eq!(
            lines[breaks..breaks + 4],
            [
                "# Held 09:10–09:20: phone call.",
                "# Paused since 09:40.",
                "# Caught at 09:30: call Anna",
                "# Holds, pauses and catches are shown here, not edited.",
            ],
            "{text}"
        );
    }

    #[test]
    fn written_entry_without_note_offers_note_line() {
        let text = written(Entry {
            note: None,
            ..entry()
        });

        assert!(text.lines().any(|line| line == "# note = \"\""), "{text}");
        assert!(!text.lines().any(|line| line.starts_with("note")), "{text}");
    }

    #[test]
    fn written_project_reads_back_as_same_project() {
        let id: ProjectId = "0193a8d0-000c-7000-8000-000000000001".parse().unwrap();
        for project in [ProjectRef::Id(id), ProjectRef::Key("acme".parse().unwrap())] {
            let entry = Entry {
                project: Some(project),
                ..entry()
            };
            let text = written(entry.clone());

            let error = read_adjustment(
                &text,
                &entry,
                &at("10:05"),
                &Scripted::without_terminal(),
                &mut Vec::new(),
            )
            .unwrap_err();

            assert_eq!(error.to_string(), "the entry is unchanged", "{text}");
            assert!(!text.contains("# project-key"), "{text}");
        }
    }

    #[test]
    fn edited_project_and_project_key_together_are_refused() {
        let text =
            saved("project = \"0193a8d0-000c-7000-8000-000000000001\"\nproject-key = \"acme\"");

        let error = read_adjustment(
            &text,
            &entry(),
            &at("10:05"),
            &Scripted::without_terminal(),
            &mut Vec::new(),
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "the edited entry has `project` and `project-key`; it needs one"
        );
        assert_eq!(error.hint(), Some("keep one of them; nothing was changed"));
    }

    #[test]
    fn edited_project_id_sets_project() {
        let id: ProjectId = "0193a8d0-000c-7000-8000-000000000001".parse().unwrap();
        let text = saved(&format!("note = \"Draft\"\nproject = \"{id}\""));

        let adjust = read_adjustment(
            &text,
            &entry(),
            &at("10:05"),
            &Scripted::without_terminal(),
            &mut Vec::new(),
        )
        .unwrap();

        assert_eq!(adjust.project, Change::Set(ProjectRef::Id(id)));
    }

    #[test]
    fn removed_end_of_ended_entry_is_refused() {
        let text = format!("start = \"{}\"\nnote = \"Draft\"\n", at("09:00"));

        let error = read_adjustment(
            &text,
            &entry(),
            &at("10:05"),
            &Scripted::without_terminal(),
            &mut Vec::new(),
        )
        .unwrap_err();

        assert_eq!(error.to_string(), "the entry has ended, so it keeps an end");
        assert_eq!(
            error.hint(),
            Some("move the end instead of removing it; nothing was changed")
        );
    }
}
