//! The store's files on disk: atomic writes, the lock, the event files of a
//! device directory, and the error of a failed file operation.

use std::{
    fs::{File, OpenOptions},
    io::Write as _,
    path::{Path, PathBuf},
};

use pace_error::{ErrorKind, PaceError, PaceResult};

/// `device.toml`: in the machine directory the device ID, in a device
/// directory the device's name and split.
pub const DEVICE_FILE: &str = "device.toml";

/// Remove the empty directories below `dir`, deepest first; `dir` stays.
pub fn remove_empty_dirs(dir: &Path) -> PaceResult<()> {
    for entry in std::fs::read_dir(dir).map_err(|error| io_error("read", dir, error))? {
        let path = entry.map_err(|error| io_error("read", dir, error))?.path();
        if !path.is_dir() {
            continue;
        }
        remove_empty_dirs(&path)?;
        let empty = std::fs::read_dir(&path)
            .map_err(|error| io_error("read", &path, error))?
            .next()
            .is_none();
        if empty {
            std::fs::remove_dir(&path).map_err(|error| io_error("remove", &path, error))?;
        }
    }
    Ok(())
}

/// The event files in a device directory, oldest period first.
pub fn event_files(dir: &Path) -> PaceResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).map_err(|error| io_error("read", &dir, error))? {
            let path = entry.map_err(|error| io_error("read", &dir, error))?.path();
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

pub fn create_dir(dir: &Path) -> PaceResult<()> {
    std::fs::create_dir_all(dir).map_err(|error| io_error("create", dir, error))
}

/// The lock file, locked exclusively until the returned file is dropped.
pub fn lock(path: &Path) -> PaceResult<File> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .map_err(|error| io_error("open", path, error))?;
    file.lock().map_err(|error| io_error("lock", path, error))?;
    Ok(file)
}

/// The name of an event file in the index: its path below `events/`, with
/// `/` as separator on every OS.
pub fn relative_name(events: &Path, path: &Path) -> String {
    path.strip_prefix(events)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Write `text` to `path` through a temporary file next to it.
pub fn write_atomically(path: &Path, text: &str) -> PaceResult<()> {
    let (temporary, path) = prepare(path, text)?;
    _ = temporary
        .persist(&path)
        .map_err(|error| io_error("write", &path, error.error))?;
    Ok(())
}

/// `text` in a temporary file next to `path`, ready to be renamed over it.
pub fn prepare(path: &Path, text: &str) -> PaceResult<(tempfile::NamedTempFile, PathBuf)> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    create_dir(dir)?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".pace-")
        .tempfile_in(dir)
        .map_err(|error| io_error("create a temporary file in", dir, error))?;
    temporary
        .write_all(text.as_bytes())
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|error| io_error("write", temporary.path(), error))?;
    Ok((temporary, path.to_path_buf()))
}

pub fn io_error(verb: &str, path: &Path, error: std::io::Error) -> Box<PaceError> {
    PaceError::with_source(ErrorKind::InputOutput, "cannot {verb} {path}", error)
        .attach_context("verb", verb)
        .attach_context("path", path.display().to_string())
        .attach_hint("check that the path exists and that pace may write there")
}
