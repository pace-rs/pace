//! The `files` backend: TOML event files in `PACE_HOME`.
//!
//! ```text
//! PACE_HOME/events/<device-id>/device.toml        name, created, split
//! PACE_HOME/events/<device-id>/2026/2026-11.toml  events filed under November
//! ```
//!
//! Each device only appends to its own directory, so syncing `PACE_HOME`
//! never conflicts. Appends happen under the lock file in the machine
//! directory; every changed file is written to a temporary file next to it
//! and then renamed over it.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, hash_map::DefaultHasher},
    fs::{File, OpenOptions},
    future::Future,
    hash::{Hash, Hasher},
    io::Write as _,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use jiff::ZonedDateTime;
use pace_core::{
    Batch, ChangeMarker, Clock, Device, DeviceId, EntryId, EventKind, EventStore, Hlc,
    RecordedEvent,
};
use pace_error::{ErrorKind, PaceError, PaceResult};
use pace_time::TimeRange;
use serde::{Deserialize, Serialize};

use crate::{
    Split, format,
    index::{INDEX_VERSION, Index, select, summarize},
    machine::{self, Cursors},
};

/// The files backend for one device.
pub struct FilesStore {
    home: PathBuf,
    machine: PathBuf,
    device: DeviceId,
    clock: Arc<dyn Clock>,
    split: Split,
}

/// `events/<device>/device.toml`
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeviceFile {
    version: u32,
    name: String,
    created: ZonedDateTime,
    split: Split,
}

const DEVICE_FILE: &str = "device.toml";
const INDEX_FILE: &str = "index.json";

/// Events parsed while refreshing the index, by file.
type Parsed = HashMap<String, Vec<RecordedEvent>>;

impl FilesStore {
    /// The store of `device`, with its data in `home` (`PACE_HOME`) and its
    /// lock in the machine directory `machine`. `split` applies when the
    /// device writes its first event; afterwards its `device.toml` decides.
    /// Nothing is created before the first append.
    pub fn new(
        home: PathBuf,
        machine: PathBuf,
        device: DeviceId,
        clock: Arc<dyn Clock>,
        split: Split,
    ) -> Self {
        Self {
            home,
            machine,
            device,
            clock,
            split,
        }
    }

    fn events_dir(&self) -> PathBuf {
        self.home.join("events")
    }

    fn device_dir(&self, device: DeviceId) -> PathBuf {
        self.events_dir().join(device.to_string())
    }

    fn append_now(&self, batch: Batch) -> PaceResult<Vec<RecordedEvent>> {
        let now = self.clock.now();
        let dir = self.device_dir(self.device);
        create_dir(&dir)?;
        create_dir(&self.machine)?;
        let _lock = lock(&self.machine.join("lock"))?;

        let (index, _) = self.refreshed_index()?;
        let own_prefix = format!("{}/", self.device);
        let own_last = index
            .files
            .iter()
            .filter(|(name, _)| name.starts_with(&own_prefix))
            .filter_map(|(_, summary)| summary.max_hlc())
            .max();
        let mut cursors = Cursors::load(&self.machine)?;
        if let (Some(own_last), Some(remembered)) = (own_last, cursors.last_written)
            && own_last > remembered
        {
            return Err(machine::clone_detected(&self.machine));
        }

        let device_file = self.device_file(&dir, &now)?;
        let mut last = index
            .files
            .values()
            .filter_map(crate::index::FileSummary::max_hlc)
            .max();
        let recorded: Vec<RecordedEvent> = batch
            .events
            .into_iter()
            .map(|event| {
                let hlc = Hlc::tick(last, now.timestamp());
                last = Some(hlc);
                RecordedEvent {
                    device: self.device,
                    batch: batch.id,
                    hlc,
                    event,
                }
            })
            .collect();

        let mut by_file: BTreeMap<String, Vec<RecordedEvent>> = BTreeMap::new();
        for event in &recorded {
            by_file
                .entry(device_file.split.file(&event.event.about))
                .or_default()
                .push(event.clone());
        }
        let mut written = Vec::new();
        for (relative, events) in by_file {
            let path = dir.join(relative);
            let mut text = match std::fs::read_to_string(&path) {
                Ok(text) => text,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => format::header(),
                Err(error) => return Err(io_error("read", &path, error)),
            };
            text.push('\n');
            text.push_str(&format::serialize(&events)?);
            written.push(prepare(&path, &text)?);
        }
        // All files are written before any is renamed, so a failure leaves
        // the log as it was; only a crash between the renames of a batch
        // that spans periods can leave part of it.
        for (temporary, path) in written {
            _ = temporary
                .persist(&path)
                .map_err(|error| io_error("write", &path, error.error))?;
        }
        cursors.last_written = recorded
            .last()
            .map(|last| last.hlc)
            .or(cursors.last_written);

        let tombstones: Vec<(EntryId, String)> = index
            .files
            .values()
            .flat_map(|summary| summary.forgotten.iter().cloned())
            .chain(
                recorded
                    .iter()
                    .filter_map(|recorded| match recorded.event.kind {
                        EventKind::EntryForgotten { entry } => {
                            Some((entry, recorded.event.about.to_string()))
                        }
                        _ => None,
                    }),
            )
            .collect();
        for (entry, about) in tombstones {
            // On an error, the cursors aren't saved, so the cleaning is retried.
            if cursors.cleaned.insert(entry) {
                self.forget_in_own_log(&dir, device_file.split, entry, &about)?;
            }
        }
        cursors.save(&self.machine)?;
        Ok(recorded)
    }

