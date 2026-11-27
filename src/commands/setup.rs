//! `pace setup`: the guided tour of the settings, the roles, the clients and
//! this directory's project, which writes only what changed.

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
use crate::questions::{Ended, Flow, ask_until_it_fits};
use crate::setup::{SetupFlow, SetupInputs, SetupOutcome};

/// The commands that do setup's parts without a terminal.
const SETUP_PARTS: [&str; 4] = [
    "`pace config set`",
    "`pace role add`",
    "`pace client add`",
    "`pace project init`",
];

/// `pace setup`
#[derive(clap::Args, Command, Debug)]
pub struct SetupCmd {}

impl Runnable for SetupCmd {
    fn run(&self) {
        exit_on_error(PACE_APP.context().and_then(|context| {
            let inputs = setup_inputs(context)?;
            print_lines(set_up(inputs, &context.terminal)?)
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
/// # Errors
///
/// Without a terminal: a usage error that names the commands that do the
/// parts. With nothing to change: exit code 3. If the user gives no
/// answer, or a file can't be written.
fn set_up(inputs: SetupInputs, asker: &dyn Ask) -> PaceResult<Vec<String>> {
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
    let mut flow = SetupFlow::new(inputs);
    while let Some(question) = flow.next_question() {
        if let Some(lines) = flow.summary() {
            asker.tell("Setup changes:");
            for line in lines {
                asker.tell(&format!("  {line}"));
            }
        }
        ask_until_it_fits(&mut flow, &question, asker)?;
    }
    let Ended { edits, made } = flow.finish()?;
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
            write_field_edits(edits)?;
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
    use std::path::Path;

    use jiff::civil::date;
    use pace_core::Catalog;
    use pace_store::testing::scratch_dir;

    use super::*;
    use crate::prompt::Scripted;

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

    /// A user who types `time_zone`, keeps the week start, adds no role
    /// or client, leaves the directory alone and answers `accept`.
    fn user(time_zone: &str, accept: bool) -> Scripted {
        Scripted::typing([time_zone])
            .and_choosing([6, 0])
            .and_confirming([false, false, accept])
    }

    #[test]
    fn setup_without_terminal_is_usage_error_naming_four_commands() -> TestResult {
        let dir = scratch_dir()?;

        let error = set_up(inputs(dir.path()), &Scripted::without_terminal()).unwrap_err();

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

        let lines = set_up(inputs(dir.path()), &asker)?;

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
        Ok(())
    }

    #[test]
    fn declined_summary_leaves_file_as_it_was() -> TestResult {
        let dir = scratch_dir()?;
        let config = dir.path().join("pace.global.toml");
        std::fs::write(&config, CONFIG)?;

        let lines = set_up(inputs(dir.path()), &user("Europe/Vienna", false))?;

        assert_eq!(lines, ["Wrote nothing; the files stay as they were."]);
        assert_eq!(std::fs::read_to_string(&config)?, CONFIG);
        Ok(())
    }

    #[test]
    fn enter_on_every_question_has_nothing_to_change() -> TestResult {
        let dir = scratch_dir()?;
        let asker = Scripted::typing([""])
            .and_choosing([6, 0])
            .and_confirming([false, false]);

        let error = set_up(inputs(dir.path()), &asker).unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::NothingToDo);
        assert_eq!(
            error.to_string(),
            "nothing to change: each answer keeps the value it had"
        );
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
