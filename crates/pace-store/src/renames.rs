//! The renames that put the files of one write in place, all or none: the
//! event files of a batch, or the TOML files of a field edit.

use std::{
    io,
    mem::DropGuard,
    path::{Path, PathBuf},
};

use pace_error::{PaceError, PaceResult};
use tempfile::NamedTempFile;

use crate::disk::{FileOp, io_error, write_atomically};

/// A file of a write, its new text in a temporary file next to it.
pub(crate) struct Prepared {
    /// The file's new text.
    pub temporary: NamedTempFile,
    /// The file.
    pub path: PathBuf,
    /// The file's text before the write; `None` if the write creates it.
    pub before: Option<String>,
}

/// The rename step of [`rename_all`] outside tests: `temporary` over `path`.
pub(crate) fn persist(temporary: NamedTempFile, path: &Path) -> io::Result<()> {
    temporary
        .persist(path)
        .map(drop)
        .map_err(|error| error.error)
}

/// A file that [`rename_all`] renamed, and its text before.
struct Renamed {
    path: PathBuf,
    before: Option<String>,
}

/// Rename each of `files` over its file with `rename`, all or none.
///
/// A guard holds the files renamed so far. If a rename fails, the guard is
/// dismissed and [`restore`] puts them back, so a restore that fails ends
/// up in the error. On a panic, the guard puts them back and reports to no
/// one. Once every rename holds, the guard is dismissed.
///
/// If a file can't be put back, the error names it, with `kept_hint` as
/// the way to take back its part.
pub(crate) fn rename_all(
    files: Vec<Prepared>,
    mut rename: impl FnMut(NamedTempFile, &Path) -> io::Result<()>,
    kept_hint: &'static str,
) -> PaceResult<()> {
    let mut renamed = DropGuard::new(Vec::with_capacity(files.len()), |renamed: Vec<Renamed>| {
        _ = restore(&renamed);
    });
    for Prepared {
        temporary,
        path,
        before,
    } in files
    {
        if let Err(error) = rename(temporary, &path) {
            let error = io_error(FileOp::Write, &path, error);
            let kept = restore(&DropGuard::dismiss(renamed));
            return Err(if kept.is_empty() {
                error
            } else {
                keeps_part(error, &kept, kept_hint)
            });
        }
        renamed.push(Renamed { path, before });
    }
    _ = DropGuard::dismiss(renamed);
    Ok(())
}

/// Put back each of `renamed`, the last first: write its text before, or
/// remove it if the write created it. Returns the files it could not put
/// back; it never panics, as it runs in a guard during an unwind too.
fn restore(renamed: &[Renamed]) -> Vec<PathBuf> {
    renamed
        .iter()
        .rev()
        .filter(|renamed| !put_back(renamed))
        .map(|renamed| renamed.path.clone())
        .collect()
}

/// Whether `renamed` holds its text before again, or is gone if the write
/// created it.
fn put_back(Renamed { path, before }: &Renamed) -> bool {
    let Some(text) = before else {
        return std::fs::remove_file(path)
            .or_else(|error| match error.kind() {
                io::ErrorKind::NotFound => Ok(()),
                _ => Err(error),
            })
            .is_ok();
    };
    write_atomically(path, text).is_ok()
}

