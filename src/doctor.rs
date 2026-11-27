//! The checks of `pace doctor`: each finding with its severity, its area,
//! where it is and how to fix it.
//!
//! [`check_everything`] runs the checks of the directories, the files, the
//! device, the entries, the structure, billing and setup, in that order.
//! Every problem becomes a [`Finding`], so doctor names them all where a
//! command stops at the first. The app gathers the files and the events;
//! the checks of `pace_store` and `pace_core` take their texts and values
//! and do no I/O. Doctor changes nothing: the only file it writes is the
//! hidden probe of a directory, which it removes at once.

mod data;
mod device;
mod directories;
mod files;
mod report;

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use pace_core::DataArea;
use pace_error::PaceError;

use crate::dirs::Dirs;

pub use data::data_check_findings;
pub use device::{device_findings, unfinished_setup};
pub use directories::directory_findings;
pub use files::{FileChecks, file_findings};
pub use pace_core::FindingSeverity;
pub use report::{doctor_lines, doctor_rows};

/// The part of pace a finding is about: a row of doctor's checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingArea {
    /// The config file's directory, the data and the machine directory.
    Directories,
    /// The global config file, the catalog's files, the manifest and the
    /// event files.
    Files,
    /// This machine's device ID.
    Device,
    /// The entries: open entries, overlaps, changes on two devices,
    /// projects without a record.
    Entries,
    /// The roles, clients and project records against each other.
    Structure,
    /// Billed and billable entries against the rates.
    Billing,
    /// An interrupted `pace setup`.
    Setup,
}

impl FindingArea {
    /// The area as the findings name it: `directories`, `files`, …
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Directories => "directories",
            Self::Files => "files",
            Self::Device => "device",
            Self::Entries => "entries",
            Self::Structure => "structure",
            Self::Billing => "billing",
            Self::Setup => "setup",
        }
    }
}

impl From<DataArea> for FindingArea {
    fn from(area: DataArea) -> Self {
        match area {
            DataArea::Entries => Self::Entries,
            DataArea::Structure => Self::Structure,
            DataArea::Billing => Self::Billing,
        }
    }
}

/// One problem doctor found: what is wrong, where, and how to fix it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// How bad it is.
    pub severity: FindingSeverity,
    /// The part of pace it is about.
    pub area: FindingArea,
    /// What is wrong, in one line.
    pub message: String,
    /// The file or directory it is about, if it is about one.
    pub file: Option<PathBuf>,
    /// The line in `file`, counted from 1.
    pub line: Option<usize>,
    /// The short ID of the entry it is about, if it is about one.
    pub entry: Option<String>,
    /// The command or the step that fixes it.
    pub fix: Option<String>,
}

impl Finding {
    /// A finding of `severity` in `area` that says `message`.
    #[must_use]
    pub fn new(severity: FindingSeverity, area: FindingArea, message: impl Into<String>) -> Self {
        Self {
            severity,
            area,
            message: message.into(),
            file: None,
            line: None,
            entry: None,
            fix: None,
        }
    }

    /// The finding an error of pace makes: the error's message in one
    /// line, the file and the line its snippet points at (or else the
    /// file it names), and its hint as the fix.
    #[must_use]
    pub fn from_error(severity: FindingSeverity, area: FindingArea, error: &PaceError) -> Self {
        let file = error
            .snippet_name()
            .or_else(|| {
                ["path", "file"]
                    .into_iter()
                    .find_map(|key| error.context_value(key))
            })
            .map(PathBuf::from);
        Self {
            file,
            line: error.snippet_line(),
            fix: error.hint().map(str::to_owned),
            ..Self::new(severity, area, one_line(&error.to_string()))
        }
    }

    /// The finding about the file or directory `file`.
    #[must_use]
    pub fn in_file(self, file: &Path) -> Self {
        Self {
            file: Some(file.to_path_buf()),
            ..self
        }
    }

    /// The finding with `fix` as the way to fix it.
    #[must_use]
    pub fn with_fix(self, fix: impl Into<String>) -> Self {
        Self {
            fix: Some(fix.into()),
            ..self
        }
    }
}

/// `text` in one line: its lines joined by a space.
fn one_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether one of `findings` is an error, so doctor exits with 1.
#[must_use]
pub fn has_errors(findings: &[Finding]) -> bool {
    findings
        .iter()
        .any(|finding| finding.severity == FindingSeverity::Error)
}

/// Every finding about the places `dirs`: the directories, the files,
/// the device, the entries, the structure, billing and setup, in that
/// order.
///
/// `cwd` is the working directory whose manifest is in reach, `test_now`
/// the value of `PACE_TEST_NOW`.
#[must_use]
pub fn check_everything(dirs: &Dirs, cwd: &Path, test_now: Option<OsString>) -> Vec<Finding> {
    let mut findings = directory_findings(dirs);
    let mut files = file_findings(dirs, cwd);
    findings.append(&mut files.findings);
    findings.extend(device_findings(&dirs.machine.path, &files.events));
    findings.extend(data_check_findings(&files, test_now));
    findings.extend(unfinished_setup(&dirs.machine.path));
    findings
}

#[cfg(test)]
mod tests {
    use pace_error::ErrorKind;
    use pace_error::Snippet;

    use super::*;

    #[test]
    fn finding_of_error_takes_its_line_and_its_hint_as_fix() {
        let text = "version = 1\n\n[work]\nkind = \"aera\"\n";
        let error = PaceError::new(ErrorKind::InvalidInput, "`aera` is no role kind")
            .attach_hint("did you mean `area`?")
            .attach_snippet(Snippet::new("roles.toml", text, 21..27, "this kind"));

        let finding = Finding::from_error(FindingSeverity::Error, FindingArea::Files, &error);

        assert_eq!(finding.message, "`aera` is no role kind");
        assert_eq!(finding.file, Some(PathBuf::from("roles.toml")));
        assert_eq!(finding.line, Some(4));
        assert_eq!(finding.fix.as_deref(), Some("did you mean `area`?"));
    }

    #[test]
    fn finding_of_error_without_snippet_names_the_path_in_one_line() {
        let error = PaceError::new(ErrorKind::InputOutput, "cannot read {path}\nfor doctor")
            .attach_context("path", "/home/pace/roles.toml");

        let finding = Finding::from_error(FindingSeverity::Error, FindingArea::Files, &error);

        assert_eq!(
            finding.message,
            "cannot read /home/pace/roles.toml for doctor"
        );
        assert_eq!(finding.file, Some(PathBuf::from("/home/pace/roles.toml")));
        assert_eq!(finding.line, None);
        assert_eq!(finding.fix, None);
    }

    #[test]
    fn only_error_makes_doctor_fail() {
        let warning = Finding::new(FindingSeverity::Warning, FindingArea::Setup, "unfinished");
        let error = Finding::new(FindingSeverity::Error, FindingArea::Files, "broken");

        assert!(!has_errors(&[]));
        assert!(!has_errors(std::slice::from_ref(&warning)));
        assert!(has_errors(&[warning, error]));
    }
}
