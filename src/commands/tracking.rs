//! The tracking commands: `begin`, `hold`, `pause`, `resume`, `catch`, `end`.

use std::collections::BTreeSet;

use abscissa_core::{Command, Runnable};
use jiff::ZonedDateTime;
use pace_core::{
    Command as UseCase, Entry, HoldSource, PauseKind, ProjectRef, Tag,
    command::{Begin, Catch, End, Hold, Pause, Resume},
};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_time::{At, TimeRange, parse_at, relative_time};

use crate::commands::now::recent_range_start;
use crate::output::describe_outcome;
use crate::prompt::{choose, interactive};
use crate::select::target_entry;
use crate::{
    application::{PACE_APP, exit_on_error},
    session::Session,
};

/// Run `build` with the session, print what happened, the notes `build`
/// left, and the first-use note.
pub(super) fn track(
    build: impl FnOnce(&Session, &mut Vec<String>) -> PaceResult<UseCase>,
) -> PaceResult<()> {
    let context = PACE_APP.context()?;
    let session = Session::open(context)?;
    let mut notes = Vec::new();
    let command = build(&session, &mut notes)?;
    let outcome = session.run(&command)?;
    for line in describe_outcome(&outcome) {
        println!("{line}");
    }
    for note in notes.iter().chain(session.first_use_note().iter()) {
        println!("{note}");
    }
    Ok(())
}

/// A `--at` value as a moment, if there is one.
pub(super) fn at(
    value: Option<&str>,
    now: &ZonedDateTime,
    notes: &mut Vec<String>,
) -> PaceResult<Option<ZonedDateTime>> {
    value.map(|value| moment(value, now, notes)).transpose()
}

/// A time in the `--at` grammar as a moment. A time that happened twice
/// asks which one was meant; without a terminal, it takes the earlier one
/// and says so.
pub(super) fn moment(
    value: &str,
    now: &ZonedDateTime,
    notes: &mut Vec<String>,
) -> PaceResult<ZonedDateTime> {
    match parse_at(value, now).map_err(|error| error.attach_exit_code(ExitCode::Usage))? {
        At::Exact(moment) => Ok(moment),
        At::Ambiguous { earlier, later } => {
            let label = |moment: &ZonedDateTime| moment.strftime("%H:%M (%:z)").to_string();
            if interactive() {
                let choice = choose(
                    &format!(
                        "{value} happened twice, because the clocks were set back. Which one?"
                    ),
                    &[
                        format!("the first {}", label(&earlier)),
                        format!("the second {}", label(&later)),
                    ],
                )?;
                return Ok(if choice == 0 { earlier } else { later });
            }
            notes.push(format!(
                "{value} happened twice, because the clocks were set back; pace took the first one, {}, not {}.",
                label(&earlier),
                label(&later)
            ));
            Ok(earlier)
        }
    }
}

pub(super) fn text(words: &[String]) -> Option<String> {
    let text = words.join(" ");
    (!text.trim().is_empty()).then_some(text)
}

pub(super) fn tags(values: &[String]) -> PaceResult<BTreeSet<Tag>> {
    values
        .iter()
        .map(|value| {
            value
                .parse()
                .map_err(|error: Box<PaceError>| error.attach_exit_code(ExitCode::Usage))
        })
        .collect()
}

/// `pace begin`
#[derive(clap::Args, Command, Debug)]
pub struct BeginCmd {
    #[arg(value_name = "NOTE", help = "What you work on")]
    note: Vec<String>,

    #[arg(
        long,
        value_name = "TIME",
        allow_hyphen_values = true,
        help = "When you began: HH:MM, -15m, yesterday HH:MM, YYYY-MM-DD HH:MM"
    )]
    at: Option<String>,

    #[arg(
        long = "tag",
        value_name = "TAG",
        help = "A tag for the entry; repeat for more"
    )]
    tags: Vec<String>,

    #[arg(long, help = "Allow the entry to overlap another one")]
    allow_overlap: bool,

    #[arg(
        long,
        conflicts_with_all = ["note", "tags"],
        help = "Begin a recent entry again: same note, project and tags"
    )]
    again: bool,
}

impl Runnable for BeginCmd {
    fn run(&self) {
        exit_on_error(track(|session, notes| {
            let at = at(self.at.as_deref(), &session.now(), notes)?;
            if self.again {
                let count = PACE_APP.context()?.settings.settings.most_recent_count;
                let again = again(session, count)?;
                return Ok(UseCase::Begin(Begin {
                    at,
                    allow_overlap: self.allow_overlap,
                    ..again
                }));
            }
            let context = PACE_APP.context()?;
            let project = context.project.as_ref().map(|project| {
                notes.push(format!("Project: {} ({}).", project.key, project.reason));
                project.reference()
            });
            Ok(UseCase::Begin(Begin {
                at,
                note: text(&self.note),
                project,
                tags: tags(&self.tags)?,
                allow_overlap: self.allow_overlap,
            }))
        }));
    }
}

