//! Correcting the record: `adjust`, `log`, `undo`, `forget`.

use abscissa_core::{Command, Runnable};
use jiff::ZonedDateTime;
use pace_core::{
    Change, Command as UseCase, EntryVerb, ProjectRef, Text,
    command::{Adjust, Forget, Log},
};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_time::parse_duration;

use crate::commands::entry_fields::{BillableFlags, DecidedFlags, entry_role, role_change};
use crate::commands::tracking::{at, entry_tags, parse_moment, tags, text, track};
use crate::prompt::Ask;
use crate::select::{Target, chosen_entry, target_entry};
use crate::{
    application::exit_on_error,
    project::{Project, Reason},
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

    #[arg(
        long,
        value_name = "KEY",
        help = "A new role; \"\" gives it the project's role again"
    )]
    role: Option<String>,

    #[command(flatten)]
    decided: DecidedFlags,

    #[command(flatten)]
    billable: BillableFlags,

    #[arg(long, help = "Allow the entry to overlap another one")]
    allow_overlap: bool,
}

impl Runnable for AdjustCmd {
    fn run(&self) {
        exit_on_error(track(|session, context, notes| {
            let now = session.now();
            let project = moved_project(context.project.as_ref());
            let note = self.note.clone().map_or(Change::Keep, |note| {
                Text::try_from(note).map_or(Change::Clear, Change::Set)
            });
            Ok(UseCase::Adjust(Adjust {
                entry: target_entry(
                    session,
                    &context.terminal,
                    Target::new(self.entry.as_deref(), self.last),
                )?,
                start: at(self.start.as_deref(), &now, &context.terminal, notes)?,
                end: at(self.end.as_deref(), &now, &context.terminal, notes)?,
                note,
                project,
                tag: tags(&self.tags)?,
                untag: tags(&self.untags)?,
                role: role_change(self.role.as_deref(), &context.catalog, notes)?,
                determined_by: self
                    .decided
                    .determined_by()
                    .map_or(Change::Keep, Change::Set),
                billable: self.billable.billable().map_or(Change::Keep, Change::Set),
                allow_overlap: self.allow_overlap,
            }))
        }));
    }
}

/// Where `adjust` moves the entry: only an explicit `--project` moves it;
/// a discovered project or `PACE_PROJECT` doesn't.
fn moved_project(project: Option<&Project>) -> Change<ProjectRef> {
    match project {
        Some(project) if project.reason == Reason::Flag => Change::Set(project.reference()),
        _ => Change::Keep,
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

    #[arg(
        long,
        value_name = "KEY",
        help = "The role its time went to; the project's if left out"
    )]
    role: Option<String>,

    #[command(flatten)]
    decided: DecidedFlags,

    #[command(flatten)]
    billable: BillableFlags,

    #[arg(long, help = "Allow the entry to overlap another one")]
    allow_overlap: bool,
}

impl Runnable for LogCmd {
    fn run(&self) {
        exit_on_error(track(|session, context, notes| {
            let now = session.now();
            let start = parse_moment(&self.from, &now, &context.terminal, notes)?;
            // clap gives `--duration` or `--to`; like `--at`, a missing time is now
            let end = match self.duration.as_deref() {
                Some(duration) => end_after(&start, duration)?,
                None => at(self.to.as_deref(), &now, &context.terminal, notes)?.unwrap_or(now),
            };
            let project = context.project.as_ref().map(|project| {
                notes.push(format!("Project: {} ({}).", project.key, project.reason));
                project.reference()
            });
            Ok(UseCase::Log(Log {
                note: text(&self.note),
                project,
                tags: entry_tags(&self.tags, context.project.as_ref())?,
                role: entry_role(self.role.as_deref(), &context.catalog, notes)?,
                determined_by: self.decided.determined_by(),
                billable: self.billable.billable(),
                allow_overlap: self.allow_overlap,
                ..Log::new(start, end)
            }))
        }));
    }
}

/// The end `--duration` gives from `start`.
fn end_after(start: &ZonedDateTime, duration: &str) -> PaceResult<ZonedDateTime> {
    let span = parse_duration(duration).map_err(|error| error.attach_exit_code(ExitCode::Usage))?;
    start.checked_add(span).map_err(|_| {
        PaceError::new(
            ErrorKind::Time,
            "`--duration {duration}` from {start} ends after the year 9999",
        )
        .attach_context("duration", duration)
        .attach_context("start", start.strftime("%Y-%m-%d %H:%M").to_string())
        .attach_hint("give a shorter `--duration`, or the end with `--to TIME`")
        .attach_exit_code(ExitCode::Usage)
    })
}

/// `pace undo`
#[derive(clap::Args, Command, Debug)]
pub struct UndoCmd {}

