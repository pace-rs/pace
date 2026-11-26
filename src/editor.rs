//! Editing text in the user's editor: `VISUAL`, else `EDITOR`.

use std::{io::Write as _, process::Command};

use pace_error::{ErrorKind, PaceError, PaceResult};

/// The editor the user set: `VISUAL`, else `EDITOR`, else `vi` (`notepad`
/// on Windows).
fn editor() -> String {
    ["VISUAL", "EDITOR"]
        .into_iter()
        .filter_map(|name| std::env::var(name).ok())
        .find(|value| !value.trim().is_empty())
        .unwrap_or_else(|| if cfg!(windows) { "notepad" } else { "vi" }.to_owned())
}

/// Let the user edit `text` in a file ending in `suffix`; the text once the
/// editor is closed.
///
/// # Errors
///
/// If the editor can't be started or fails, or the file can't be written
/// or read.
pub fn edit(text: &str, suffix: &str) -> PaceResult<String> {
    let editor = editor();
    let words = shell_words::split(&editor).map_err(|error| {
        PaceError::with_source(
            ErrorKind::InvalidInput,
            "pace can't read the editor command `{editor}`",
            error,
        )
        .attach_context("editor", editor.clone())
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
            .attach_context("editor", editor.clone())
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
