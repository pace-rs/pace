//! The file checks of `pace doctor`: the global config file, the catalog's
//! files, the manifest in reach and every event file read.
//!
//! The app reads the files; `pace_store` checks their texts and names
//! every mistake with its file and line, where a command stops at the
//! first: [`check_catalog_files`] for the catalog,
//! [`parse_events_collecting`] for each event file.

use std::{
    ffi::OsString,
    io,
    path::{Path, PathBuf},
};

use pace_core::RecordedEvent;
use pace_error::PaceError;
use pace_store::{
    catalog::{catalog_file_paths, check_catalog_files, read_catalog},
    event_file_paths, parse_events_collecting,
    toml_file::read_hint,
};

use crate::commands::mending_command;
use crate::config::load_settings;
use crate::dirs::{Dirs, Located};
use crate::doctor::{Finding, FindingArea, FindingSeverity};
use crate::project::{Records, discover_project};

/// What the file checks found: the findings, and the events that read,
/// for the checks that look at the events.
#[derive(Debug, Default)]
pub struct FileChecks {
    /// A finding for each mistake, in the order of the files.
    pub findings: Vec<Finding>,
    /// The valid events of every event file.
    pub events: Vec<RecordedEvent>,
}

/// The error finding of a file in `files`.
fn file_error(error: &PaceError) -> Finding {
    Finding::from_error(FindingSeverity::Error, FindingArea::Files, error)
}

/// The finding of the file `path` that can't be read for `error`.
fn unreadable(path: &Path, error: &io::Error) -> Finding {
    Finding::new(
        FindingSeverity::Error,
        FindingArea::Files,
        format!("cannot read {}: {error}", path.display()),
    )
    .in_file(path)
    .with_fix(read_hint(error))
}

/// Every finding about the files of `dirs`, with the manifest in reach of
/// `cwd`, and the events that read.
#[must_use]
pub fn file_findings(dirs: &Dirs, cwd: &Path) -> FileChecks {
    let home = &dirs.data.path;
    let mut findings = settings_findings(&dirs.config_file);
    findings.extend(catalog_findings(home));
    findings.extend(manifest_findings(home, cwd));
    let events = event_findings(home);
    findings.extend(events.findings);
    FileChecks {
        findings,
        events: events.events,
    }
}

/// The finding of the global config file and of the settings in the
/// environment, if they don't load; else a warning for each problem that
/// doesn't stop pace, such as an unknown `PACE_*` variable.
fn settings_findings(config_file: &Located) -> Vec<Finding> {
    let user_home = directories::BaseDirs::new().map(|base| base.home_dir().to_path_buf());
    let loaded = load_settings(
        Some(config_file),
        user_home.as_deref(),
        |name: &str| -> Option<OsString> { std::env::var_os(name) },
        std::env::vars_os().map(|(name, _)| name),
    );
    match loaded {
        Ok(loaded) => loaded
            .warnings
            .iter()
            .map(|warning| {
                Finding::from_error(FindingSeverity::Warning, FindingArea::Files, warning)
            })
            .collect(),
        Err(error) => vec![file_error(&error)],
    }
}

/// A finding for each mistake of the catalog's files in `home`; its fix
/// names the command that opens the file, where there is one.
fn catalog_findings(home: &Path) -> Vec<Finding> {
    let paths = match catalog_file_paths(home) {
        Ok(paths) => paths,
        Err(error) => return vec![file_error(&error)],
    };
    let mut findings = Vec::new();
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    for path in paths {
        match std::fs::read_to_string(&path) {
            Ok(text) => files.push((path, text)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => findings.push(unreadable(&path, &error)),
        }
    }
    findings.extend(
        check_catalog_files(home, files)
            .iter()
            .map(|mistake| with_mending_command(home, file_error(mistake))),
    );
    findings
}

/// `finding` with the command that opens its catalog file in the editor
/// after its fix, such as `pace role edit`.
fn with_mending_command(home: &Path, finding: Finding) -> Finding {
    let Some(command) = finding
        .file
        .as_deref()
        .and_then(|file| mending_command(home, file))
    else {
        return finding;
    };
    let fix = finding.fix.as_ref().map_or_else(
        || format!("`{command}` opens the file"),
        |fix| format!("{fix}; `{command}` opens the file"),
    );
    finding.with_fix(fix)
}

/// The finding of the manifest in reach of `cwd`, if it doesn't read; its
/// project is checked against the catalog of `home`, if that reads.
fn manifest_findings(home: &Path, cwd: &Path) -> Vec<Finding> {
    let catalog = read_catalog(home).unwrap_or_default();
    let records = Records {
        catalog: &catalog,
        home,
    };
    match discover_project(None, None, cwd, &[], records) {
        Ok(_) => Vec::new(),
        Err(error) => vec![file_error(&error)],
    }
}

/// A finding for each invalid event of every event file in `home`, and
/// the events that read.
fn event_findings(home: &Path) -> FileChecks {
    let paths = match event_file_paths(home) {
        Ok(paths) => paths,
        Err(error) => {
            return FileChecks {
                findings: vec![file_error(&error)],
                events: Vec::new(),
            };
        }
    };
    let mut checks = FileChecks::default();
    for (device, path) in paths {
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let read = parse_events_collecting(&path, &text, device);
                checks
                    .findings
                    .extend(read.mistakes.iter().map(|mistake| file_error(mistake)));
                checks.events.extend(read.events);
            }
            Err(error) => checks.findings.push(unreadable(&path, &error)),
        }
    }
    checks
}

#[cfg(test)]
mod tests {
    use pace_store::testing::scratch_dir;

    use super::*;

    #[test]
    fn mistake_of_roles_file_names_role_edit_after_its_hint() {
        let home = Path::new("/home/pace");
        let finding = Finding::new(FindingSeverity::Error, FindingArea::Files, "bad kind")
            .in_file(&home.join("roles.toml"))
            .with_fix("did you mean `area`?");

        let mended = with_mending_command(home, finding);

        assert_eq!(
            mended.fix.as_deref(),
            Some("did you mean `area`?; `pace role edit` opens the file")
        );
    }

    #[test]
    fn mistake_without_hint_or_outside_catalog_keeps_its_fix() {
        let home = Path::new("/home/pace");
        let bare = Finding::new(FindingSeverity::Error, FindingArea::Files, "bad")
            .in_file(&home.join("clients.toml"));
        let elsewhere = Finding::new(FindingSeverity::Error, FindingArea::Files, "bad")
            .in_file(Path::new("/work/pace.toml"));

        assert_eq!(
            with_mending_command(home, bare).fix.as_deref(),
            Some("`pace client edit KEY` opens the file")
        );
        assert_eq!(with_mending_command(home, elsewhere.clone()), elsewhere);
    }

    #[test]
    fn unreadable_catalog_file_is_named_with_the_hint_to_read_it() -> Result<(), io::Error> {
        let home = scratch_dir()?;
        std::fs::create_dir(home.path().join("roles.toml"))?;

        let findings = catalog_findings(home.path());

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].message.starts_with("cannot read "),
            "{}",
            findings[0].message
        );
        assert_eq!(findings[0].file, Some(home.path().join("roles.toml")));
        Ok(())
    }

    #[test]
    fn events_directory_with_foreign_directory_is_one_finding() -> Result<(), io::Error> {
        let home = scratch_dir()?;
        std::fs::create_dir_all(home.path().join("events/notes"))?;

        let checks = event_findings(home.path());

        assert_eq!(checks.findings.len(), 1, "{:?}", checks.findings);
        assert_eq!(checks.events.len(), 0);
        Ok(())
    }
}
