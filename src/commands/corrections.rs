//! Correcting the record: `adjust`, `log`, `undo`, `forget`.

use abscissa_core::{Command, Runnable};
use pace_core::{
    Change, Command as UseCase, ProjectRef,
    command::{Adjust, Forget, Log},
};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_time::parse_duration;

use super::tracking::{at, moment, tags, text, track};
use crate::{
    application::{PACE_APP, exit_on_error},
    output,
    project::Reason,
    prompt, select,
};

/// `pace adjust`
#[derive(clap::Args, Command, Debug)]
pub struct AdjustCmd {
    #[arg(
        value_name = "ENTRY",
        conflicts_with = "last",
        help = "The entry, by its short ID; the open one if left out"
    )]
    entry: Option<String>,

    #[arg(long, help = "The entry that ended last")]
    last: bool,

    #[arg(
        long,
        value_name = "TIME",
        allow_hyphen_values = true,
        help = "A new start"
    )]
    start: Option<String>,

    #[arg(
        long,
        value_name = "TIME",
        allow_hyphen_values = true,
        help = "A new end, for an entry that has ended"
    )]
    end: Option<String>,

    #[arg(long, value_name = "TEXT", help = "A new note; \"\" removes it")]
    note: Option<String>,

    #[arg(long = "tag", value_name = "TAG", help = "Add a tag; repeat for more")]
    tags: Vec<String>,

    #[arg(
        long = "untag",
        value_name = "TAG",
        help = "Remove a tag; repeat for more"
    )]
    untags: Vec<String>,

    #[arg(long, help = "Allow the entry to overlap another one")]
    allow_overlap: bool,
}

impl Runnable for AdjustCmd {
    fn run(&self) {
        exit_on_error(track(|session, notes| {
            let now = session.now();
            // Only an explicit `--project` moves the entry; a discovered
            // project or PACE_PROJECT doesn't.
            let project = match &PACE_APP.context()?.project {
                Some(project) if project.reason == Reason::Flag => {
                    Change::Set(ProjectRef::Key(project.key.clone()))
                }
                _ => Change::Keep,
            };
            let note = match self.note.as_deref() {
                None => Change::Keep,
                Some(note) if note.trim().is_empty() => Change::Clear,
                Some(note) => Change::Set(note.to_owned()),
            };
            Ok(UseCase::Adjust(Adjust {
                entry: select::target(session, self.entry.as_deref(), self.last)?,
                start: at(self.start.as_deref(), &now, notes)?,
                end: at(self.end.as_deref(), &now, notes)?,
                note,
                project,
                tag: tags(&self.tags)?,
                untag: tags(&self.untags)?,
                allow_overlap: self.allow_overlap,
            }))
        }));
    }
}

/// `pace log`
#[derive(clap::Args, Command, Debug)]
pub struct LogCmd {
    #[arg(value_name = "NOTE", required = true, help = "What you worked on")]
    note: Vec<String>,

    #[arg(
        long,
        value_name = "TIME",
        allow_hyphen_values = true,
        help = "When it began: HH:MM, -2h, yesterday HH:MM, YYYY-MM-DD HH:MM"
    )]
    from: String,

    #[arg(
        long,
        value_name = "TIME",
        allow_hyphen_values = true,
        required_unless_present = "duration",
        conflicts_with = "duration",
        help = "When it ended"
    )]
    to: Option<String>,

    #[arg(
        long,
        value_name = "DURATION",
        help = "How long it took, for example 45m or 1h30m"
    )]
    duration: Option<String>,

    #[arg(
        long = "tag",
        value_name = "TAG",
        help = "A tag for the entry; repeat for more"
    )]
    tags: Vec<String>,

    #[arg(long, help = "Allow the entry to overlap another one")]
    allow_overlap: bool,
}

impl Runnable for LogCmd {
    fn run(&self) {
        exit_on_error(track(|session, notes| {
            let now = session.now();
            let start = moment(&self.from, &now, notes)?;
            let end = match (&self.to, &self.duration) {
                (Some(to), _) => moment(to, &now, notes)?,
                (None, Some(duration)) => {
                    let duration = parse_duration(duration)
                        .map_err(|error| error.attach_exit_code(ExitCode::Usage))?;
                    start.checked_add(duration).map_err(|error| {
                        PaceError::with_source(ErrorKind::Time, "the end lies too far out", error)
                            .attach_exit_code(ExitCode::Usage)
                    })?
                }
                (None, None) => {
                    return Err(
                        PaceError::new(ErrorKind::InvalidInput, "`pace log` needs an end")
                            .attach_hint("give `--to TIME` or `--duration DURATION`")
                            .attach_exit_code(ExitCode::Usage),
                    );
                }
            };
            let project = PACE_APP.context()?.project.as_ref().map(|project| {
                notes.push(format!("Project: {} ({}).", project.key, project.reason));
                ProjectRef::Key(project.key.clone())
            });
            Ok(UseCase::Log(Log {
                note: text(&self.note),
                project,
                tags: tags(&self.tags)?,
                allow_overlap: self.allow_overlap,
                ..Log::new(start, end)
            }))
        }));
    }
}

/// `pace undo`
#[derive(clap::Args, Command, Debug)]
pub struct UndoCmd {}

impl Runnable for UndoCmd {
    fn run(&self) {
        exit_on_error(track(|_, _| Ok(UseCase::Undo)));
    }
}

/// `pace forget`
#[derive(clap::Args, Command, Debug)]
pub struct ForgetCmd {
    #[arg(
        value_name = "ENTRY",
        conflicts_with = "last",
        help = "The entry, by its short ID; the open one if left out"
    )]
    entry: Option<String>,

    #[arg(long, help = "The entry that ended last")]
    last: bool,

    #[arg(long, help = "Forget it without asking: it is gone for good")]
    for_good: bool,
}

impl Runnable for ForgetCmd {
    fn run(&self) {
        exit_on_error(track(|session, _| {
            let chosen = select::chosen(session, self.entry.as_deref(), self.last, "forget")?;
            if !self.for_good {
                confirm(&output::label(&chosen.state, &chosen.entry, &session.now()))?;
            }
            Ok(UseCase::Forget(Forget {
                entry: chosen.entry.id,
            }))
        }));
    }
}

/// Ask before forgetting; without a terminal, `--for-good` must say so.
fn confirm(label: &str) -> PaceResult<()> {
    if !prompt::interactive() {
        return Err(PaceError::new(
            ErrorKind::InvalidInput,
            "forgetting {entry} can't be undone, so pace asks first",
        )
        .attach_context("entry", label)
        .attach_hint("add `--for-good` to forget it without being asked")
        .attach_exit_code(ExitCode::Usage));
    }
    if prompt::confirm(&format!(
        "Forget {label} for good? `pace undo` can't bring it back."
    ))? {
        return Ok(());
    }
    Err(
        PaceError::new(ErrorKind::InvalidInput, "nothing was forgotten")
            .attach_exit_code(ExitCode::NothingToDo),
    )
}
