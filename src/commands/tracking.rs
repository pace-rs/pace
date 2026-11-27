//! The tracking commands: `begin`, `hold`, `pause`, `resume`, `catch`, `end`.

use std::collections::BTreeSet;

use abscissa_core::{Command, Runnable};
use jiff::ZonedDateTime;
use pace_core::{
    Command as UseCase, Entry, HoldSource, PauseKind, Tag, Text,
    command::{Begin, Catch, End, Hold, Pause, Resume},
};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_time::{At, TimeRange, parse_at, relative_time};

use crate::commands::now::recent_range_start;
use crate::output::{describe_outcome, project_name};
use crate::print::print_lines;
use crate::prompt::Ask;
use crate::select::{Target, target_entry};
use crate::{
    application::{Context, PACE_APP, exit_on_error},
    project::{Project, Reason},
    session::Session,
};

/// Run `build` with the session and the context, print what happened, the
/// notes `build` left, and the first-use note.
pub(super) fn track(
    build: impl FnOnce(&Session, &Context, &mut Vec<String>) -> PaceResult<UseCase>,
) -> PaceResult<()> {
    let context = PACE_APP.context()?;
    let session = Session::open(context)?;
    let mut notes = Vec::new();
    let command = build(&session, context, &mut notes)?;
    let outcome = session.run(&command)?;
    print_lines(
        describe_outcome(&outcome)?
            .into_iter()
            .chain(notes)
            .chain(session.first_use_note()),
    )
}

/// A `--at` value as a moment, if there is one.
pub(super) fn at(
    value: Option<&str>,
    now: &ZonedDateTime,
    asker: &dyn Ask,
    notes: &mut Vec<String>,
) -> PaceResult<Option<ZonedDateTime>> {
    value
        .map(|value| moment(value, now, asker, notes))
        .transpose()
}

/// A time in the `--at` grammar as a moment, chosen as
/// [`chosen_moment`] says.
pub(super) fn moment(
    value: &str,
    now: &ZonedDateTime,
    asker: &dyn Ask,
    notes: &mut Vec<String>,
) -> PaceResult<ZonedDateTime> {
    let at = parse_at(value, now).map_err(|error| error.attach_exit_code(ExitCode::Usage))?;
    chosen_moment(value, at, asker, notes)
}

