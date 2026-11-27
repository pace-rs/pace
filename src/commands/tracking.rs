//! The tracking commands: `begin`, `hold`, `pause`, `resume`, `catch`, `end`.

use std::collections::BTreeSet;

use abscissa_core::{Command, Runnable};
use jiff::ZonedDateTime;
use pace_core::{
    Catalog, Command as UseCase, Entry, HoldSource, Outcome, PauseKind, State, Tag, Text,
    command::{Begin, Catch, End, Hold, Pause, Resume},
};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_time::{At, TimeRange, parse_at, relative_time};

use crate::commands::entry_fields::{BillableFlags, DecidedFlags, entry_role};
use crate::commands::now::recent_range_start;
use crate::output::{billable_label, decided_label, describe_outcome, project_label, role_label};
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
    track_run(|session, context, notes| session.run(&build(session, context, notes)?))
}

/// Like [`track`], but `run` runs the command itself, so it may try again
/// when the rules refuse one.
pub(super) fn track_run(
    run: impl FnOnce(&Session, &Context, &mut Vec<String>) -> PaceResult<Outcome>,
) -> PaceResult<()> {
    let context = PACE_APP.context()?;
    let session = Session::open(context)?;
    let mut notes = Vec::new();
    let outcome = run(&session, context, &mut notes)?;
    let warnings = context
        .project
        .iter()
        .flat_map(|project| project.warnings.iter().cloned());
    print_lines(
        describe_outcome(&outcome, &context.catalog)?
            .into_iter()
            .chain(notes)
            .chain(warnings)
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
        .map(|value| parse_moment(value, now, asker, notes))
        .transpose()
}

/// A time in the `--at` grammar as a moment, chosen as
/// [`chosen_moment`] says.
pub(super) fn parse_moment(
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
                    0,
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

/// The tags of a new entry: those of `--tag`, and the `default-tags` of
/// the manifest that links the directory to `project`.
pub(super) fn entry_tags(
    values: &[String],
    project: Option<&Project>,
) -> PaceResult<BTreeSet<Tag>> {
    let mut tags = tags(values)?;
    tags.extend(
        project
            .into_iter()
            .flat_map(|project| project.default_tags.iter().cloned()),
    );
    Ok(tags)
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

    #[arg(
        long,
        value_name = "KEY",
        help = "The role its time goes to; the project's if left out"
    )]
    role: Option<String>,

    #[command(flatten)]
    decided: DecidedFlags,

    #[command(flatten)]
    billable: BillableFlags,

    #[arg(long, help = "Allow the entry to overlap another one")]
    allow_overlap: bool,

    #[arg(
        long,
        conflicts_with_all = ["note", "tags", "project"],
        help = "Begin a recent entry again: same note, project, tags, role, who decided and billable"
    )]
    again: bool,
}

