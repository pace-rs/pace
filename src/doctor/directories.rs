//! The directory checks of `pace doctor`: pace can write to the config
//! file's directory, the data and the machine directory, or create them;
//! the machine directory lies outside the data directory; and whether
//! portable mode is on.
//!
//! [`probe_dir`] does the I/O: it writes a hidden probe file into the
//! directory, or into the nearest part of its path that exists, and
//! removes it at once. [`place_finding`] decides from its [`Probe`].

use std::path::{Path, PathBuf};

use crate::dirs::{Dirs, GLOBAL_CONFIG_FILE, Located, machine_inside_data};
use crate::doctor::{Finding, FindingArea, FindingSeverity};
use crate::origin::Origin;

/// One of the three places as doctor checks it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Place {
    /// The place in words: "the data directory".
    what: &'static str,
    /// The directory.
    path: PathBuf,
    /// The variable that points pace somewhere else.
    variable: &'static str,
    /// How bad it is when pace can't write there.
    severity: FindingSeverity,
}

/// The three places of `dirs`. Only `pace config set` and `pace setup`
/// write the config file, so its directory gives a warning.
fn places(dirs: &Dirs) -> [Place; 3] {
    let config_dir = dirs
        .config_file
        .path
        .parent()
        .map_or_else(PathBuf::new, Path::to_path_buf);
    [
        Place {
            what: "the config file's directory",
            path: config_dir,
            variable: "PACE_CONFIG",
            severity: FindingSeverity::Warning,
        },
        Place {
            what: "the data directory",
            path: dirs.data.path.clone(),
            variable: "PACE_HOME",
            severity: FindingSeverity::Error,
        },
        Place {
            what: "the machine directory",
            path: dirs.machine.path.clone(),
            variable: "PACE_MACHINE_DIR",
            severity: FindingSeverity::Error,
        },
    ]
}

/// What the probe of a directory found.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Probe {
    /// It exists and pace may write to it.
    Writable,
    /// It doesn't exist yet, and pace may create it with its first write.
    Creatable,
    /// A file lies where the directory should be.
    NotADirectory,
    /// It exists, but pace may not write to it, for `reason`.
    NotWritable(String),
    /// It doesn't exist, and pace can't create it in `parent`, the nearest
    /// part of its path that exists, for `reason`.
    NotCreatable { parent: PathBuf, reason: String },
}

/// Probe `dir`: whether pace may write to it, or create it.
fn probe_dir(dir: &Path) -> Probe {
    if dir.is_dir() {
        return write_probe(dir).map_or_else(Probe::NotWritable, |()| Probe::Writable);
    }
    if dir.exists() {
        return Probe::NotADirectory;
    }
    let Some(parent) = dir.ancestors().skip(1).find(|parent| parent.exists()) else {
        return Probe::NotCreatable {
            parent: dir.to_path_buf(),
            reason: "no part of its path exists".to_owned(),
        };
    };
    let reason = if parent.is_dir() {
        write_probe(parent).err()
    } else {
        Some(format!("{} is a file", parent.display()))
    };
    reason.map_or(Probe::Creatable, |reason| Probe::NotCreatable {
        parent: parent.to_path_buf(),
        reason,
    })
}

/// Write a hidden file into `dir` and remove it again; the error says
/// why pace can't.
fn write_probe(dir: &Path) -> Result<(), String> {
    tempfile::Builder::new()
        .prefix(".pace-doctor-")
        .tempfile_in(dir)
        .map(drop)
        .map_err(|error| error.to_string())
}

/// The finding about `place` that its `probe` gives; `None` if pace may
/// write there.
fn place_finding(place: &Place, probe: Probe) -> Option<Finding> {
    let Place {
        what,
        path,
        variable,
        severity,
    } = place;
    let shown = path.display();
    let (message, fix) = match probe {
        Probe::Writable | Probe::Creatable => return None,
        Probe::NotADirectory => (
            format!("{what} {shown} is a file, not a directory"),
            format!("move the file away, or point {variable} at a directory"),
        ),
        Probe::NotWritable(reason) => (
            format!("pace cannot write to {what} {shown}: {reason}"),
            format!("give your user write access to {shown}, or point {variable} elsewhere"),
        ),
        Probe::NotCreatable { parent, reason } => (
            format!(
                "pace cannot create {what} {shown} in {}: {reason}",
                parent.display()
            ),
            format!("create {shown} for your user, or point {variable} elsewhere"),
        ),
    };
    Some(
        Finding::new(*severity, FindingArea::Directories, message)
            .in_file(path)
            .with_fix(fix),
    )
}

/// The warning that portable mode is on, if one of `dirs` comes from it:
/// a `pace.global.toml` beside the executable keeps pace's files there.
fn portable_mode(dirs: &Dirs) -> Option<Finding> {
    let located: [&Located; 3] = [&dirs.config_file, &dirs.data, &dirs.machine];
    let dir = located
        .into_iter()
        .find(|located| located.origin == Origin::Portable)?
        .path
        .parent()?;
    let file = dir.join(GLOBAL_CONFIG_FILE);
    Some(
        Finding::new(
            FindingSeverity::Warning,
            FindingArea::Directories,
            format!(
                "portable mode is on: pace keeps its files beside its executable in {}",
                dir.display()
            ),
        )
        .in_file(&file)
        .with_fix(format!(
            "to keep them in your user's directories, move {} away",
            file.display()
        )),
    )
}

