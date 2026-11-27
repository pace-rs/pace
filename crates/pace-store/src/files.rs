//! The `files` backend: TOML event files in `PACE_HOME`.
//!
//! ```text
//! PACE_HOME/events/<device-id>/device.toml        name, created, split
//! PACE_HOME/events/<device-id>/2026/2026-11.toml  events filed under November
//! ```
//!
//! Each device only appends to its own directory, so syncing `PACE_HOME`
//! never conflicts. Appends happen under the lock file in the machine
//! directory, and only if the event files are as they were when the
//! command read them; every changed file is written to a temporary file
//! next to it and then renamed over it.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, hash_map::DefaultHasher},
    future::Future,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::SystemTime,
};

use jiff::ZonedDateTime;
use pace_core::{
    Appended, Batch, ChangeMarker, Clock, Device, DeviceId, EntryId, EventKind, EventStore, Hlc,
    RecordedEvent,
};
use pace_error::{ErrorKind, PaceError, PaceResult, Severity};
use pace_time::TimeRange;
use serde::{Deserialize, Serialize};

use crate::disk::{
    DEVICE_FILE, check_version, create_dir, event_files, io_error, lock, prepare, relative_name,
    write_atomically,
};
use crate::format::{file_header, parse_events, serialize_events};
use crate::machine::clone_detected;
use crate::{
    Split,
    index::{FileSummary, INDEX_VERSION, Index, select, summarize},
    machine::Cursors,
};

mod own_log;

/// The files backend for one device.
pub struct FilesStore {
    home: PathBuf,
    machine: PathBuf,
    device: DeviceId,
    clock: Arc<dyn Clock>,
    split: Split,
    problems: Mutex<Vec<PaceError>>,
}

/// What [`FilesStore::resplit`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resplit {
    /// The split the device's files had.
    pub from: Split,
    /// The split they have now.
    pub to: Split,
    /// How many event files the device had.
    pub files_before: usize,
    /// How many it has now.
    pub files_after: usize,
    /// How many events were rewritten; none if the files already lay so.
    pub events: usize,
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

/// The version of a device's `device.toml` this pace reads and writes.
const DEVICE_VERSION: u32 = 1;

const INDEX_FILE: &str = "index.json";

/// An event file found while refreshing the index: its device and path,
/// and its events if the refresh parsed them.
struct FoundFile {
    device: DeviceId,
    path: PathBuf,
    events: Option<Vec<RecordedEvent>>,
}