    /// Remove the events of a forgotten entry from this device's own file of
    /// the entry's period; the tombstone stays. Other devices' files are
    /// theirs to clean.
    fn forget_in_own_log(
        &self,
        dir: &Path,
        split: Split,
        entry: EntryId,
        about: &str,
    ) -> PaceResult<()> {
        let Ok(about) = about.parse::<ZonedDateTime>() else {
            return Ok(());
        };
        let path = dir.join(split.file(&about));
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(io_error("read", &path, error)),
        };
        let events = format::parse(&path, &text, self.device)?;
        let kept: Vec<RecordedEvent> = events
            .iter()
            .filter(|recorded| {
                matches!(recorded.event.kind, EventKind::EntryForgotten { .. })
                    || mentioned_entry(&recorded.event.kind) != Some(entry)
            })
            .cloned()
            .collect();
        if kept.len() == events.len() {
            return Ok(());
        }
        if kept.is_empty() {
            return std::fs::remove_file(&path).map_err(|error| io_error("remove", &path, error));
        }
        let mut text = format::header();
        text.push('\n');
        text.push_str(&format::serialize(&kept)?);
        write_atomically(&path, &text)
    }

    /// This device's `device.toml`, written with its split on first use.
    fn device_file(&self, dir: &Path, now: &ZonedDateTime) -> PaceResult<DeviceFile> {
        let path = dir.join(DEVICE_FILE);
        if path.is_file() {
            return read_device_file(&path);
        }
        let name = gethostname::gethostname().to_string_lossy().into_owned();
        let file = DeviceFile {
            version: 1,
            name: if name.is_empty() {
                "device".to_owned()
            } else {
                name
            },
            created: now.clone(),
            split: self.split,
        };
        let text = toml::to_string(&file).map_err(|error| {
            PaceError::with_source(ErrorKind::Internal, "cannot write device.toml", error)
                .ask_report()
        })?;
        write_atomically(&path, &text)?;
        Ok(file)
    }

    /// The events of the files `range` needs, through the index.
    fn read_now(&self, range: &TimeRange) -> PaceResult<Vec<RecordedEvent>> {
        let (index, mut parsed) = self.refreshed_index()?;
        let mut events = Vec::new();
        for name in select(&index.files, range) {
            match parsed.remove(&name) {
                Some(file_events) => events.extend(file_events),
                None => events.extend(self.parse_file(&name)?),
            }
        }
        Ok(events)
    }

    fn parse_file(&self, name: &str) -> PaceResult<Vec<RecordedEvent>> {
        let path = self.events_dir().join(name);
        let device = name
            .split('/')
            .next()
            .and_then(|device| device.parse::<DeviceId>().ok())
            .ok_or_else(|| {
                PaceError::new(
                    ErrorKind::Internal,
                    "the index names {name}, outside a device directory",
                )
                .attach_context("name", name)
                .ask_report()
            })?;
        let text =
            std::fs::read_to_string(&path).map_err(|error| io_error("read", &path, error))?;
        format::parse(&path, &text, device)
    }

    /// The index, brought up to date with the files: changed and new files
    /// are parsed and summarized, gone files dropped. Returns what it parsed.
    fn refreshed_index(&self) -> PaceResult<(Index, Parsed)> {
        let index_path = self.machine.join(INDEX_FILE);
        let mut index: Index = std::fs::read_to_string(&index_path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .filter(|index: &Index| index.version == INDEX_VERSION)
            .unwrap_or_else(|| Index {
                version: INDEX_VERSION,
                files: BTreeMap::new(),
            });
        let mut parsed = Parsed::new();
        let mut seen = BTreeSet::new();
        let mut changed = false;

        for (device, dir) in self.device_dirs()? {
            for path in event_files(&dir)? {
                let name = relative_name(&self.events_dir(), &path);
                let metadata =
                    std::fs::metadata(&path).map_err(|error| io_error("read", &path, error))?;
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|since| since.as_nanos());
                _ = seen.insert(name.clone());
                if index.files.get(&name).is_some_and(|summary| {
                    summary.len == metadata.len() && summary.modified == modified
                }) {
                    continue;
                }
                let text = std::fs::read_to_string(&path)
                    .map_err(|error| io_error("read", &path, error))?;
                let events = format::parse(&path, &text, device)?;
                let mut summary = summarize(&events);
                summary.len = metadata.len();
                summary.modified = modified;
                _ = index.files.insert(name.clone(), summary);
                _ = parsed.insert(name, events);
                changed = true;
            }
        }
        let before = index.files.len();
        index.files.retain(|name, _| seen.contains(name));
        changed |= index.files.len() != before;

        if changed {
            create_dir(&self.machine)?;
            let text = serde_json::to_string(&index).map_err(|error| {
                PaceError::with_source(ErrorKind::Internal, "cannot write the index", error)
                    .ask_report()
            })?;
            write_atomically(&index_path, &text)?;
        }
        Ok((index, parsed))
    }

    /// The directories under `events/`, by device.
    fn device_dirs(&self) -> PaceResult<Vec<(DeviceId, PathBuf)>> {
        let dir = self.events_dir();
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(io_error("read", &dir, error)),
        };
        let mut dirs = Vec::new();
        for entry in entries {
            let path = entry.map_err(|error| io_error("read", &dir, error))?.path();
            let name = path
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            if name.starts_with('.') || !path.is_dir() {
                continue;
            }
            let device = name.parse::<DeviceId>().map_err(|_| {
                PaceError::new(ErrorKind::InputOutput, "{path} is no device directory")
                    .attach_context("path", path.display().to_string())
                    .attach_hint("`events/` holds one directory per device, named by its ID; move other files elsewhere")
            })?;
            dirs.push((device, path));
        }
        dirs.sort();
        Ok(dirs)
    }

    fn devices_now(&self) -> PaceResult<Vec<Device>> {
        self.device_dirs()?
            .into_iter()
            .filter(|(_, dir)| dir.join(DEVICE_FILE).is_file())
            .map(|(id, dir)| {
                let file = read_device_file(&dir.join(DEVICE_FILE))?;
                Ok(Device {
                    id,
                    name: file.name,
                    created: file.created,
                })
            })
            .collect()
    }

    fn marker_now(&self) -> PaceResult<ChangeMarker> {
        let mut hasher = DefaultHasher::new();
        for (device, dir) in self.device_dirs()? {
            device.hash(&mut hasher);
            for path in event_files(&dir)? {
                let metadata =
                    std::fs::metadata(&path).map_err(|error| io_error("read", &path, error))?;
                path.hash(&mut hasher);
                metadata.len().hash(&mut hasher);
                metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .hash(&mut hasher);
            }
        }
        Ok(ChangeMarker::new(hasher.finish()))
    }
}