/// `error` of a failed rename, which also names the files `kept` that still
/// hold part of the write, and `hint`, how to take that part back.
fn keeps_part(error: Box<PaceError>, kept: &[PathBuf], hint: &'static str) -> Box<PaceError> {
    let kept = kept
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    error
        .append_guidance_line("{kept} still holds part of the command: pace failed to put it back")
        .attach_context("kept", kept)
        .attach_hint(hint)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pace_core::{
        Batch, ChangeMarker, Clock, Command, DeviceId, Policy, ReadScope, RecordedEvent, Rules,
        State, command::Begin, testing::SequentialIds, testing::TestClock,
    };

    use super::*;
    use crate::{EventStore, FilesStore, Split, Tracker, testing::scratch_dir};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// A laptop with the split `day` whose "Write offer" began on the
    /// evening of the 26th, at 09:00 on the 27th. A `begin` now ends the
    /// entry in the 26th's file and begins one in the 27th's.
    struct Laptop {
        dir: tempfile::TempDir,
        tracker: Tracker<FilesStore>,
        clock: Arc<TestClock>,
        device: DeviceId,
    }

    impl Laptop {
        fn new() -> Result<Self, Box<dyn std::error::Error>> {
            let dir = scratch_dir()?;
            let device: DeviceId = "0193a8d0-000d-7000-8000-000000000001".parse()?;
            let clock = Arc::new(TestClock::new("2026-11-26T22:00[Europe/Berlin]".parse()?));
            let store = FilesStore::new(
                dir.path().join("home"),
                dir.path().join("machine"),
                device,
                "laptop".to_owned(),
                clock.clone(),
                Split::Day,
            );
            let tracker = Tracker::new(
                store,
                clock.clone(),
                Arc::new(SequentialIds::starting_at(&clock.now(), 1)),
                Policy::default(),
                device,
            );
            _ = tracker.run(&Command::Begin(Begin {
                note: Some("Write offer".parse()?),
                ..Begin::default()
            }))?;
            clock.set("2026-11-27T09:00[Europe/Berlin]".parse()?);
            Ok(Self {
                dir,
                tracker,
                clock,
                device,
            })
        }

        fn store(&self) -> &FilesStore {
            self.tracker.store()
        }

        fn log(&self) -> PaceResult<Vec<RecordedEvent>> {
            self.store().read(&ReadScope::everything())
        }

        /// The batch of `begin Review`, and the marker it was decided on.
        fn begin_review(&self) -> PaceResult<(Batch, ChangeMarker)> {
            let marker = self.store().marker()?;
            let now = self.clock.now();
            let batch = Rules {
                state: &State::from_events(&self.log()?),
                now: &now,
                device: self.device,
                ids: &SequentialIds::starting_at(&now, 2),
                policy: &Policy::default(),
            }
            .decide(&Command::Begin(Begin {
                note: Some("Review".parse()?),
                ..Begin::default()
            }))?;
            Ok((batch, marker))
        }

        fn day_file(&self, day: &str) -> PathBuf {
            self.dir
                .path()
                .join("home/events")
                .join(self.device.to_string())
                .join(format!("2026/11/2026-11-{day}.toml"))
        }
    }

    /// A rename step whose rename number `failing` fails, and that calls
    /// `before_failing` first.
    fn failing_rename(
        failing: usize,
        mut before_failing: impl FnMut(),
    ) -> impl FnMut(NamedTempFile, &Path) -> io::Result<()> {
        let mut renames = 0;
        move |temporary, path| {
            renames += 1;
            if renames == failing {
                before_failing();
                Err(io::Error::other("the test fails this rename"))
            } else {
                persist(temporary, path)
            }
        }
    }

    #[test]
    fn failed_second_rename_leaves_log_as_before_batch() -> TestResult {
        let laptop = Laptop::new()?;
        let before = laptop.log()?;
        let yesterday = std::fs::read_to_string(laptop.day_file("26"))?;
        let (batch, marker) = laptop.begin_review()?;

        let appended = laptop
            .store()
            .append_with(batch, marker, failing_rename(2, || ()));

        let error = appended.err().ok_or("the second rename fails")?;
        assert_eq!(
            error.to_string(),
            format!("cannot write {}", laptop.day_file("27").display())
        );
        assert_eq!(std::fs::read_to_string(laptop.day_file("26"))?, yesterday);
        assert!(!laptop.day_file("27").exists(), "the batch created it");
        assert_eq!(laptop.log()?, before);
        Ok(())
    }

    #[test]
    fn panic_in_rename_leaves_log_as_before_batch() -> TestResult {
        let laptop = Laptop::new()?;
        let before = laptop.log()?;
        let (batch, marker) = laptop.begin_review()?;

        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            laptop.store().append_with(
                batch,
                marker,
                failing_rename(2, || panic!("the test panics in the second rename")),
            )
        }));

        assert!(panicked.is_err());
        assert_eq!(laptop.log()?, before);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn failed_restore_names_file_that_keeps_part_of_batch() -> TestResult {
        use std::os::unix::fs::PermissionsExt as _;

        let laptop = Laptop::new()?;
        let (batch, marker) = laptop.begin_review()?;
        let month = laptop
            .day_file("26")
            .parent()
            .ok_or("the month's folder")?
            .to_path_buf();
        let mode = |mode| std::fs::Permissions::from_mode(mode);

        // A read-only directory takes no temporary file to put the 26th back.
        let appended = laptop.store().append_with(
            batch,
            marker,
            failing_rename(2, || {
                _ = std::fs::set_permissions(&month, mode(0o555));
            }),
        );
        std::fs::set_permissions(&month, mode(0o755))?;

        let error = appended.err().ok_or("the second rename fails")?;
        assert!(
            error.to_string().contains(&format!(
                "{} still holds part",
                laptop.day_file("26").display()
            )),
            "{error}"
        );
        assert_eq!(
            error.hint(),
            Some("run `pace undo` to take back the part that pace recorded")
        );
        Ok(())
    }
}
