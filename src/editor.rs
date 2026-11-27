//! Editing text in the user's editor: `VISUAL`, else `EDITOR`; the round
//! trip that opens a file again until its check takes it; and what pace
//! says once it saved a catalog file.

use std::{ffi::OsString, io::Write as _, path::Path, process::Command};

use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult, Severity};

use crate::application::print_error;
use crate::commands::mending_command;
use crate::print::print_lines;
use crate::prompt::Ask;

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

/// What [`edit_until_checked`] edits: the file, and the command that
/// opens it again.
#[derive(Debug, Clone, Copy)]
pub struct EditedFile<'file> {
    /// The `PACE_HOME` of the catalog the file belongs to.
    pub home: &'file Path,
    /// The file the text is written to once it is checked.
    pub path: &'file Path,
    /// The command that edits it: `pace role edit`.
    pub command: &'file str,
}

/// Let the user change `text` of `file` with `edit` until `check` takes it;
/// the text and what `check` made of it.
///
/// A mistake, an error of the kind `InvalidInput`, is shown with `report`,
/// and the user may open the edited text again to fix it. Other errors,
/// such as a failing editor, end the round trip.
///
/// # Errors
///
/// If the text comes back unchanged, or the user leaves a mistake, the
/// error exits with 3 and the file stays as it was. Without a terminal, a
/// mistake is the error.
pub fn edit_until_checked<T>(
    file: EditedFile<'_>,
    text: &str,
    asker: &dyn Ask,
    mut edit: impl FnMut(&str) -> PaceResult<String>,
    mut check: impl FnMut(&str) -> PaceResult<T>,
    report: impl Fn(&PaceError),
) -> PaceResult<(String, T)> {
    let path = file.path.display().to_string();
    let mut edited = edit(text)?;
    loop {
        if edited == text {
            return Err(
                PaceError::new(ErrorKind::InvalidInput, "nothing changed in {path}")
                    .attach_context("path", path)
                    .attach_exit_code(ExitCode::NothingToDo),
            );
        }
        let error = match check(&edited) {
            Ok(checked) => return Ok((edited, checked)),
            Err(error) if error.kind() == ErrorKind::InvalidInput => mistake_elsewhere(file, error),
            Err(error) => return Err(error),
        };
        if !asker.can_ask() {
            return Err(error);
        }
        report(&error);
        if !asker.confirm(&format!("Open {path} again to fix it?"), true)? {
            return Err(
                PaceError::new(ErrorKind::InvalidInput, "{path} stays as it was")
                    .attach_context("path", path)
                    .attach_hint(format!("`{}` opens it again", file.command))
                    .attach_exit_code(ExitCode::NothingToDo),
            );
        }
        edited = edit(&edited)?;
    }
}

/// `mistake` of the check of `file`: when it points at another catalog
/// file, a line before it names the command that opens that file, as the
/// edit doesn't save until either file is fixed.
fn mistake_elsewhere(file: EditedFile<'_>, mistake: Box<PaceError>) -> Box<PaceError> {
    let command = mistake
        .snippet_name()
        .filter(|name| Path::new(name) != file.path)
        .and_then(|name| mending_command(file.home, Path::new(name)));
    match command {
        Some(command) => mistake.prepend_guidance_line(format!(
            "pace doesn't save the edit until this mistake is fixed: `{command}` opens it"
        )),
        None => mistake,
    }
}

/// What `pace role edit`, `client edit` and `project edit` say once they
/// saved a catalog file: the sentence that says so, and the mistake of
/// another file that still keeps the catalog from reading.
#[derive(Debug)]
pub struct SavedFile {
    /// The sentence that says what pace saved.
    pub line: String,
    /// The mistake of another file, as a warning that names the command
    /// which edits that file.
    pub left: Option<Box<PaceError>>,
}

impl SavedFile {
    /// The sentence `line`, and the mistake `left` of another catalog file
    /// of `home`, if any.
    #[must_use]
    pub fn new(home: &Path, line: String, left: Option<Box<PaceError>>) -> Self {
        Self {
            line,
            left: left.map(|mistake| left_mistake(home, mistake)),
        }
    }

    /// Print the sentence, then the mistake left, if any.
    ///
    /// # Errors
    ///
    /// If the standard output takes no more.
    pub fn print(self) -> PaceResult<()> {
        print_lines([self.line])?;
        if let Some(mistake) = &self.left {
            print_error(mistake);
        }
        Ok(())
    }
}

