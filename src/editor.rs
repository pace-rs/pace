//! Editing text in the user's editor: `VISUAL`, else `EDITOR`.

use std::{ffi::OsString, io::Write as _, process::Command};

use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};

/// The editor the user set: `VISUAL`, else `EDITOR`; a variable that is
/// empty or no UTF-8 counts as unset. The edge reads it once.
#[must_use]
pub fn editor_variable(env: impl Fn(&str) -> Option<OsString>) -> Option<String> {
    ["VISUAL", "EDITOR"]
        .into_iter()
        .filter_map(|name| env(name)?.into_string().ok())
        .find(|value| !value.trim().is_empty())
}

/// The editor to start: the one the user set, else `vi` (`notepad` on
/// Windows) on a terminal. Without either, pace would start an editor that
/// waits for input nobody gives, so it stops and names `pace adjust`.
///
/// # Errors
///
/// If no editor is set and no terminal is attached.
pub fn editor_command(variable: Option<&str>, terminal: bool) -> PaceResult<String> {
    match variable {
        Some(editor) => Ok(editor.to_owned()),
        None if terminal => Ok(if cfg!(windows) { "notepad" } else { "vi" }.to_owned()),
        None => Err(PaceError::new(
            ErrorKind::InvalidInput,
            "`pace edit` has no terminal and no editor in VISUAL or EDITOR",
        )
        .attach_hint("change the entry with `pace adjust`, or set VISUAL or EDITOR")
        .attach_exit_code(ExitCode::Usage)),
    }
}

/// Let the user edit `text` in a file ending in `suffix`; the text once the
/// editor is closed.
///
/// # Errors
///
/// If the editor can't be started or fails, or the file can't be written
/// or read.
pub fn edit_in_editor(editor: &str, text: &str, suffix: &str) -> PaceResult<String> {
    let words = shell_words::split(editor).map_err(|error| {
        PaceError::with_source(
            ErrorKind::InvalidInput,
            "pace can't read the editor command `{editor}`",
            error,
        )
        .attach_context("editor", editor)
        .attach_hint("check the quotes in VISUAL or EDITOR")
    })?;
    let Some((program, arguments)) = words.split_first() else {
        return Err(
            PaceError::new(ErrorKind::InvalidInput, "the editor command is empty")
                .attach_hint("set VISUAL or EDITOR to the editor you use, for example `nano`"),
        );
    };

    let file_error = |error: std::io::Error| {
        PaceError::with_source(
            ErrorKind::InputOutput,
            "pace can't write the file for the editor",
            error,
        )
        .attach_hint("check that the temporary directory is writable")
    };
    let mut file = tempfile::Builder::new()
        .prefix("pace-")
        .suffix(suffix)
        .tempfile()
        .map_err(file_error)?;
    file.write_all(text.as_bytes()).map_err(file_error)?;
    // Closed, so editors that replace the file work on every OS; the path
    // removes the file when it is dropped.
    let path = file.into_temp_path();

    let status = Command::new(program)
        .args(arguments)
        .arg(&path)
        .status()
        .map_err(|error| {
            PaceError::with_source(
                ErrorKind::InputOutput,
                "pace can't start the editor `{editor}`",
                error,
            )
            .attach_context("editor", editor)
            .attach_hint("set VISUAL or EDITOR to the editor you use, for example `nano`")
        })?;
    if !status.success() {
        return Err(PaceError::new(
            ErrorKind::InputOutput,
            "the editor `{editor}` ended with {status}",
        )
        .attach_context("editor", editor)
        .attach_context("status", status.to_string())
        .attach_hint("nothing was changed"));
    }
    std::fs::read_to_string(&path).map_err(|error| {
        PaceError::with_source(
            ErrorKind::InputOutput,
            "pace can't read the file back from the editor",
            error,
        )
        .attach_hint("nothing was changed")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(vars: &'a [(&str, &str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
        move |name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        }
    }

    #[test]
    fn visual_wins_over_editor() {
        let vars = [("VISUAL", "code --wait"), ("EDITOR", "nano")];

        assert_eq!(editor_variable(env(&vars)), Some("code --wait".to_owned()));
    }

    #[test]
    fn empty_visual_leaves_editor() {
        let vars = [("VISUAL", " "), ("EDITOR", "nano")];

        assert_eq!(editor_variable(env(&vars)), Some("nano".to_owned()));
        assert_eq!(editor_variable(env(&[])), None);
    }

    #[test]
    fn set_editor_runs_without_terminal() {
        assert_eq!(editor_command(Some("nano"), false).unwrap(), "nano");
    }

    #[cfg(unix)]
    #[test]
    fn terminal_without_editor_starts_vi() {
        assert_eq!(editor_command(None, true).unwrap(), "vi");
    }

    #[test]
    fn no_terminal_and_no_editor_names_adjust() {
        let error = editor_command(None, false).unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(
            error.hint(),
            Some("change the entry with `pace adjust`, or set VISUAL or EDITOR")
        );
    }
}