impl Runnable for UndoCmd {
    fn run(&self) {
        exit_on_error(track(|_, _, _| Ok(UseCase::Undo)));
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
        exit_on_error(track(|session, context, _| {
            let asker = &context.terminal;
            let chosen = chosen_entry(
                session,
                asker,
                Target::new(self.entry.as_deref(), self.last),
                EntryVerb::Forget,
            )?;
            if !self.for_good {
                confirm_forget(
                    asker,
                    &chosen.state.entry_label(&chosen.entry, &session.now()),
                )?;
            }
            Ok(UseCase::Forget(Forget {
                entry: chosen.entry.id,
            }))
        }));
    }
}

/// Ask before forgetting; without a terminal, `--for-good` must say so.
fn confirm_forget(asker: &dyn Ask, label: &str) -> PaceResult<()> {
    if !asker.can_ask() {
        return Err(PaceError::new(
            ErrorKind::InvalidInput,
            "forgetting {entry} can't be undone, so pace asks first",
        )
        .attach_context("entry", label)
        .attach_hint("add `--for-good` to forget it without being asked")
        .attach_exit_code(ExitCode::Usage));
    }
    if asker.confirm(
        &format!("Forget {label} for good? `pace undo` can't bring it back."),
        false,
    )? {
        return Ok(());
    }
    Err(
        PaceError::new(ErrorKind::InvalidInput, "nothing was forgotten")
            .attach_hint("`pace forget` asks again; `--for-good` forgets without asking")
            .attach_exit_code(ExitCode::NothingToDo),
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::prompt::Scripted;

    #[test]
    fn adjust_help_names_no_default_for_decided_and_billable() {
        let help = <AdjustCmd as clap::Args>::augment_args(clap::Command::new("adjust"))
            .render_help()
            .to_string();

        for flag in ["--by-me", "--by-others", "--billable", "--not-billable"] {
            let line = help
                .lines()
                .find(|line| line.trim_start().starts_with(flag))
                .unwrap();
            assert!(!line.contains("left out"), "{line}");
        }
    }

    fn project(reason: Reason) -> Project {
        Project {
            id: None,
            key: "offer".parse().unwrap(),
            name: None,
            default_tags: std::collections::BTreeSet::new(),
            reason,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn project_flag_moves_entry() {
        let change = moved_project(Some(&project(Reason::Flag)));

        assert_eq!(
            change,
            Change::Set(ProjectRef::Key("offer".parse().unwrap()))
        );
    }

    #[test]
    fn discovered_project_keeps_entry_where_it_is() {
        for reason in [
            Reason::Variable,
            Reason::Manifest(PathBuf::from("pace.toml")),
        ] {
            assert_eq!(moved_project(Some(&project(reason))), Change::Keep);
        }
        assert_eq!(moved_project(None), Change::Keep);
    }

    #[test]
    fn end_too_far_out_names_duration_and_hint() {
        let start: ZonedDateTime = "2026-11-26T09:00[Europe/Berlin]".parse().unwrap();

        let error = end_after(&start, "99999999h").unwrap_err();

        assert_eq!(
            error.to_string(),
            "`--duration 99999999h` from 2026-11-26 09:00 ends after the year 9999"
        );
        assert_eq!(
            error.hint(),
            Some("give a shorter `--duration`, or the end with `--to TIME`")
        );
        assert!(std::error::Error::source(&*error).is_none());
        assert_eq!(error.exit_code(), ExitCode::Usage);
    }

    #[test]
    fn duration_from_start_gives_end() {
        let start: ZonedDateTime = "2026-11-26T09:00[Europe/Berlin]".parse().unwrap();

        let end = end_after(&start, "1h30m").unwrap();

        assert_eq!(end.to_string(), "2026-11-26T10:30:00+01:00[Europe/Berlin]");
    }

    #[test]
    fn forget_declined_forgets_nothing() {
        let asker = Scripted::confirming([false]);

        let error = confirm_forget(&asker, "\"Write offer\" (09:00–10:00)").unwrap_err();

        assert_eq!(error.to_string(), "nothing was forgotten");
        assert_eq!(error.exit_code(), ExitCode::NothingToDo);
        assert_eq!(
            error.hint(),
            Some("`pace forget` asks again; `--for-good` forgets without asking")
        );
        assert_eq!(
            asker.questions(),
            ["Forget \"Write offer\" (09:00–10:00) for good? `pace undo` can't bring it back."]
        );
    }

    #[test]
    fn forget_confirmed_goes_on() {
        let asker = Scripted::confirming([true]);

        confirm_forget(&asker, "\"Write offer\" (09:00–10:00)").unwrap();

        assert_eq!(asker.questions().len(), 1);
    }

    #[test]
    fn forget_without_terminal_names_for_good() {
        let asker = Scripted::without_terminal();

        let error = confirm_forget(&asker, "\"Write offer\" (09:00–10:00)").unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(
            error.hint(),
            Some("add `--for-good` to forget it without being asked")
        );
        assert_eq!(asker.questions(), Vec::<String>::new());
    }
}