/// `mistake` of a catalog file of `home` that is left after pace saved
/// another one: a warning that names the command which edits its file.
fn left_mistake(home: &Path, mistake: Box<PaceError>) -> Box<PaceError> {
    let command = mistake
        .snippet_name()
        .and_then(|file| mending_command(home, Path::new(file)));
    let line = command.map_or_else(
        || "the catalog doesn't read until this mistake is fixed".to_owned(),
        |command| {
            format!("the catalog doesn't read until this mistake is fixed: `{command}` opens it")
        },
    );
    mistake
        .prepend_guidance_line(line)
        .attach_severity(Severity::Warning)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use pace_error::Snippet;

    use super::*;
    use crate::prompt::Scripted;

    /// The roles file, as `pace role edit` edits it.
    fn roles() -> EditedFile<'static> {
        EditedFile {
            home: Path::new("home"),
            path: Path::new("roles.toml"),
            command: "pace role edit",
        }
    }

    /// A check that takes text without `oops`, its length as what it made.
    fn no_oops(text: &str) -> PaceResult<usize> {
        if text.contains("oops") {
            Err(PaceError::new(
                ErrorKind::InvalidInput,
                "the text says oops",
            ))
        } else {
            Ok(text.len())
        }
    }

    /// An editor that saves `saved` in turn, whatever it was given.
    fn editor(saved: &[&str]) -> impl FnMut(&str) -> PaceResult<String> {
        let mut saved: Vec<String> = saved.iter().rev().map(|&text| text.to_owned()).collect();
        move |_| {
            saved.pop().ok_or_else(|| {
                PaceError::new(ErrorKind::Internal, "the test has no more saved texts")
            })
        }
    }

    #[test]
    fn checked_text_comes_back_with_what_check_made() {
        let edit = editor(&["version = 1\n[work]\n"]);

        let (text, length) = edit_until_checked(
            roles(),
            "version = 1\n",
            &Scripted::without_terminal(),
            edit,
            no_oops,
            |_| {},
        )
        .unwrap();

        assert_eq!(text, "version = 1\n[work]\n");
        assert_eq!(length, 19);
    }

    #[test]
    fn mistake_fixed_after_opening_again_is_checked() {
        let edit = editor(&["oops", "fixed"]);
        let asker = Scripted::confirming([true]);
        let reported = RefCell::new(Vec::new());

        let (text, _) = edit_until_checked(roles(), "before", &asker, edit, no_oops, |error| {
            reported.borrow_mut().push(error.to_string());
        })
        .unwrap();

        assert_eq!(text, "fixed");
        assert_eq!(reported.into_inner(), ["the text says oops"]);
        assert_eq!(asker.questions(), ["Open roles.toml again to fix it?"]);
    }

    #[test]
    fn mistake_left_keeps_file_and_names_command() {
        let edit = editor(&["oops"]);
        let asker = Scripted::confirming([false]);

        let error =
            edit_until_checked(roles(), "before", &asker, edit, no_oops, |_| {}).unwrap_err();

        assert_eq!(error.to_string(), "roles.toml stays as it was");
        assert_eq!(error.hint(), Some("`pace role edit` opens it again"));
        assert_eq!(error.exit_code(), ExitCode::NothingToDo);
    }

    #[test]
    fn mistake_without_terminal_is_error() {
        let edit = editor(&["oops"]);

        let error = edit_until_checked(
            roles(),
            "before",
            &Scripted::without_terminal(),
            edit,
            no_oops,
            |_| {},
        )
        .unwrap_err();

        assert_eq!(error.to_string(), "the text says oops");
    }

    #[test]
    fn mistake_in_other_file_names_command_that_opens_it() {
        let edit = editor(&["after"]);
        let record = Path::new("home/projects/webshop.toml");

        let error = edit_until_checked(
            roles(),
            "before",
            &Scripted::without_terminal(),
            edit,
            |_| -> PaceResult<()> {
                Err(
                    PaceError::new(ErrorKind::InvalidInput, "the client is unknown")
                        .attach_snippet(Snippet::new(
                            record.display().to_string(),
                            "client = \"acmee\"\n",
                            9..16,
                            "this client",
                        )),
                )
            },
            |_| {},
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "pace doesn't save the edit until this mistake is fixed: `pace project edit webshop` opens it\nthe client is unknown"
        );
    }

    #[test]
    fn mistake_in_edited_file_names_no_command() {
        let edit = editor(&["after"]);

        let error = edit_until_checked(
            roles(),
            "before",
            &Scripted::without_terminal(),
            edit,
            |_| -> PaceResult<()> {
                Err(
                    PaceError::new(ErrorKind::InvalidInput, "the kind is unknown").attach_snippet(
                        Snippet::new("roles.toml", "kind = 1\n", 7..8, "this kind"),
                    ),
                )
            },
            |_| {},
        )
        .unwrap_err();

        assert_eq!(error.to_string(), "the kind is unknown");
    }

    #[test]
    fn unchanged_text_has_nothing_to_do() {
        let edit = editor(&["before"]);

        let error = edit_until_checked(
            roles(),
            "before",
            &Scripted::without_terminal(),
            edit,
            no_oops,
            |_| {},
        )
        .unwrap_err();

        assert_eq!(error.to_string(), "nothing changed in roles.toml");
        assert_eq!(error.exit_code(), ExitCode::NothingToDo);
    }

    #[test]
    fn failing_check_of_other_kind_opens_nothing_again() {
        let edit = editor(&["after"]);
        let asker = Scripted::confirming([true]);

        let error = edit_until_checked(
            roles(),
            "before",
            &asker,
            edit,
            |_| -> PaceResult<()> { Err(PaceError::new(ErrorKind::InputOutput, "disk full")) },
            |_| {},
        )
        .unwrap_err();

        assert_eq!(error.to_string(), "disk full");
        assert_eq!(asker.questions(), Vec::<String>::new());
    }

    fn env<'vars>(vars: &'vars [(&str, &str)]) -> impl Fn(&str) -> Option<OsString> + 'vars {
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

    #[test]
    fn editor_with_unclosed_quote_names_command() {
        let error = edit_in_editor("vi 'notes", "", ".toml").unwrap_err();

        assert_eq!(
            error.to_string(),
            "pace can't read the editor command `vi 'notes`"
        );
        assert_eq!(error.hint(), Some("check the quotes in VISUAL or EDITOR"));
    }

    #[test]
    fn editor_without_words_is_empty_command() {
        let error = edit_in_editor(" ", "", ".toml").unwrap_err();

        assert_eq!(error.to_string(), "the editor command is empty");
        assert_eq!(
            error.hint(),
            Some("set VISUAL or EDITOR to the editor you use, for example `nano`")
        );
    }

    #[test]
    fn editor_that_does_not_exist_names_command() {
        let error = edit_in_editor("pace-has-no-such-editor", "", ".toml").unwrap_err();

        assert_eq!(
            error.to_string(),
            "pace can't start the editor `pace-has-no-such-editor`"
        );
        assert_eq!(
            error.hint(),
            Some("set VISUAL or EDITOR to the editor you use, for example `nano`")
        );
    }

    #[cfg(unix)]
    #[test]
    fn editor_that_fails_changes_nothing() {
        let error = edit_in_editor("false", "", ".toml").unwrap_err();

        assert_eq!(
            error.to_string(),
            "the editor `false` ended with exit status: 1"
        );
        assert_eq!(error.hint(), Some("nothing was changed"));
    }

    #[cfg(unix)]
    #[test]
    fn editor_that_removes_file_changes_nothing() {
        let error = edit_in_editor("rm", "", ".toml").unwrap_err();

        assert_eq!(
            error.to_string(),
            "pace can't read the file back from the editor"
        );
        assert_eq!(error.hint(), Some("nothing was changed"));
    }

    #[cfg(unix)]
    #[test]
    fn editor_gets_text_in_file_with_suffix_and_returns_saved_text() {
        // POSIX sed: GNU's `-i` without a suffix fails on macOS's BSD sed.
        let editor = "sh -c 'case \"$0\" in *.toml) \
            sed s/draft/offer/ \"$0\" > \"$0.new\" && mv \"$0.new\" \"$0\";; esac'";

        let text = edit_in_editor(editor, "note = \"draft\"\n", ".toml").unwrap();

        assert_eq!(text, "note = \"offer\"\n");
    }
}