/// Note, project and tags of a recent entry, picked by the user.
fn again(session: &Session, count: usize) -> PaceResult<Begin> {
    let now = session.now();
    let recent = TimeRange::new(Some(recent_range_start(&now)), Some(now.clone()))?;
    let state = session.state(&recent)?;
    let mut candidates: Vec<Begin> = Vec::new();
    let mut entries: Vec<&Entry> = state.entries().filter(|entry| !entry.is_open()).collect();
    entries.sort_by(|a, b| b.start.cmp(&a.start));
    let mut labels = Vec::new();
    for entry in entries {
        let candidate = Begin {
            note: entry.note.clone(),
            project: entry.project.clone(),
            tags: entry.tags.clone(),
            ..Begin::default()
        };
        if candidates.contains(&candidate) {
            continue;
        }
        labels.push(label(entry, &now));
        candidates.push(candidate);
        if candidates.len() == count {
            break;
        }
    }

    match candidates.len() {
        0 => Err(PaceError::new(
            ErrorKind::InvalidInput,
            "there is no recent entry to begin again",
        )
        .attach_hint("`pace begin <note>` begins a new one")
        .attach_exit_code(ExitCode::NothingToDo)),
        1 => Ok(candidates.remove(0)),
        _ if interactive() => {
            let choice = choose("Begin which entry again?", &labels)?;
            Ok(candidates.swap_remove(choice))
        }
        count => {
            let list: Vec<String> = labels
                .iter()
                .enumerate()
                .map(|(index, label)| format!("{}. {label}", index + 1))
                .collect();
            Err(PaceError::new(
                ErrorKind::InvalidInput,
                "`pace begin --again` needs a choice of {count} recent entries:\n{list}",
            )
            .attach_context("count", count.to_string())
            .attach_context("list", list.join("\n"))
            .attach_hint("run it in a terminal to pick one, or begin with `pace begin <note>`"))
        }
    }
}

fn label(entry: &Entry, now: &ZonedDateTime) -> String {
    let mut parts = vec![
        entry
            .note
            .as_ref()
            .map_or_else(|| "(no note)".to_owned(), |note| format!("\"{note}\"")),
    ];
    if let Some(ProjectRef::Key(key)) = &entry.project {
        parts.push(format!("project {key}"));
    }
    if !entry.tags.is_empty() {
        let tags: Vec<&str> = entry.tags.iter().map(Tag::as_str).collect();
        parts.push(format!("tags {}", tags.join(", ")));
    }
    parts.push(format!("last {}", relative_time(&entry.start, now)));
    parts.join(", ")
}

/// `pace hold`
#[derive(clap::Args, Command, Debug)]
pub struct HoldCmd {
    #[arg(long, conflicts_with = "outer", help = "You broke off yourself")]
    inner: bool,

    #[arg(long, help = "Someone else interrupted you")]
    outer: bool,

    #[arg(long, value_name = "TEXT", help = "Why, for example \"phone call\"")]
    reason: Option<String>,

    #[arg(
        long,
        value_name = "TIME",
        allow_hyphen_values = true,
        help = "When the interruption began"
    )]
    at: Option<String>,
}

impl Runnable for HoldCmd {
    fn run(&self) {
        exit_on_error(track(|session, notes| {
            let source = match (self.inner, self.outer) {
                (true, _) => Some(HoldSource::Inner),
                (_, true) => Some(HoldSource::Outer),
                _ => None,
            };
            Ok(UseCase::Hold(Hold {
                at: at(self.at.as_deref(), &session.now(), notes)?,
                source,
                reason: self.reason.clone(),
            }))
        }));
    }
}

/// `pace pause`
#[derive(clap::Args, Command, Debug)]
pub struct PauseCmd {
    #[arg(long, help = "A lunch break")]
    lunch: bool,

    #[arg(long, value_name = "TEXT", help = "Why")]
    reason: Option<String>,

    #[arg(
        long,
        value_name = "TIME",
        allow_hyphen_values = true,
        help = "When the break began"
    )]
    at: Option<String>,
}

impl Runnable for PauseCmd {
    fn run(&self) {
        exit_on_error(track(|session, notes| {
            Ok(UseCase::Pause(Pause {
                at: at(self.at.as_deref(), &session.now(), notes)?,
                kind: if self.lunch {
                    PauseKind::Lunch
                } else {
                    PauseKind::Break
                },
                reason: self.reason.clone(),
            }))
        }));
    }
}

/// `pace resume`
#[derive(clap::Args, Command, Debug)]
pub struct ResumeCmd {
    #[arg(
        value_name = "ENTRY",
        help = "The entry, by its short ID; the open one if left out"
    )]
    entry: Option<String>,

    #[arg(
        long,
        value_name = "TIME",
        allow_hyphen_values = true,
        help = "When you went on"
    )]
    at: Option<String>,
}

impl Runnable for ResumeCmd {
    fn run(&self) {
        exit_on_error(track(|session, notes| {
            Ok(UseCase::Resume(Resume {
                at: at(self.at.as_deref(), &session.now(), notes)?,
                entry: target_entry(session, self.entry.as_deref(), false)?,
            }))
        }));
    }
}

/// `pace catch`
#[derive(clap::Args, Command, Debug)]
pub struct CatchCmd {
    #[arg(
        value_name = "THOUGHT",
        required = true,
        help = "The thought to write down"
    )]
    thought: Vec<String>,
}

impl Runnable for CatchCmd {
    fn run(&self) {
        exit_on_error(track(|_, _| {
            Ok(UseCase::Catch(Catch {
                text: text(&self.thought).unwrap_or_default(),
            }))
        }));
    }
}

/// `pace end`
#[derive(clap::Args, Command, Debug)]
pub struct EndCmd {
    #[arg(
        value_name = "ENTRY",
        help = "The entry, by its short ID; the open one if left out"
    )]
    entry: Option<String>,

    #[arg(
        long,
        value_name = "TIME",
        allow_hyphen_values = true,
        help = "When the entry ended"
    )]
    at: Option<String>,
}

impl Runnable for EndCmd {
    fn run(&self) {
        exit_on_error(track(|session, notes| {
            Ok(UseCase::End(End {
                at: at(self.at.as_deref(), &session.now(), notes)?,
                entry: target_entry(session, self.entry.as_deref(), false)?,
            }))
        }));
    }
}
