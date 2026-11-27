//! A file or directory pace cannot read or write stops it with its path and
//! how to go on; what pace doesn't own in its directories it skips.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use jiff::ZonedDateTime;
use pace_core::{
    Clock, Command, DeviceId, Policy, ReadScope, SequentialIds, command::Begin, testing::TestClock,
};
use pace_error::PaceError;
use pace_store::{EventStore, FilesStore, Split, Tracker, device_id, testing::scratch_dir};
use tempfile::TempDir;

use crate::TestResult;

const DEVICE: &str = "0193a8d0-000d-7000-8000-000000000001";

/// A scratch directory with `home/` and `machine/` for one device.
struct Setup {
    dir: TempDir,
    device: DeviceId,
}

impl Setup {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            dir: scratch_dir()?,
            device: DEVICE.parse()?,
        })
    }

    fn home(&self) -> PathBuf {
        self.dir.path().join("home")
    }

    fn machine(&self) -> PathBuf {
        self.dir.path().join("machine")
    }

    fn device_dir(&self) -> PathBuf {
        self.home().join("events").join(DEVICE)
    }

    fn store(&self, clock: &Arc<TestClock>) -> FilesStore {
        FilesStore::new(
            self.home(),
            self.machine(),
            self.device,
            "laptop".to_owned(),
            clock.clone(),
            Split::Month,
        )
    }

    fn tracker(&self) -> Result<Tracker<FilesStore>, Box<dyn std::error::Error>> {
        let clock = Arc::new(TestClock::new(
            "2026-11-26T09:00[Europe/Berlin]".parse::<ZonedDateTime>()?,
        ));
        Ok(Tracker::new(
            self.store(&clock),
            clock.clone(),
            Arc::new(SequentialIds::starting_at(&clock.now(), 1)),
            Policy::default(),
            self.device,
        ))
    }

    /// The error of a `begin`, which must fail.
    fn begin_fails(&self) -> Result<Box<PaceError>, Box<dyn std::error::Error>> {
        self.tracker()?
            .run(&Command::Begin(Begin::default()))
            .err()
            .ok_or_else(|| "the begin must fail".into())
    }
}

fn ids() -> Result<SequentialIds, jiff::Error> {
    Ok(SequentialIds::starting_at(
        &"2026-11-26T09:00[Europe/Berlin]".parse()?,
        7,
    ))
}

fn message(guidance: &str, path: &Path) -> String {
    guidance.replace("{path}", &path.display().to_string())
}

#[test]
fn device_directory_taken_by_file_cannot_be_created() -> TestResult {
    let setup = Setup::new()?;
    std::fs::create_dir_all(setup.home().join("events"))?;
    std::fs::write(setup.device_dir(), "")?;

    let error = setup.begin_fails()?;

    assert_eq!(
        error.to_string(),
        message("cannot create the directory {path}", &setup.device_dir())
    );
    assert_eq!(
        error.hint(),
        Some("check that pace may create a directory there")
    );
    Ok(())
}

#[test]
fn lock_file_taken_by_directory_cannot_be_opened() -> TestResult {
    let setup = Setup::new()?;
    let lock = setup.machine().join("lock");
    std::fs::create_dir_all(&lock)?;

    let error = setup.begin_fails()?;

    assert_eq!(
        error.to_string(),
        message("cannot open the lock file {path}", &lock)
    );
    assert_eq!(
        error.hint(),
        Some("check that pace may write to the machine directory")
    );
    Ok(())
}

#[test]
fn cursors_file_taken_by_directory_stops_writing() -> TestResult {
    let setup = Setup::new()?;
    let cursors = setup.machine().join("cursors.toml");
    std::fs::create_dir_all(&cursors)?;

    let error = setup.begin_fails()?;

    assert_eq!(error.to_string(), message("cannot read {path}", &cursors));
    assert_eq!(
        error.hint(),
        Some("check that the path exists and that pace may read it")
    );
    Ok(())
}

#[test]
fn event_file_taken_by_directory_takes_no_append() -> TestResult {
    let setup = Setup::new()?;
    let month = setup.device_dir().join("2026/2026-11.toml");
    std::fs::create_dir_all(&month)?;

    let error = setup.begin_fails()?;

    assert_eq!(error.to_string(), message("cannot read {path}", &month));
    Ok(())
}

#[test]
fn events_path_taken_by_file_cannot_be_read() -> TestResult {
    let setup = Setup::new()?;
    let events = setup.home().join("events");
    std::fs::create_dir_all(setup.home())?;
    std::fs::write(&events, "")?;
    let clock = Arc::new(TestClock::new("2026-11-26T09:00[Europe/Berlin]".parse()?));

    let error = setup
        .store(&clock)
        .read(&ReadScope::everything())
        .err()
        .ok_or("the read must fail")?;

    assert_eq!(error.to_string(), message("cannot read {path}", &events));
    Ok(())
}

#[test]
fn device_file_taken_by_directory_cannot_be_read() -> TestResult {
    let setup = Setup::new()?;
    let device_file = setup.machine().join("device.toml");
    std::fs::create_dir_all(&device_file)?;

    let error = device_id(&setup.machine(), &ids()?)
        .err()
        .ok_or("the device ID must fail")?;

    assert_eq!(
        error.to_string(),
        message("cannot read {path}", &device_file)
    );
    let cause = std::error::Error::source(&*error)
        .and_then(|cause| cause.downcast_ref::<std::io::Error>())
        .ok_or("the I/O error is the cause")?;
    assert_ne!(
        cause.kind(),
        std::io::ErrorKind::NotFound,
        "the read of the directory failed, not a later look for the file"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn read_only_machine_directory_takes_no_device_id() -> TestResult {
    use std::os::unix::fs::PermissionsExt as _;

    let setup = Setup::new()?;
    std::fs::create_dir_all(setup.machine())?;
    std::fs::set_permissions(setup.machine(), std::fs::Permissions::from_mode(0o555))?;

    let result = device_id(&setup.machine(), &ids()?);
    std::fs::set_permissions(setup.machine(), std::fs::Permissions::from_mode(0o755))?;

    let error = result.err().ok_or("the device ID must fail")?;
    assert_eq!(
        error.to_string(),
        message("cannot create a temporary file in {path}", &setup.machine())
    );
    assert_eq!(
        error.hint(),
        Some("check that the directory exists and that pace may write there")
    );
    Ok(())
}

#[test]
fn events_directory_skips_stray_files_and_hidden_directories() -> TestResult {
    let setup = Setup::new()?;
    let written = setup.tracker()?.run(&Command::Begin(Begin::default()))?;
    let events = setup.home().join("events");
    std::fs::write(events.join("README.md"), "notes of the user")?;
    std::fs::create_dir_all(events.join(".stversions"))?;
    let clock = Arc::new(TestClock::new("2026-11-26T10:00[Europe/Berlin]".parse()?));

    let read = setup.store(&clock).read(&ReadScope::everything())?;

    assert_eq!(read, written.recorded);
    Ok(())
}

#[test]
fn device_directory_skips_hidden_event_files() -> TestResult {
    let setup = Setup::new()?;
    let written = setup.tracker()?.run(&Command::Begin(Begin::default()))?;
    std::fs::write(
        setup.device_dir().join("2026/.2026-11.sync-conflict.toml"),
        "no TOML at all",
    )?;
    let clock = Arc::new(TestClock::new("2026-11-26T10:00[Europe/Berlin]".parse()?));

    let read = setup.store(&clock).read(&ReadScope::everything())?;

    assert_eq!(read, written.recorded);
    Ok(())
}
