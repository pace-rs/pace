//! The file `setup.toml` in the machine directory, where `pace setup`
//! keeps its answers until it writes them, so an interrupted setup can
//! continue.
//!
//! It lies in the machine directory, which is never synced: the answers
//! belong to one run on this device. [`write_saved_setup`] writes it
//! through a temporary file and a rename, so an interruption leaves the
//! last whole state. [`read_saved_setup`] checks its `version`: a file
//! of a newer pace, or one that doesn't read, is an error that names the
//! file, which setup shows as a warning before it starts over.

use std::io::ErrorKind as IoErrorKind;
use std::path::{Path, PathBuf};

use pace_error::{ErrorKind, PaceError, PaceResult};
use pace_store::toml_file::read_hint;
use pace_store::write_atomically;

use crate::setup::{SAVED_SETUP_VERSION, SavedSetup};

/// The name of the file of an interrupted setup.
pub const SETUP_FILE: &str = "setup.toml";

/// The file of an interrupted setup in the machine directory `machine`.
#[must_use]
pub fn setup_file_in(machine: &Path) -> PathBuf {
    machine.join(SETUP_FILE)
}

/// The saved setup in `path`; `None` if there is none.
///
/// Setup shows an error of this function as a warning and starts over,
/// so each names the file and says why it can't continue, without a hint
/// to repair a file that setup overwrites at its first answer.
///
/// # Errors
///
/// If the file can't be read, is no valid TOML, was written by a newer
/// pace, or holds no saved setup of [`SAVED_SETUP_VERSION`].
pub fn read_saved_setup(path: &Path) -> PaceResult<Option<SavedSetup>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == IoErrorKind::NotFound => return Ok(None),
        Err(error) => {
            let hint = read_hint(&error);
            return Err(
                file_error("cannot read the saved setup {path}", path, error).attach_hint(hint),
            );
        }
    };
    let table: toml::Table = text
        .parse()
        .map_err(|error| file_error("the saved setup {path} is not valid TOML", path, error))?;
    let version = table.get("version").and_then(toml::Value::as_integer);
    if version.is_some_and(|version| version > i64::from(SAVED_SETUP_VERSION)) {
        return Err(PaceError::new(
            ErrorKind::InputOutput,
            "a newer pace wrote the saved setup {path}",
        )
        .attach_context("path", path.display().to_string())
        .attach_hint("update pace"));
    }
    toml::Value::Table(table)
        .try_into::<SavedSetup>()
        .ok()
        .filter(|saved| saved.version == SAVED_SETUP_VERSION)
        .map(Some)
        .ok_or_else(|| {
            PaceError::new(
                ErrorKind::InputOutput,
                "the saved setup {path} holds no setup that pace can continue",
            )
            .attach_context("path", path.display().to_string())
        })
}

/// Write `saved` to `path` through a temporary file and a rename.
///
/// # Errors
///
/// If the file can't be written.
pub fn write_saved_setup(path: &Path, saved: &SavedSetup) -> PaceResult<()> {
    let text = toml::to_string(saved).map_err(|error| {
        PaceError::with_source(ErrorKind::Internal, "cannot write {path}", error)
            .attach_context("path", path.display().to_string())
            .ask_report()
    })?;
    write_atomically(path, &text)
}

/// Remove the saved setup in `path`, if there is one.
///
/// # Errors
///
/// If the file exists and can't be removed.
pub fn remove_saved_setup(path: &Path) -> PaceResult<()> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != IoErrorKind::NotFound => {
            Err(file_error("cannot remove {path}", path, error)
                .attach_hint("check that pace may write to the machine directory"))
        }
        _ => Ok(()),
    }
}

/// The error `guidance` about the file `path`, caused by `error`.
fn file_error(
    guidance: &'static str,
    path: &Path,
    error: impl Into<Box<dyn std::error::Error + Send + Sync>>,
) -> Box<PaceError> {
    PaceError::with_source(ErrorKind::InputOutput, guidance, error)
        .attach_context("path", path.display().to_string())
}

#[cfg(test)]
mod tests {
    use pace_store::testing::scratch_dir;

    use super::*;
    use crate::questions::Answer;
    use crate::setup::{SavedAnswer, SetupStep};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn saved() -> SavedSetup {
        SavedSetup {
            version: SAVED_SETUP_VERSION,
            step: SetupStep::Settings,
            answers: vec![SavedAnswer {
                question: "time-zone".to_owned(),
                answer: Answer::Text("UTC".to_owned()),
                replaced: Some("system".to_owned()),
                picked: None,
            }],
        }
    }

    #[test]
    fn written_setup_reads_back_from_machine_directory() -> TestResult {
        let dir = scratch_dir()?;
        let path = setup_file_in(&dir.path().join("machine"));

        write_saved_setup(&path, &saved())?;

        assert_eq!(read_saved_setup(&path)?, Some(saved()));
        assert!(std::fs::read_to_string(&path)?.starts_with("version = 1\n"));
        Ok(())
    }

    #[test]
    fn missing_file_is_no_saved_setup() -> TestResult {
        let dir = scratch_dir()?;

        assert_eq!(read_saved_setup(&dir.path().join(SETUP_FILE))?, None);
        Ok(())
    }

    #[test]
    fn file_of_newer_pace_names_itself_and_asks_for_update() -> TestResult {
        let dir = scratch_dir()?;
        let path = dir.path().join(SETUP_FILE);
        std::fs::write(&path, "version = 2\nstep = \"later\"\n")?;

        let error = read_saved_setup(&path).unwrap_err();

        assert_eq!(
            error.to_string(),
            format!("a newer pace wrote the saved setup {}", path.display())
        );
        assert_eq!(error.hint(), Some("update pace"));
        Ok(())
    }

    #[test]
    fn file_that_is_not_toml_names_itself_without_hint() -> TestResult {
        let dir = scratch_dir()?;
        let path = dir.path().join(SETUP_FILE);
        std::fs::write(&path, "version = 1\nstep = \n")?;

        let error = read_saved_setup(&path).unwrap_err();

        assert_eq!(
            error.to_string(),
            format!("the saved setup {} is not valid TOML", path.display())
        );
        assert_eq!(error.hint(), None);
        Ok(())
    }

    #[test]
    fn broken_file_names_itself() -> TestResult {
        let dir = scratch_dir()?;
        let path = dir.path().join(SETUP_FILE);
        std::fs::write(&path, "version = 1\nstep = \"roles\"\n")?;

        let error = read_saved_setup(&path).unwrap_err();

        assert_eq!(
            error.to_string(),
            format!(
                "the saved setup {} holds no setup that pace can continue",
                path.display()
            )
        );
        Ok(())
    }

    #[test]
    fn file_of_older_version_holds_no_setup_to_continue() -> TestResult {
        let dir = scratch_dir()?;
        let path = dir.path().join(SETUP_FILE);
        let mut older = saved();
        older.version = 0;
        write_saved_setup(&path, &older)?;

        let error = read_saved_setup(&path).unwrap_err();

        assert!(
            error
                .to_string()
                .ends_with("holds no setup that pace can continue")
        );
        Ok(())
    }

    #[test]
    fn removed_file_is_gone_and_removing_none_is_fine() -> TestResult {
        let dir = scratch_dir()?;
        let path = dir.path().join(SETUP_FILE);
        write_saved_setup(&path, &saved())?;

        remove_saved_setup(&path)?;
        remove_saved_setup(&path)?;

        assert!(!path.exists());
        Ok(())
    }
}
