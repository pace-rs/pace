//! The tracking commands: `begin`, `hold`, `pause`, `resume`, `catch`, `end`.

use std::collections::BTreeSet;

use abscissa_core::{Command, Runnable};
use jiff::ZonedDateTime;
use pace_core::{
    Command as UseCase, HoldSource, PauseKind, ProjectRef, Tag,
    command::{Begin, Catch, End, Hold, Pause, Resume},
};
use pace_error::{ExitCode, PaceError, PaceResult};
use pace_time::{At, parse_at};

use crate::{
    application::{PACE_APP, exit_on_error},
    output,
    session::Session,
};

/// Run `build` with the session, print what happened, and the first-use
/// note.
fn track(build: impl FnOnce(&Session, &mut Vec<String>) -> PaceResult<UseCase>) -> PaceResult<()> {
    let context = PACE_APP.context()?;
    let session = Session::open(context)?;
    let mut notes = Vec::new();
    let command = build(&session, &mut notes)?;
    let outcome = session.run(&command)?;
    for line in output::describe(&outcome) {
        println!("{line}");
    }
    if let (UseCase::Begin(begin), Some(project)) = (&command, &context.project)
        && begin.project.is_some()
    {
        println!("Project: {} ({}).", project.key, project.reason);
    }
    for note in notes.iter().chain(session.first_use_note().iter()) {
        println!("{note}");
    }
    Ok(())
}

/// A `--at` value as a moment; a time that happened twice takes the earlier
/// one and says so.
fn at(
    value: Option<&str>,
    now: &ZonedDateTime,
    notes: &mut Vec<String>,
) -> PaceResult<Option<ZonedDateTime>> {
    let Some(value) = value else {
        return Ok(None);
    };
    match parse_at(value, now).map_err(|error| error.attach_exit_code(ExitCode::Usage))? {
        At::Exact(moment) => Ok(Some(moment)),
        At::Ambiguous { earlier, later } => {
            notes.push(format!(
                "{value} happened twice because the clocks were set back; pace took the earlier one ({}, not {}).",
                earlier.strftime("%H:%M %:z"),
                later.strftime("%H:%M %:z")
            ));
            Ok(Some(earlier))
        }
    }
}

fn text(words: &[String]) -> Option<String> {
    let text = words.join(" ");
    (!text.trim().is_empty()).then_some(text)
}

fn tags(values: &[String]) -> PaceResult<BTreeSet<Tag>> {
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
}

impl Runnable for BeginCmd {
    fn run(&self) {
        exit_on_error(track(|session, notes| {
            let project = PACE_APP
                .context()?
                .project
                .as_ref()
                .map(|project| ProjectRef::Key(project.key.clone()));
            Ok(UseCase::Begin(Begin {
                at: at(self.at.as_deref(), &session.now(), notes)?,
                note: text(&self.note),
                project,
                tags: tags(&self.tags)?,
                allow_overlap: self.allow_overlap,
            }))
        }));
    }
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
                entry: None,
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
                entry: None,
            }))
        }));
    }
}