/// Every finding about the directories of `dirs`: each place pace can't
/// write to or create, a machine directory inside the data directory, and
/// portable mode.
#[must_use]
pub fn directory_findings(dirs: &Dirs) -> Vec<Finding> {
    let mut findings: Vec<Finding> = places(dirs)
        .iter()
        .filter_map(|place| place_finding(place, probe_dir(&place.path)))
        .collect();
    findings.extend(machine_inside_data(dirs).map(|error| {
        Finding::from_error(FindingSeverity::Error, FindingArea::Directories, &error)
            .in_file(&dirs.machine.path)
    }));
    findings.extend(portable_mode(dirs));
    findings
}

#[cfg(test)]
mod tests {
    use pace_store::testing::scratch_dir;

    use super::*;

    fn located(path: &Path, origin: Origin) -> Located {
        Located {
            path: path.to_path_buf(),
            origin,
        }
    }

    fn dirs_in(root: &Path, origin: &Origin) -> Dirs {
        Dirs {
            config_file: located(&root.join(GLOBAL_CONFIG_FILE), origin.clone()),
            data: located(&root.join("data"), origin.clone()),
            machine: located(&root.join("machine"), origin.clone()),
        }
    }

    fn data_place(path: &Path) -> Place {
        Place {
            what: "the data directory",
            path: path.to_path_buf(),
            variable: "PACE_HOME",
            severity: FindingSeverity::Error,
        }
    }

    #[test]
    fn existing_and_missing_directories_probe_as_pace_may_write() -> Result<(), std::io::Error> {
        let dir = scratch_dir()?;
        std::fs::write(dir.path().join("file"), "")?;

        assert_eq!(probe_dir(dir.path()), Probe::Writable);
        assert_eq!(probe_dir(&dir.path().join("new/deeper")), Probe::Creatable);
        assert_eq!(probe_dir(&dir.path().join("file")), Probe::NotADirectory);
        assert_eq!(
            probe_dir(&dir.path().join("file/below")),
            Probe::NotCreatable {
                parent: dir.path().join("file"),
                reason: format!("{} is a file", dir.path().join("file").display()),
            }
        );
        assert_eq!(
            std::fs::read_dir(dir.path())?.count(),
            1,
            "the probe leaves nothing"
        );
        Ok(())
    }

    #[test]
    fn directory_pace_may_write_or_create_is_no_finding() {
        let place = data_place(Path::new("/sync/pace"));

        assert_eq!(place_finding(&place, Probe::Writable), None);
        assert_eq!(place_finding(&place, Probe::Creatable), None);
    }

    #[test]
    fn directory_pace_cannot_write_names_why_and_its_variable() {
        let place = data_place(Path::new("/sync/pace"));

        let finding = place_finding(&place, Probe::NotWritable("Permission denied".to_owned()));

        assert_eq!(
            finding,
            Some(
                Finding::new(
                    FindingSeverity::Error,
                    FindingArea::Directories,
                    "pace cannot write to the data directory /sync/pace: Permission denied",
                )
                .in_file(Path::new("/sync/pace"))
                .with_fix(
                    "give your user write access to /sync/pace, or point PACE_HOME elsewhere"
                )
            )
        );
    }

    #[test]
    fn file_in_place_of_directory_and_uncreatable_directory_are_errors() {
        let place = data_place(Path::new("/sync/pace"));
        let file = place_finding(&place, Probe::NotADirectory);
        let uncreatable = place_finding(
            &place,
            Probe::NotCreatable {
                parent: PathBuf::from("/sync"),
                reason: "Read-only file system".to_owned(),
            },
        );

        assert_eq!(
            file.map(|finding| finding.message),
            Some("the data directory /sync/pace is a file, not a directory".to_owned())
        );
        assert_eq!(
            uncreatable.map(|finding| finding.message),
            Some(
                "pace cannot create the data directory /sync/pace in /sync: Read-only file system"
                    .to_owned()
            )
        );
    }

    #[test]
    fn portable_mode_is_warning_that_names_its_file() {
        let root = Path::new("/usb/pace");

        let portable = portable_mode(&dirs_in(root, &Origin::Portable));

        assert_eq!(
            portable.as_ref().map(|finding| finding.severity),
            Some(FindingSeverity::Warning)
        );
        assert_eq!(
            portable.and_then(|finding| finding.file),
            Some(root.join(GLOBAL_CONFIG_FILE))
        );
        assert_eq!(portable_mode(&dirs_in(root, &Origin::Default)), None);
    }

    #[test]
    fn machine_directory_inside_data_directory_is_error() -> Result<(), std::io::Error> {
        let dir = scratch_dir()?;
        let mut dirs = dirs_in(dir.path(), &Origin::Default);
        dirs.machine.path = dirs.data.path.join("machine");

        let findings = directory_findings(&dirs);

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, FindingSeverity::Error);
        assert_eq!(findings[0].file.as_ref(), Some(&dirs.machine.path));
        Ok(())
    }
}
