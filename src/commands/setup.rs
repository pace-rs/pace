//! `pace setup`: the guided tour of the settings, the roles, the clients and
//! this directory's project, which writes only what changed.
//!
//! After each answer, the command saves where setup stands to `setup.toml`
//! in the machine directory, so an interrupted setup continues where it
//! stopped; it removes the file once setup ends.

use std::path::Path;

use abscissa_core::{Command, Runnable};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_store::field_edit::write_field_edits;

use crate::application::{Context, PACE_APP, exit_on_error};
use crate::clock::RandomIds;
use crate::config::Key;
use crate::output::listed;
use crate::print::print_lines;
use crate::project::{Records, manifest_in};
use crate::prompt::Ask;
use crate::questions::{Ended, Flow, ask_until_it_fits, told};
use crate::setup::setup_file::{
    read_saved_setup, remove_saved_setup, setup_file_in, write_saved_setup,
};
use crate::setup::{Replayed, SavedSetup, SetupFlow, SetupInputs, SetupOutcome};

/// The commands that do setup's parts without a terminal.
const SETUP_PARTS: [&str; 4] = [
    "`pace config set`",
    "`pace role add`",
    "`pace client add`",
    "`pace project init`",
];

/// `pace setup`
#[derive(clap::Args, Command, Debug)]
pub struct SetupCmd {
    /// Start over without asking, even if an interrupted setup was saved
    #[arg(long)]
    start_over: bool,
}

impl Runnable for SetupCmd {
    fn run(&self) {
        exit_on_error(PACE_APP.context().and_then(|context| {
            let inputs = setup_inputs(context)?;
            let saved_file = setup_file_in(&context.dirs.machine.path);
            print_lines(set_up(
                inputs,
                &context.terminal,
                &saved_file,
                self.start_over,
            )?)
        }));
    }
}

/// What setup starts from: the settings as the global file has them, the
/// catalog, the working directory with its manifest, and today.
///
/// # Errors
///
/// If the manifest of the working directory doesn't read, or today is
/// unknown.
fn setup_inputs(context: &Context) -> PaceResult<SetupInputs> {
    let home = &context.dirs.data.path;
    let records = Records {
        catalog: &context.catalog,
        home,
    };
    let text = |key| match context.settings.file_or_default(key) {
        toml::Value::String(text) => text,
        other => other.to_string(),
    };
    Ok(SetupInputs {
        config_file: context.dirs.config_file.path.clone(),
        time_zone: text(Key::TimeZone),
        week_start: text(Key::WeekStart),
        home: home.clone(),
        catalog: context.catalog.clone(),
        dir: context.cwd.clone(),
        manifest: manifest_in(&context.cwd, records)?,
        today: context.today()?,
        ids: Box::new(RandomIds),
    })
}

/// Ask setup's questions through `asker`, write the changes the user
/// accepts, and say what was written.
///
/// An interrupted setup saved in `saved_file` is offered to continue,
/// unless `start_over`. After each answer that leaves a question, the
/// answers so far go to `saved_file`; once setup ends, the file goes.
///
/// # Errors
///
/// Without a terminal: a usage error that names the commands that do the
/// parts. With nothing to change: exit code 3. If the user gives no
/// answer, or a file can't be written.
fn set_up(
    inputs: SetupInputs,
    asker: &dyn Ask,
    saved_file: &Path,
    start_over: bool,
) -> PaceResult<Vec<String>> {
    if !asker.can_ask() {
        return Err(PaceError::new(
            ErrorKind::InvalidInput,
            "pace setup asks its questions on a terminal, and there is none",
        )
        .attach_hint(format!(
            "set each part with {}",
            listed(&SETUP_PARTS.map(str::to_owned))
        ))
        .attach_exit_code(ExitCode::Usage));
    }
    let mut flow = resumed_or_new(inputs, asker, saved_file, start_over)?;
    while let Some(question) = flow.next_question() {
        if let Some(lines) = flow.summary() {
            asker.tell("Setup changes:");
            for line in lines {
                asker.tell(&format!("  {line}"));
            }
        }
        ask_until_it_fits(&mut flow, &question, asker)?;
        // The answer to the summary isn't saved: a write that fails
        // continues at the summary, which shows the changes again.
        if flow.next_question().is_some() {
            write_saved_setup(saved_file, &flow.saved())?;
        }
    }
    let Ended { edits, made } = flow.finish()?;
    if matches!(made, SetupOutcome::Accepted { .. }) {
        write_field_edits(edits)?;
    }
    remove_saved_setup(saved_file)?;
    outcome_said(made)
}