/// The event files found while refreshing the index, by their name in it.
type Found = HashMap<String, FoundFile>;

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
            problems: Mutex::default(),
        }
    }

    fn events_dir(&self) -> PathBuf {
        self.home.join("events")
    }

    fn device_dir(&self, device: DeviceId) -> PathBuf {
        self.events_dir().join(device.to_string())
    }

    fn append_now(&self, batch: Batch, read: ChangeMarker) -> PaceResult<Appended> {
        let now = self.clock.now();
        create_dir(&self.machine)?;
        let _lock = lock(&self.machine.join("lock"))?;
        // Under the lock, so no other process writes between this check and
        // the append. The device directory comes after it: a new directory
        // changes the marker.
        if self.marker_now()? != read {
            return Ok(Appended::Stale);
        }
        let dir = self.device_dir(self.device);
        create_dir(&dir)?;

        let (index, _) = self.refreshed_index()?;
        let index_path = self.machine.join(INDEX_FILE);
        let own_prefix = format!("{}/", self.device);
        let own_last = max_hlc(
            index
                .files
                .iter()
                .filter(|(name, _)| name.starts_with(&own_prefix))
                .map(|(_, summary)| summary),
            &index_path,
        )?;
        let mut cursors = Cursors::load(&self.machine)?;
        if let (Some(own_last), Some(remembered)) = (own_last, cursors.last_written)
            && own_last > remembered
        {
            return Err(clone_detected(&self.machine, own_last, remembered));
        }

        let device_file = self.device_file(&dir, &now)?;
        // After the latest event of every log, and after the cursor, which
        // may be ahead of the own log (below).
        let mut last = max_hlc(index.files.values(), &index_path)?.max(cursors.last_written);
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
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => file_header(),
                Err(error) => return Err(io_error("read", &path, error)),
            };
            text.push('\n');
            text.push_str(&serialize_events(&events)?);
            written.push(prepare(&path, &text)?);
        }
        // The cursor goes ahead of the log, before the renames: a cursor
        // ahead of the own log passes the clone check, a log ahead of the
        // cursor doesn't. So a failure or a stop from here on never locks
        // the device out.
        cursors.last_written = last;
        cursors.save(&self.machine)?;
        // All files are written before any is renamed, so a failure leaves
        // the log as it was; only a crash between the renames of a batch
        // that spans periods can leave part of it.
        for (temporary, path) in written {
            _ = temporary
                .persist(&path)
                .map_err(|error| io_error("write", &path, error.error))?;
        }
        // The batch is in the log: what follows can fail without failing it.
        self.clean_own_log(&dir, &index, &recorded, cursors);
        Ok(Appended::Recorded(recorded))
    }

    /// Remove the events of forgotten entries from this device's own files,
    /// for each tombstone this machine hasn't applied yet. A cleaning that
    /// fails becomes a [problem](Self::take_problems) and stays out of the
    /// cursors' `cleaned`, so the next append tries it again.
    fn clean_own_log(
        &self,
        dir: &Path,
        index: &Index,
        recorded: &[RecordedEvent],
        mut cursors: Cursors,
    ) {
        let tombstones: Vec<(EntryId, i64)> = index
            .files
            .values()
            .flat_map(|summary| summary.forgotten.iter().copied())
            .chain(
                recorded
                    .iter()
                    .filter_map(|recorded| match recorded.event.kind {
                        EventKind::EntryForgotten { entry } => {
                            Some((entry, recorded.event.about.timestamp().as_millisecond()))
                        }
                        _ => None,
                    }),
            )
            .collect();
        let mut cleaned = false;
        for (entry, about) in tombstones {
            if cursors.cleaned.contains(&entry) {
                continue;
            }
            match self.forget_in_own_log(dir, index, entry, about) {
                Ok(()) => {
                    cleaned |= cursors.cleaned.insert(entry);
                }
                Err(error) => self.report(
                    PaceError::with_source(
                        ErrorKind::InputOutput,
                        "pace recorded the command but kept the events of the forgotten entry {entry} in this device's files",
                        error,
                    )
                    .attach_context("entry", entry.to_string())
                    .attach_hint("pace removes them with the next command that writes")
                    .attach_severity(Severity::Warning),
                ),
            }
        }
        if cleaned && let Err(error) = cursors.save(&self.machine) {
            self.report(error.attach_severity(Severity::Warning));
        }
    }

    /// What went wrong after an append was recorded, such as a cleaning
    /// that failed; taking them empties the list. The append succeeded, so
    /// the caller shows them as warnings.
    pub fn take_problems(&self) -> Vec<PaceError> {
        std::mem::take(
            &mut *self
                .problems
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }

    fn report(&self, problem: Box<PaceError>) {
        self.problems
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(*problem);
    }

    /// This device's `device.toml`, written with its split on first use.
    fn device_file(&self, dir: &Path, now: &ZonedDateTime) -> PaceResult<DeviceFile> {
        let path = dir.join(DEVICE_FILE);
        if path.is_file() {
            return read_device_file(&path);
        }
        let name = gethostname::gethostname().to_string_lossy().into_owned();
        let file = DeviceFile {
            version: DEVICE_VERSION,
            name: if name.is_empty() {
                "device".to_owned()
            } else {
                name
            },
            created: now.clone(),
            split: self.split,
        };
        write_device_file(&path, &file)?;
        Ok(file)
    }

    /// The events of the files `range` needs, through the index; an event
    /// in two files (left by an interrupted resplit) counts once.
    fn read_now(&self, range: &TimeRange) -> PaceResult<Vec<RecordedEvent>> {
        let (index, mut found) = self.refreshed_index()?;
        let mut events = Vec::new();
        let mut seen = BTreeSet::new();
        for name in select(&index.files, range) {
            // The refreshed index names only the files it found.
            let Some(file) = found.remove(&name) else {
                continue;
            };
            let file_events = match file.events {
                Some(file_events) => file_events,
                None => read_events(&file.path, file.device)?,
            };
            events.extend(
                file_events
                    .into_iter()
                    .filter(|recorded| seen.insert((recorded.device, recorded.event.id))),
            );
        }
        Ok(events)
    }

    /// The index, brought up to date with the files: changed and new files
    /// are parsed and summarized, gone files dropped. Returns the files it
    /// found, with the events of those it parsed.
    fn refreshed_index(&self) -> PaceResult<(Index, Found)> {
        let index_path = self.machine.join(INDEX_FILE);
        let mut index: Index = std::fs::read_to_string(&index_path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .filter(|index: &Index| index.version == INDEX_VERSION)
            .unwrap_or_else(|| Index {
                version: INDEX_VERSION,
                files: BTreeMap::new(),
            });
        let mut found = Found::new();
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
                let unchanged = index.files.get(&name).is_some_and(|summary| {
                    summary.len == metadata.len() && summary.modified == modified
                });
                let events = if unchanged {
                    None
                } else {
                    let events = read_events(&path, device)?;
                    let mut summary = summarize(&events);
                    summary.len = metadata.len();
                    summary.modified = modified;
                    _ = index.files.insert(name.clone(), summary);
                    changed = true;
                    Some(events)
                };
                _ = found.insert(
                    name,
                    FoundFile {
                        device,
                        path,
                        events,
                    },
                );
            }
        }
        let before = index.files.len();
        index.files.retain(|name, _| found.contains_key(name));
        changed |= index.files.len() != before;

        // The index is a cache: if it can't be saved, this run works with
        // the one in memory and the next run parses the files again. So a
        // read works in a machine directory that pace may not write.
        if changed {
            _ = save_index(&self.machine, &index_path, &index);
        }
        Ok((index, found))
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
    fn append(
        &self,
        batch: Batch,
        read: ChangeMarker,
    ) -> impl Future<Output = PaceResult<Appended>> + Send {
        std::future::ready(self.append_now(batch, read))
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

fn save_index(machine: &Path, path: &Path, index: &Index) -> PaceResult<()> {
    create_dir(machine)?;
    let text = serde_json::to_string(index).map_err(|error| {
        PaceError::with_source(ErrorKind::Internal, "cannot write the index", error).ask_report()
    })?;
    write_atomically(path, &text)
}

fn write_device_file(path: &Path, file: &DeviceFile) -> PaceResult<()> {
    let text = toml::to_string(file).map_err(|error| {
        PaceError::with_source(ErrorKind::Internal, "cannot write device.toml", error).ask_report()
    })?;
    write_atomically(path, &text)
}

/// The events of the event file `path` of `device`.
fn read_events(path: &Path, device: DeviceId) -> PaceResult<Vec<RecordedEvent>> {
    let text = std::fs::read_to_string(path).map_err(|error| io_error("read", path, error))?;
    parse_events(path, &text, device)
}

fn read_device_file(path: &Path) -> PaceResult<DeviceFile> {
    let text = std::fs::read_to_string(path).map_err(|error| io_error("read", path, error))?;
    check_version(
        path,
        &text,
        DEVICE_VERSION,
        &format!("write `version = {DEVICE_VERSION}`"),
    )?;
    toml::from_str(&text).map_err(|error| {
        PaceError::with_source(
            ErrorKind::InputOutput,
            "{path} is no valid device file",
            error,
        )
        .attach_context("path", path.display().to_string())
        .attach_hint("it holds `version`, `name`, `created` and `split`; fix the file by hand")
    })
}

/// The latest HLC of `summaries`, from the index at `index_path`.
fn max_hlc<'index>(
    summaries: impl Iterator<Item = &'index FileSummary>,
    index_path: &Path,
) -> PaceResult<Option<Hlc>> {
    let mut latest = None;
    for summary in summaries {
        latest = latest.max(summary.max_hlc(index_path)?);
    }
    Ok(latest)
}
