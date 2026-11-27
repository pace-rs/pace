//! The rewrites of this device's own log: a new split, and the events of
//! forgotten entries taken out.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use jiff::ZonedDateTime;
use pace_core::{DeviceId, EntryId, EventKind, RecordedEvent};
use pace_error::PaceResult;

use crate::Split;
use crate::disk::{
    DEVICE_FILE, create_dir, event_files, io_error, lock, prepare, relative_name,
    remove_empty_dirs, write_atomically,
};
use crate::files::{FilesStore, Resplit, read_device_file, write_device_file};
use crate::format::{file_header, parse_events, serialize_events};
use crate::index::Index;

impl FilesStore {
    /// Rewrite this device's event files split by `split`, and record the
    /// split in its `device.toml`. Other devices' files stay as they are;
    /// pace reads each device with its own split.
    ///
    /// Each event is kept once, and every event file that `split` doesn't
    /// name is removed. So a split that pace stopped in the middle is
    /// finished by running it again, to the same split too. If the files
    /// already lie as `split` wants them, nothing is written.
    ///
    /// # Errors
    ///
    /// If a file can't be read, written or removed.
    pub fn resplit(&self, split: Split) -> PaceResult<Resplit> {
        let dir = self.device_dir(self.device);
        if !dir.exists() && split == self.split {
            return Ok(Resplit {
                from: split,
                to: split,
                files_before: 0,
                files_after: 0,
                events: 0,
            });
        }
        create_dir(&self.machine)?;
        let _lock = lock(&self.machine.join("lock"))?;
        let device_path = dir.join(DEVICE_FILE);
        let from = if device_path.is_file() {
            read_device_file(&device_path)?.split
        } else {
            self.split
        };
        let old_files = if dir.exists() {
            event_files(&dir)?
        } else {
            Vec::new()
        };
        let layout = Layout::read(&dir, &old_files, self.device, split)?;
        if from == split && layout.in_place {
            return Ok(Resplit {
                from,
                to: split,
                files_before: old_files.len(),
                files_after: old_files.len(),
                events: 0,
            });
        }

        // The new files first, then the split, then the old files: a stop
        // in between leaves events twice, which reading counts once and the
        // next resplit cleans.
        let mut written = Vec::new();
        for (relative, events) in &layout.by_file {
            let mut text = file_header();
            text.push('\n');
            text.push_str(&serialize_events(events)?);
            written.push(prepare(&dir.join(relative), &text)?);
        }
        let new_files: BTreeSet<PathBuf> = written.iter().map(|(_, path)| path.clone()).collect();
        for (temporary, path) in written {
            _ = temporary
                .persist(&path)
                .map_err(|error| io_error("write", &path, error.error))?;
        }
        let mut device_file = self.device_file(&dir, &self.clock.now())?;
        if device_file.split != split {
            device_file.split = split;
            write_device_file(&device_path, &device_file)?;
        }
        for path in old_files.iter().filter(|path| !new_files.contains(*path)) {
            std::fs::remove_file(path).map_err(|error| io_error("remove", path, error))?;
        }
        remove_empty_dirs(&dir)?;
        Ok(Resplit {
            from,
            to: split,
            files_before: old_files.len(),
            files_after: new_files.len(),
            events: layout.events,
        })
    }

    /// Remove the events of a forgotten entry from every file of this
    /// device's own log that holds them; the tombstone stays. Other
    /// devices' files are theirs to clean. The index skips the files whose
    /// events are about other moments.
    pub(super) fn forget_in_own_log(
        &self,
        dir: &Path,
        index: &Index,
        entry: EntryId,
        about: &str,
    ) -> PaceResult<()> {
        let about = about
            .parse::<ZonedDateTime>()
            .ok()
            .map(|about| about.timestamp().as_millisecond());
        for path in event_files(dir)? {
            let summary = index.files.get(&relative_name(&self.events_dir(), &path));
            let elsewhere = summary.zip(about).is_some_and(|(summary, about)| {
                summary
                    .abouts
                    .is_none_or(|(min, max)| about < min || max < about)
            });
            if !elsewhere {
                forget_in_file(&path, self.device, entry)?;
            }
        }
        Ok(())
    }
}

/// The events of a device's files as `split` files them.
struct Layout {
    /// Each event once, by the file `split` names for it, oldest first.
    by_file: BTreeMap<String, Vec<RecordedEvent>>,
    /// How many events there are.
    events: usize,
    /// Whether the files already lie so: each event once, in its file, and
    /// no other file.
    in_place: bool,
}

impl Layout {
    fn read(dir: &Path, files: &[PathBuf], device: DeviceId, split: Split) -> PaceResult<Self> {
        let mut by_file: BTreeMap<String, Vec<RecordedEvent>> = BTreeMap::new();
        let mut seen = BTreeSet::new();
        let mut in_place = true;
        for path in files {
            let text =
                std::fs::read_to_string(path).map_err(|error| io_error("read", path, error))?;
            let name = relative_name(dir, path);
            for recorded in parse_events(path, &text, device)? {
                if !seen.insert(recorded.event.id) {
                    in_place = false;
                    continue;
                }
                let target = split.file(&recorded.event.about);
                in_place &= target == name;
                by_file.entry(target).or_default().push(recorded);
            }
        }
        in_place &= by_file.len() == files.len();
        for events in by_file.values_mut() {
            events.sort_by_key(RecordedEvent::merge_key);
        }
        Ok(Self {
            by_file,
            events: seen.len(),
            in_place,
        })
    }
}

/// Remove the events of `entry` from the event file `path`, but its
/// tombstone; a file left without events is removed.
fn forget_in_file(path: &Path, device: DeviceId, entry: EntryId) -> PaceResult<()> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(io_error("read", path, error)),
    };
    let events = parse_events(path, &text, device)?;
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
        return std::fs::remove_file(path).map_err(|error| io_error("remove", path, error));
    }
    let mut text = file_header();
    text.push('\n');
    text.push_str(&serialize_events(&kept)?);
    write_atomically(path, &text)
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
        EventKind::BatchUndone { .. } => None,
    }
}