/// The flow setup asks through: the interrupted one in `saved_file` if
/// the user continues it, else a new one. With `start_over`, or if the
/// user starts over, the saved answers go. A saved setup that doesn't
/// read is told as a warning, and setup starts over.
///
/// # Errors
///
/// If the user gives no answer, or the saved setup can't be removed.
fn resumed_or_new(
    inputs: SetupInputs,
    asker: &dyn Ask,
    saved_file: &Path,
    start_over: bool,
) -> PaceResult<SetupFlow> {
    if start_over {
        remove_saved_setup(saved_file)?;
        return Ok(SetupFlow::new(inputs));
    }
    let saved = match read_saved_setup(saved_file) {
        Ok(Some(saved)) => saved,
        Ok(None) => return Ok(SetupFlow::new(inputs)),
        Err(error) => {
            asker.tell(&format!("warning: {}; setup starts over", told(&error)));
            return Ok(SetupFlow::new(inputs));
        }
    };
    if !continues(&saved, asker)? {
        remove_saved_setup(saved_file)?;
        return Ok(SetupFlow::new(inputs));
    }
    let Replayed { flow, changed } = SetupFlow::replayed(inputs, &saved);
    if let Some(changed) = changed {
        asker.tell(&changed);
    }
    Ok(flow)
}

/// Whether the user continues `saved`, asked through `asker`.
///
/// # Errors
///
/// If the user gives no answer.
fn continues(saved: &SavedSetup, asker: &dyn Ask) -> PaceResult<bool> {
    let step = saved.step.label();
    let picked = asker.choose(
        &format!("An earlier setup stopped at {step}. Continue there?"),
        &[format!("continue at {step}"), "start over".to_owned()],
        0,
    )?;
    Ok(picked == 0)
}