// The files backend works synchronously, so its futures are ready at once.
impl EventStore for FilesStore {
    fn append(&self, batch: Batch) -> impl Future<Output = PaceResult<Vec<RecordedEvent>>> + Send {
        std::future::ready(self.append_now(batch))
    }

    fn read(
        &self,
        range: &TimeRange,
    ) -> impl Future<Output = PaceResult<Vec<RecordedEvent>>> + Send {
        std::future::ready(self.read_now(range))
    }

    fn devices(&self) -> impl Future<Output = PaceResult<Vec<Device>>> + Send {
        std::future::ready(self.devices_now())
    }

    fn marker(&self) -> impl Future<Output = PaceResult<ChangeMarker>> + Send {
        std::future::ready(self.marker_now())
    }
}

fn read_device_file(path: &Path) -> PaceResult<DeviceFile> {
    let text = std::fs::read_to_string(path).map_err(|error| io_error("read", path, error))?;
    toml::from_str(&text).map_err(|error| {
        PaceError::with_source(
            ErrorKind::InputOutput,
            "{path} is no valid device file",
            error,
        )
        .attach_context("path", path.display().to_string())
        .attach_hint(
            "it holds `version`, `name`, `created` and `split`; `pace doctor` will repair it",
        )
    })
}

/// The event files in a device directory, oldest period first.
fn event_files(dir: &Path) -> PaceResult<Vec<PathBuf>> {
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

fn create_dir(dir: &Path) -> PaceResult<()> {
    std::fs::create_dir_all(dir).map_err(|error| io_error("create", dir, error))
}

/// The lock file, locked exclusively until the returned file is dropped.
fn lock(path: &Path) -> PaceResult<File> {
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
fn relative_name(events: &Path, path: &Path) -> String {
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
fn prepare(path: &Path, text: &str) -> PaceResult<(tempfile::NamedTempFile, PathBuf)> {
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

fn io_error(verb: &str, path: &Path, error: std::io::Error) -> Box<PaceError> {
    PaceError::with_source(ErrorKind::InputOutput, "cannot {verb} {path}", error)
        .attach_context("verb", verb)
        .attach_context("path", path.display().to_string())
        .attach_hint("check that the path exists and that pace may write there")
}

/// The entry an event is about, if any.
const fn mentioned_entry(kind: &EventKind) -> Option<EntryId> {
    match kind {
        EventKind::EntryBegun { entry, .. }
        | EventKind::EntryHeld { entry, .. }
        | EventKind::EntryPaused { entry, .. }
        | EventKind::EntryResumed { entry }
        | EventKind::EntryEnded { entry }
        | EventKind::CatchNoted { entry, .. }
        | EventKind::EntryLogged { entry, .. }
        | EventKind::EntryAdjusted { entry, .. }
        | EventKind::EntryForgotten { entry } => Some(*entry),
        _ => None,
    }
}