impl Runnable for BeginCmd {
    fn run(&self) {
        exit_on_error(track(|session, context, notes| {
            let at = at(self.at.as_deref(), &session.now(), &context.terminal, notes)?;
            let role = entry_role(self.role.as_deref(), &context.catalog, notes)?;
            let determined_by = self.decided.determined_by();
            let billable = self.billable.billable();
            if self.again {
                refuse_project_flag(context.project.as_ref())?;
                let count = context.settings.settings.most_recent_count;
                let again = again(session, context, count)?;
                if let Some(project) = &again.project {
                    notes.push(format!(
                        "Project: {} (from the earlier entry).",
                        project_label(project, &context.catalog)
                    ));
                }
                return Ok(UseCase::Begin(Begin {
                    at,
                    role: role.or(again.role),
                    determined_by: determined_by.or(again.determined_by),
                    billable: billable.or(again.billable),
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
                tags: entry_tags(&self.tags, context.project.as_ref())?,
                role,
                determined_by,
                billable,
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

/// Note, project, tags, role, who decided and billable of a recent entry,
/// picked by the user.
fn again(session: &Session, context: &Context, count: usize) -> PaceResult<Begin> {
    let now = session.now();
    let recent = TimeRange::new(Some(recent_range_start(&now)), Some(now.clone()))?;
    again_from(
        &session.state(&recent)?,
        &now,
        &context.terminal,
        count,
        &context.catalog,
    )
}

/// Note, project, tags, role, who decided and billable of one of the
/// `count` most recent entries of `state` that differ in them: the only
/// one, or the one the user picks. A project or a role with a record in
/// `catalog` shows its name.
fn again_from(
    state: &State,
    now: &ZonedDateTime,
    asker: &dyn Ask,
    count: usize,
    catalog: &Catalog,
) -> PaceResult<Begin> {
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
            role: entry.role,
            determined_by: entry.determined_by,
            billable: entry.billable,
            ..Begin::default()
        };
        if candidates.contains(&candidate) {
            continue;
        }
        labels.push(label(entry, now, catalog));
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
            let choice = asker.choose("Begin which entry again?", &labels, 0)?;
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

fn label(entry: &Entry, now: &ZonedDateTime, catalog: &Catalog) -> String {
    let mut parts = vec![
        entry
            .note
            .as_ref()
            .map_or_else(|| "(no note)".to_owned(), |note| format!("\"{note}\"")),
    ];
    if let Some(project) = &entry.project {
        parts.push(format!("project {}", project_label(project, catalog)));
    }
    if !entry.tags.is_empty() {
        let tags: Vec<&str> = entry.tags.iter().map(Tag::as_str).collect();
        parts.push(format!("tags {}", tags.join(", ")));
    }
    if let Some(role) = entry.role {
        parts.push(format!("role {}", role_label(role, catalog)));
    }
    parts.extend(entry.determined_by.map(decided_label).map(str::to_owned));
    parts.extend(entry.billable.map(billable_label).map(str::to_owned));
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
    use pace_core::{ProjectRecord, ProjectRef};

    use crate::prompt::Scripted;

    /// Noon on the day the clocks in Berlin went back from 03:00 to 02:00.
    fn clocks_set_back() -> ZonedDateTime {
        "2026-10-25T12:00[Europe/Berlin]".parse().unwrap()
    }

    #[test]
    fn label_names_project_by_record_name_else_key_or_id() {
        let now: ZonedDateTime = "2026-11-26T12:00[Europe/Berlin]".parse().unwrap();
        let entry = |project| Entry {
            id: "0193a8d0-000a-7000-8000-000000000001".parse().unwrap(),
            device: "0193a8d0-000d-7000-8000-000000000001".parse().unwrap(),
            start: "2026-11-26T09:00[Europe/Berlin]".parse().unwrap(),
            end: Some("2026-11-26T10:00[Europe/Berlin]".parse().unwrap()),
            project: Some(project),
            note: Some("Standup".to_owned()),
            tags: BTreeSet::new(),
            role: None,
            determined_by: None,
            billable: None,
            allow_overlap: false,
            holds: Vec::new(),
            pauses: Vec::new(),
            catches: Vec::new(),
            origin: pace_core::EntryOrigin::Live,
            filed: "2026-11-26T09:00[Europe/Berlin]".parse().unwrap(),
        };
        let id = "0193a7c4-6c1e-7f3a-9b2d-4e5f60718293";

        assert_eq!(
            label(
                &entry(ProjectRef::Key("gamma".parse().unwrap())),
                &now,
                &Catalog::default()
            ),
            "\"Standup\", project gamma, last 3 hours ago"
        );
        assert_eq!(
            label(
                &entry(ProjectRef::Id(id.parse().unwrap())),
                &now,
                &Catalog::default()
            ),
            format!("\"Standup\", project {id}, last 3 hours ago")
        );
        let record = ProjectRecord {
            id: id.parse().unwrap(),
            key: "gamma".parse().unwrap(),
            name: "Gamma relaunch".to_owned(),
            client: None,
            role: None,
            billable: false,
            budget: None,
            starts_on: None,
            ends_on: None,
            done_when: None,
            allow_overlap: false,
            archived: None,
        };
        let catalog = Catalog::new(Vec::new(), Vec::new(), vec![record]).unwrap();
        assert_eq!(
            label(&entry(ProjectRef::Id(id.parse().unwrap())), &now, &catalog),
            "\"Standup\", project Gamma relaunch, last 3 hours ago"
        );
    }

    #[test]
    fn label_names_own_role_who_decided_and_billable() {
        let now: ZonedDateTime = "2026-11-26T12:00[Europe/Berlin]".parse().unwrap();
        let role_id = "0193a7c4-0000-7000-8000-0000000000a1";
        let entry = Entry {
            id: "0193a8d0-000a-7000-8000-000000000001".parse().unwrap(),
            device: "0193a8d0-000d-7000-8000-000000000001".parse().unwrap(),
            start: "2026-11-26T09:00[Europe/Berlin]".parse().unwrap(),
            end: Some("2026-11-26T10:00[Europe/Berlin]".parse().unwrap()),
            project: None,
            note: Some("Standup".to_owned()),
            tags: BTreeSet::new(),
            role: Some(role_id.parse().unwrap()),
            determined_by: Some(pace_core::DeterminedBy::Others),
            billable: Some(false),
            allow_overlap: false,
            holds: Vec::new(),
            pauses: Vec::new(),
            catches: Vec::new(),
            origin: pace_core::EntryOrigin::Live,
            filed: "2026-11-26T09:00[Europe/Berlin]".parse().unwrap(),
        };
        let role = pace_core::Role {
            id: role_id.parse().unwrap(),
            key: "work".parse().unwrap(),
            kind: pace_core::RoleKind::Area,
            name: "Work".to_owned(),
            purpose: None,
            weekly_target: None,
            archived: None,
        };
        let catalog = Catalog::new(vec![role], Vec::new(), Vec::new()).unwrap();

        assert_eq!(
            label(&entry, &now, &catalog),
            "\"Standup\", role Work, by others, not billable, last 3 hours ago"
        );
        assert_eq!(
            label(&entry, &now, &Catalog::default()),
            format!("\"Standup\", role {role_id}, by others, not billable, last 3 hours ago")
        );
    }

    #[test]
    fn time_that_happened_twice_is_the_one_user_chose() {
        let asker = Scripted::choosing([1]);
        let mut notes = Vec::new();

        let moment = parse_moment("02:30", &clocks_set_back(), &asker, &mut notes).unwrap();

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

        let moment = parse_moment("02:30", &clocks_set_back(), &asker, &mut notes).unwrap();

        assert_eq!(
            moment.to_string(),
            "2026-10-25T02:30:00+02:00[Europe/Berlin]"
        );
        assert_eq!(notes.len(), 1);
        assert_eq!(asker.questions(), Vec::<String>::new());
    }

    /// Entry `number`, begun at `start` with `note` and ended at `end`, on
    /// 2026-11-26.
    fn ended(number: u32, start: &str, end: &str, note: &str) -> Vec<pace_core::RecordedEvent> {
        let id =
            |prefix: u16, number: u32| format!("0193a8d0-{prefix:04x}-7000-8000-{number:012x}");
        let at = |time: &str| -> ZonedDateTime {
            format!("2026-11-26T{time}[Europe/Berlin]").parse().unwrap()
        };
        let entry: pace_core::EntryId = id(0xa, number).parse().unwrap();
        let event = |sequence: u32, time: &str, kind| pace_core::RecordedEvent {
            device: id(0xd, 1).parse().unwrap(),
            batch: id(0xb, number * 2 + sequence).parse().unwrap(),
            hlc: pace_core::Hlc::new(at(time).timestamp(), number * 2 + sequence),
            event: pace_core::Event {
                id: id(0xe, number * 2 + sequence).parse().unwrap(),
                at: at(time),
                about: at(time),
                kind,
            },
        };
        vec![
            event(
                0,
                start,
                pace_core::EventKind::EntryBegun {
                    entry,
                    project: None,
                    note: Some(note.to_owned()),
                    tags: BTreeSet::new(),
                    role: None,
                    determined_by: None,
                    billable: None,
                    allow_overlap: false,
                },
            ),
            event(1, end, pace_core::EventKind::EntryEnded { entry }),
        ]
    }

    #[test]
    fn begin_again_on_terminal_begins_entry_user_picked() {
        let mut events = ended(1, "09:00", "10:00", "Write offer");
        events.extend(ended(2, "10:00", "11:00", "Call ACME"));
        let state = State::from_events(&events);
        let now: ZonedDateTime = "2026-11-26T12:00[Europe/Berlin]".parse().unwrap();
        let asker = Scripted::choosing([1]);

        let begin = again_from(&state, &now, &asker, 5, &Catalog::default()).unwrap();

        assert_eq!(begin.note, Some("Write offer".parse().unwrap()));
        assert_eq!(asker.questions(), ["Begin which entry again?"]);
        assert_eq!(
            asker.offered(),
            [vec![
                "\"Call ACME\", last 2 hours ago".to_owned(),
                "\"Write offer\", last 3 hours ago".to_owned(),
            ]]
        );
    }
}