/// What setup says it did with `made`, once the files are written.
///
/// # Errors
///
/// With nothing to change: exit code 3.
fn outcome_said(made: SetupOutcome) -> PaceResult<Vec<String>> {
    match made {
        SetupOutcome::NothingToChange => Err(PaceError::new(
            ErrorKind::InvalidInput,
            "nothing to change: each answer keeps the value it had",
        )
        .attach_exit_code(ExitCode::NothingToDo)),
        SetupOutcome::Declined { .. } => Ok(vec![
            "Wrote nothing; the files stay as they were.".to_owned(),
        ]),
        SetupOutcome::Accepted { changes, files } => {
            let count = match changes.len() {
                1 => "1 change".to_owned(),
                count => format!("{count} changes"),
            };
            Ok(vec![format!("Wrote {count} into {}.", listed(&files))])
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use jiff::civil::date;
    use pace_core::Catalog;
    use pace_store::testing::scratch_dir;

    use super::*;
    use crate::prompt::Scripted;
    use crate::questions::Answer;
    use crate::setup::SetupStep;
    use crate::setup::setup_file::read_saved_setup;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// The inputs of a run in `dir`: the config file there sets the week
    /// start to sunday; no roles, no clients, no manifest.
    fn inputs(dir: &Path) -> SetupInputs {
        SetupInputs {
            config_file: dir.join("pace.global.toml"),
            time_zone: "system".to_owned(),
            week_start: "sunday".to_owned(),
            home: dir.join("home"),
            catalog: Catalog::default(),
            dir: dir.join("work"),
            manifest: None,
            today: date(2026, 11, 27),
            ids: Box::new(RandomIds),
        }
    }

    const CONFIG: &str = "# my week\nweek-start = \"sunday\"\n";

    /// The file of an interrupted setup in the machine directory of `dir`.
    fn saved_file(dir: &Path) -> PathBuf {
        setup_file_in(&dir.join("machine"))
    }

    /// A run in `dir` that the user stops after the time zone
    /// `Europe/Vienna` and the week start, as Ctrl-C would.
    fn interrupted(dir: &Path) -> Box<PaceError> {
        let asker = Scripted::typing(["Europe/Vienna"]).and_choosing([6]);
        set_up(inputs(dir), &asker, &saved_file(dir), false).unwrap_err()
    }

    /// The offer to continue an interrupted setup.
    const OFFER: &str = "An earlier setup stopped at the roles. Continue there?";

    /// A user who types `time_zone`, keeps the week start, adds no role
    /// or client, leaves the directory alone and answers `accept`.
    fn user(time_zone: &str, accept: bool) -> Scripted {
        Scripted::typing([time_zone])
            .and_choosing([6, 0])
            .and_confirming([false, false, accept])
    }

    /// The `user` of a run that first picks `pick` of the offer to
    /// continue: 0 continues, 1 starts over.
    fn user_after_offer(pick: usize, time_zone: &str, accept: bool) -> Scripted {
        Scripted::typing([time_zone])
            .and_choosing([pick, 6, 0])
            .and_confirming([false, false, accept])
    }

    #[test]
    fn setup_without_terminal_is_usage_error_naming_four_commands() -> TestResult {
        let dir = scratch_dir()?;

        let error = set_up(
            inputs(dir.path()),
            &Scripted::without_terminal(),
            &saved_file(dir.path()),
            false,
        )
        .unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(
            error.hint(),
            Some(
                "set each part with `pace config set`, `pace role add`, `pace client add` and \
                 `pace project init`"
            )
        );
        Ok(())
    }

    #[test]
    fn accepted_time_zone_is_written_beside_comment_and_other_field() -> TestResult {
        let dir = scratch_dir()?;
        let config = dir.path().join("pace.global.toml");
        std::fs::write(&config, CONFIG)?;

        let asker = user("Europe/Vienna", true);

        let lines = set_up(inputs(dir.path()), &asker, &saved_file(dir.path()), false)?;

        assert_eq!(lines, ["Wrote 1 change into pace.global.toml."]);
        assert_eq!(
            asker.told(),
            [
                "Setup changes:",
                "  pace.global.toml: time-zone system → Europe/Vienna"
            ]
        );
        assert_eq!(
            std::fs::read_to_string(&config)?,
            format!("{CONFIG}time-zone = \"Europe/Vienna\"\n")
        );
        assert!(!saved_file(dir.path()).exists());
        Ok(())
    }

    #[test]
    fn declined_summary_leaves_file_as_it_was() -> TestResult {
        let dir = scratch_dir()?;
        let config = dir.path().join("pace.global.toml");
        std::fs::write(&config, CONFIG)?;

        let lines = set_up(
            inputs(dir.path()),
            &user("Europe/Vienna", false),
            &saved_file(dir.path()),
            false,
        )?;

        assert_eq!(lines, ["Wrote nothing; the files stay as they were."]);
        assert_eq!(std::fs::read_to_string(&config)?, CONFIG);
        assert!(!saved_file(dir.path()).exists());
        Ok(())
    }

    #[test]
    fn interrupted_setup_keeps_its_answers_and_changes_no_config_file() -> TestResult {
        let dir = scratch_dir()?;
        let config = dir.path().join("pace.global.toml");
        std::fs::write(&config, CONFIG)?;

        _ = interrupted(dir.path());

        let saved = read_saved_setup(&saved_file(dir.path()))?.unwrap();
        assert_eq!(saved.step, SetupStep::Roles);
        assert_eq!(
            saved
                .answers
                .iter()
                .map(|answer| answer.answer.clone())
                .collect::<Vec<_>>(),
            [Answer::Text("Europe/Vienna".to_owned()), Answer::Choice(6)]
        );
        assert_eq!(std::fs::read_to_string(&config)?, CONFIG);
        Ok(())
    }

    #[test]
    fn next_setup_continues_at_step_where_it_stopped() -> TestResult {
        let dir = scratch_dir()?;
        _ = interrupted(dir.path());
        let asker = Scripted::choosing([0, 0]).and_confirming([false, false, true]);

        let lines = set_up(inputs(dir.path()), &asker, &saved_file(dir.path()), false)?;

        assert_eq!(
            asker.questions()[..2],
            [OFFER, "You have no roles yet. Add one?"]
        );
        assert_eq!(asker.offered()[0], ["continue at the roles", "start over"]);
        assert_eq!(lines, ["Wrote 1 change into pace.global.toml."]);
        assert!(!saved_file(dir.path()).exists());
        Ok(())
    }

    #[test]
    fn answer_to_start_over_asks_from_first_question() -> TestResult {
        let dir = scratch_dir()?;
        _ = interrupted(dir.path());
        let asker = user_after_offer(1, "UTC", true);

        _ = set_up(inputs(dir.path()), &asker, &saved_file(dir.path()), false)?;

        assert_eq!(
            asker.questions()[..2],
            [
                OFFER,
                "Which time zone do you live in, such as Europe/Berlin, or system?"
            ]
        );
        Ok(())
    }

    #[test]
    fn start_over_flag_asks_from_first_question_without_offer() -> TestResult {
        let dir = scratch_dir()?;
        _ = interrupted(dir.path());
        let asker = user("UTC", true);

        let lines = set_up(inputs(dir.path()), &asker, &saved_file(dir.path()), true)?;

        assert_eq!(
            asker.questions()[0],
            "Which time zone do you live in, such as Europe/Berlin, or system?"
        );
        assert_eq!(lines, ["Wrote 1 change into pace.global.toml."]);
        assert!(!saved_file(dir.path()).exists());
        Ok(())
    }

    #[test]
    fn value_changed_since_interruption_is_asked_again_with_change() -> TestResult {
        let dir = scratch_dir()?;
        _ = interrupted(dir.path());
        let edited = SetupInputs {
            time_zone: "UTC".to_owned(),
            ..inputs(dir.path())
        };
        let asker = user_after_offer(0, "Europe/Vienna", true);

        _ = set_up(edited, &asker, &saved_file(dir.path()), false)?;

        assert_eq!(
            asker.told()[0],
            "Since setup stopped, the value changed: `system` → `UTC`."
        );
        assert_eq!(
            asker.questions()[1],
            "Which time zone do you live in, such as Europe/Berlin, or system?"
        );
        Ok(())
    }

    #[test]
    fn saved_setup_of_newer_pace_warns_and_starts_over() -> TestResult {
        let dir = scratch_dir()?;
        let path = saved_file(dir.path());
        std::fs::create_dir_all(dir.path().join("machine"))?;
        std::fs::write(&path, "version = 2\n")?;
        let asker = user("UTC", true);

        _ = set_up(inputs(dir.path()), &asker, &path, false)?;

        assert_eq!(
            asker.told()[0],
            "warning: this pace reads version 1 of the saved setup, not 2: a newer pace wrote \
             this file; update pace; setup starts over"
        );
        assert_eq!(
            asker.questions()[0],
            "Which time zone do you live in, such as Europe/Berlin, or system?"
        );
        Ok(())
    }

    #[test]
    fn enter_on_every_question_has_nothing_to_change() -> TestResult {
        let dir = scratch_dir()?;
        let asker = Scripted::typing([""])
            .and_choosing([6, 0])
            .and_confirming([false, false]);

        let error = set_up(inputs(dir.path()), &asker, &saved_file(dir.path()), false).unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::NothingToDo);
        assert_eq!(
            error.to_string(),
            "nothing to change: each answer keeps the value it had"
        );
        assert!(!saved_file(dir.path()).exists());
        assert_eq!(
            asker.questions()[..2],
            [
                "Which time zone do you live in, such as Europe/Berlin, or system?",
                "On which day does your week start?"
            ]
        );
        Ok(())
    }
}