/// The moment `value` means. A time that happened twice asks which one was
/// meant; without a terminal, it takes the earlier one and says so.
pub(super) fn chosen_moment(
    value: &str,
    at: At,
    asker: &dyn Ask,
    notes: &mut Vec<String>,
) -> PaceResult<ZonedDateTime> {
    match at {
        At::Exact(moment) => Ok(moment),
        At::Ambiguous { earlier, later } => {
            let label = |moment: &ZonedDateTime| moment.strftime("%H:%M (%:z)").to_string();
            if asker.can_ask() {
                let choice = asker.choose(
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

/// The words of a note; without words, no note.
pub(super) fn text(words: &[String]) -> Option<Text> {
    Text::try_from(words.join(" ")).ok()
}

/// The text of `--reason`, refused without words.
pub(super) fn reason(value: Option<&str>) -> PaceResult<Option<Text>> {
    value
        .map(|value| {
            value.parse().map_err(|error: Box<PaceError>| {
                error
                    .attach_hint("say why, like `--reason \"phone call\"`, or leave out `--reason`")
                    .attach_exit_code(ExitCode::Usage)
            })
        })
        .transpose()
}

/// The words of a caught thought, refused without words.
fn thought(words: &[String]) -> PaceResult<Text> {
    Text::try_from(words.join(" ")).map_err(|error| {
        error
            .attach_hint("write the thought down, like `pace catch call the bank`")
            .attach_exit_code(ExitCode::Usage)
    })
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
        conflicts_with_all = ["note", "tags", "project"],
        help = "Begin a recent entry again: same note, project and tags"
    )]
    again: bool,
}

impl Runnable for BeginCmd {
    fn run(&self) {
        exit_on_error(track(|session, context, notes| {
            let at = at(self.at.as_deref(), &session.now(), &context.terminal, notes)?;
            if self.again {
                refuse_project_flag(context.project.as_ref())?;
                let count = context.settings.settings.most_recent_count;
                let again = again(session, &context.terminal, count)?;
                if let Some(project) = &again.project {
                    notes.push(format!(
                        "Project: {} (from the earlier entry).",
                        project_name(project)
                    ));
                }
                return Ok(UseCase::Begin(Begin {
                    at,
                    allow_overlap: self.allow_overlap,
                    ..again
                }));
            }
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

/// `--again` takes the project of the earlier entry, so it refuses
/// `--project`; clap only sees the flag after the subcommand.
fn refuse_project_flag(project: Option<&Project>) -> PaceResult<()> {
    match project {
        Some(project) if project.reason == Reason::Flag => Err(PaceError::new(
            ErrorKind::InvalidInput,
            "`pace begin --again` takes the project of the earlier entry, not `--project {key}`",
        )
        .attach_context("key", project.key.to_string())
        .attach_hint("leave out `--project`, or begin a new entry with `pace begin <note>`")
        .attach_exit_code(ExitCode::Usage)),
        _ => Ok(()),
    }
}

/// Note, project and tags of a recent entry, picked by the user.
fn again(session: &Session, asker: &dyn Ask, count: usize) -> PaceResult<Begin> {
    let now = session.now();
    let recent = TimeRange::new(Some(recent_range_start(&now)), Some(now.clone()))?;
    let state = session.state(&recent)?;
    let mut candidates: Vec<Begin> = Vec::new();
    let mut entries: Vec<&Entry> = state.entries().filter(|entry| !entry.is_open()).collect();
    entries.sort_by(|a, b| b.start.cmp(&a.start));
    let mut labels = Vec::new();
    for entry in entries {
        let candidate = Begin {
            note: entry
                .note
                .clone()
                .and_then(|note| Text::try_from(note).ok()),
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
        _ if asker.can_ask() => {
            let choice = asker.choose("Begin which entry again?", &labels)?;
            Ok(candidates.swap_remove(choice))
        }
        count => Err(PaceError::new(
            ErrorKind::InvalidInput,
            "`pace begin --again` needs a choice of {count} recent entries:\n{list}",
        )
        .attach_context("count", count.to_string())
        .attach_context("list", labels.join("\n"))
        .attach_hint("run it in a terminal to pick one, or begin with `pace begin <note>`")),
    }
}

fn label(entry: &Entry, now: &ZonedDateTime) -> String {
    let mut parts = vec![
        entry
            .note
            .as_ref()
            .map_or_else(|| "(no note)".to_owned(), |note| format!("\"{note}\"")),
    ];
    if let Some(project) = &entry.project {
        parts.push(format!("project {}", project_name(project)));
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
        exit_on_error(track(|session, context, notes| {
            let source = match (self.inner, self.outer) {
                (true, _) => Some(HoldSource::Inner),
                (_, true) => Some(HoldSource::Outer),
                _ => None,
            };
            Ok(UseCase::Hold(Hold {
                at: at(self.at.as_deref(), &session.now(), &context.terminal, notes)?,
                source,
                reason: reason(self.reason.as_deref())?,
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
        exit_on_error(track(|session, context, notes| {
            Ok(UseCase::Pause(Pause {
                at: at(self.at.as_deref(), &session.now(), &context.terminal, notes)?,
                kind: if self.lunch {
                    PauseKind::Lunch
                } else {
                    PauseKind::Break
                },
                reason: reason(self.reason.as_deref())?,
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
        exit_on_error(track(|session, context, notes| {
            Ok(UseCase::Resume(Resume {
                at: at(self.at.as_deref(), &session.now(), &context.terminal, notes)?,
                entry: target_entry(
                    session,
                    &context.terminal,
                    Target::id_or_open(self.entry.as_deref()),
                )?,
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
        exit_on_error(track(|_, _, _| {
            Ok(UseCase::Catch(Catch {
                text: thought(&self.thought)?,
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
        exit_on_error(track(|session, context, notes| {
            Ok(UseCase::End(End {
                at: at(self.at.as_deref(), &session.now(), &context.terminal, notes)?,
                entry: target_entry(
                    session,
                    &context.terminal,
                    Target::id_or_open(self.entry.as_deref()),
                )?,
            }))
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pace_core::ProjectRef;

    use crate::prompt::Scripted;

    /// Noon on the day the clocks in Berlin went back from 03:00 to 02:00.
    fn clocks_set_back() -> ZonedDateTime {
        "2026-10-25T12:00[Europe/Berlin]".parse().unwrap()
    }

    #[test]
    fn label_names_project_by_key_or_by_id() {
        let now: ZonedDateTime = "2026-11-26T12:00[Europe/Berlin]".parse().unwrap();
        let entry = |project| Entry {
            id: "0193a8d0-000a-7000-8000-000000000001".parse().unwrap(),
            device: "0193a8d0-000d-7000-8000-000000000001".parse().unwrap(),
            start: "2026-11-26T09:00[Europe/Berlin]".parse().unwrap(),
            end: Some("2026-11-26T10:00[Europe/Berlin]".parse().unwrap()),
            project: Some(project),
            note: Some("Standup".to_owned()),
            tags: BTreeSet::new(),
            holds: Vec::new(),
            pauses: Vec::new(),
            catches: Vec::new(),
            origin: pace_core::EntryOrigin::Live,
            filed: "2026-11-26T09:00[Europe/Berlin]".parse().unwrap(),
        };
        let id = "0193a7c4-6c1e-7f3a-9b2d-4e5f60718293";

        assert_eq!(
            label(&entry(ProjectRef::Key("gamma".parse().unwrap())), &now),
            "\"Standup\", project gamma, last 3 hours ago"
        );
        assert_eq!(
            label(&entry(ProjectRef::Id(id.parse().unwrap())), &now),
            format!("\"Standup\", project {id}, last 3 hours ago")
        );
    }

    #[test]
    fn time_that_happened_twice_is_the_one_user_chose() {
        let asker = Scripted::choosing([1]);
        let mut notes = Vec::new();

        let moment = moment("02:30", &clocks_set_back(), &asker, &mut notes).unwrap();

        assert_eq!(
            moment.to_string(),
            "2026-10-25T02:30:00+01:00[Europe/Berlin]"
        );
        assert_eq!(
            asker.offered(),
            [vec![
                "the first 02:30 (+02:00)".to_owned(),
                "the second 02:30 (+01:00)".to_owned(),
            ]]
        );
        assert_eq!(notes, Vec::<String>::new());
    }

    #[test]
    fn time_that_happened_twice_without_terminal_is_first_with_note() {
        let asker = Scripted::without_terminal();
        let mut notes = Vec::new();

        let moment = moment("02:30", &clocks_set_back(), &asker, &mut notes).unwrap();

        assert_eq!(
            moment.to_string(),
            "2026-10-25T02:30:00+02:00[Europe/Berlin]"
        );
        assert_eq!(notes.len(), 1);
        assert_eq!(asker.questions(), Vec::<String>::new());
    }
}
