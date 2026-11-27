//! The store's files on disk: atomic writes, the lock, the event files of a
//! device directory, and the error of a failed file operation.

use std::{
    fs::{File, OpenOptions},
    io::Write as _,
    path::{Path, PathBuf},
};

use pace_error::{ErrorKind, PaceError, PaceResult, Snippet};
use serde::Deserialize;
use toml::Spanned;

/// `device.toml`: in the machine directory the device ID, in a device
/// directory the device's name and split.
pub(crate) const DEVICE_FILE: &str = "device.toml";

/// Remove the empty directories below `dir`, deepest first; `dir` stays.
pub(crate) fn remove_empty_dirs(dir: &Path) -> PaceResult<()> {
    for entry in std::fs::read_dir(dir).map_err(|error| io_error(FileOp::Read, dir, error))? {
        let path = entry
            .map_err(|error| io_error(FileOp::Read, dir, error))?
            .path();
        if !path.is_dir() {
            continue;
        }
        remove_empty_dirs(&path)?;
        let empty = std::fs::read_dir(&path)
            .map_err(|error| io_error(FileOp::Read, &path, error))?
            .next()
            .is_none();
        if empty {
            std::fs::remove_dir(&path).map_err(|error| io_error(FileOp::Remove, &path, error))?;
        }
    }
    Ok(())
}

/// The event files in a device directory, oldest period first.
pub(crate) fn event_files(dir: &Path) -> PaceResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).map_err(|error| io_error(FileOp::Read, &dir, error))? {
            let path = entry
                .map_err(|error| io_error(FileOp::Read, &dir, error))?
                .path();
            let name = path
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                pending.push(path);
            } else if name != DEVICE_FILE
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("toml"))
            {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

pub(crate) fn create_dir(dir: &Path) -> PaceResult<()> {
    std::fs::create_dir_all(dir).map_err(|error| io_error(FileOp::CreateDir, dir, error))
}

/// The lock file, locked exclusively until the returned file is dropped.
pub(crate) fn lock(path: &Path) -> PaceResult<File> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .map_err(|error| io_error(FileOp::OpenLock, path, error))?;
    file.lock()
        .map_err(|error| io_error(FileOp::Lock, path, error))?;
    Ok(file)
}

/// The name of an event file in the index: its path below `events/`, with
/// `/` as separator on every OS.
pub(crate) fn relative_name(events: &Path, path: &Path) -> String {
    path.strip_prefix(events)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Write `text` to `path` through a temporary file next to it.
pub(crate) fn write_atomically(path: &Path, text: &str) -> PaceResult<()> {
    let (temporary, path) = prepare(path, text)?;
    _ = temporary
        .persist(&path)
        .map_err(|error| io_error(FileOp::Write, &path, error.error))?;
    Ok(())
}

/// `text` in a temporary file next to `path`, ready to be renamed over it.
pub(crate) fn prepare(path: &Path, text: &str) -> PaceResult<(tempfile::NamedTempFile, PathBuf)> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    create_dir(dir)?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".pace-")
        .tempfile_in(dir)
        .map_err(|error| io_error(FileOp::CreateTemporary, dir, error))?;
    temporary
        .write_all(text.as_bytes())
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|error| io_error(FileOp::Write, temporary.path(), error))?;
    Ok((temporary, path.to_path_buf()))
}

/// The `version` of a store file, read before its other keys.
#[derive(Deserialize)]
struct Versioned {
    version: Spanned<i64>,
}

/// Check that the store file `path` with the content `text` has the
/// `version` this pace reads. It runs before the file's other keys are
/// read, so a file of a newer pace asks for an update instead of naming a
/// key this pace doesn't know. An older or broken version gets the hint
/// `repair`.
///
/// A file without a readable `version` passes: reading the whole file
/// then names what is wrong with it.
pub(crate) fn check_version(path: &Path, text: &str, reads: u32, repair: &str) -> PaceResult<()> {
    let Ok(Versioned { version }) = toml::from_str::<Versioned>(text) else {
        return Ok(());
    };
    let found = *version.get_ref();
    if found == i64::from(reads) {
        return Ok(());
    }
    let hint = if found > i64::from(reads) {
        "a newer pace wrote this file; update pace"
    } else {
        repair
    };
    Err(PaceError::new(
        ErrorKind::InputOutput,
        "this pace reads version {reads} of {file}, not {found}",
    )
    .attach_context("reads", reads.to_string())
    .attach_context("file", path.display().to_string())
    .attach_context("found", found.to_string())
    .attach_hint(hint)
    .attach_error_code("pace::store::version")
    .attach_snippet(Snippet::new(
        path.display().to_string(),
        text,
        version.span(),
        "this version",
    )))
}

/// A file operation that failed, for [`io_error`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileOp {
    Read,
    Write,
    Remove,
    CreateDir,
    CreateTemporary,
    OpenLock,
    Lock,
}

impl FileOp {
    /// The message, a whole phrase with the placeholder `{path}`.
    const fn guidance(self) -> &'static str {
        match self {
            Self::Read => "cannot read {path}",
            Self::Write => "cannot write {path}",
            Self::Remove => "cannot remove {path}",
            Self::CreateDir => "cannot create the directory {path}",
            Self::CreateTemporary => "cannot create a temporary file in {path}",
            Self::OpenLock => "cannot open the lock file {path}",
            Self::Lock => "cannot lock {path}",
        }
    }

    const fn hint(self) -> &'static str {
        match self {
            Self::Read => "check that the path exists and that pace may read it",
            Self::Write | Self::CreateTemporary => {
                "check that the directory exists and that pace may write there"
            }
            Self::Remove => "check that pace may write to the directory that holds the file",
            Self::CreateDir => "check that pace may create a directory there",
            Self::OpenLock => "check that pace may write to the machine directory",
            Self::Lock => "check that the file system of the machine directory supports file locks",
        }
    }
}

/// The error of the file operation `op` on `path`, caused by `error`.
pub(crate) fn io_error(op: FileOp, path: &Path, error: std::io::Error) -> Box<PaceError> {
    PaceError::with_source(ErrorKind::InputOutput, op.guidance(), error)
        .attach_context("path", path.display().to_string())
        .attach_hint(op.hint())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_read_hints_at_reading() {
        let error = io_error(
            FileOp::Read,
            Path::new("/s/home/events"),
            std::io::ErrorKind::PermissionDenied.into(),
        );

        assert_eq!(error.to_string(), "cannot read /s/home/events");
        assert_eq!(
            error.hint(),
            Some("check that the path exists and that pace may read it")
        );
    }

    #[test]
    fn failed_lock_says_lock() {
        let error = io_error(
            FileOp::Lock,
            Path::new("/s/machine/lock"),
            std::io::ErrorKind::Unsupported.into(),
        );

        assert_eq!(error.to_string(), "cannot lock /s/machine/lock");
    }
}
