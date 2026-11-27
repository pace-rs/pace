//! The rewrites of this device's own log: a new split, and the events of
//! forgotten entries taken out.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use pace_core::{DeviceId, EntryId, EventKind, RecordedEvent};
use pace_error::{ErrorKind, PaceError, PaceResult, Severity};

use crate::Split;
use crate::disk::{
    DEVICE_FILE, FileOp, create_dir, event_files, io_error, lock_file, prepare_temporary_file,
    relative_name, remove_empty_dirs, write_atomically,
};
use crate::files::parsed::Events;
use crate::files::{FilesStore, Resplit, read_device_info_file, write_device_info_file};
use crate::format::{file_header, parse_events, serialize_events};
use crate::index::Index;
use crate::machine::Cursors;

/// A tombstone: the forgotten entry, and the moment its events are filed
/// under, in milliseconds.
pub(super) type Tombstone = (EntryId, i64);

/// The tombstones in the files of `index`.
pub(super) fn index_tombstones(index: &Index) -> Vec<Tombstone> {
    index
        .files
        .values()
        .flat_map(|summary| summary.forgotten.iter().copied())
        .collect()
}

/// The tombstones among the events `recorded`.
pub(super) fn batch_tombstones(recorded: &[RecordedEvent]) -> Vec<Tombstone> {
    recorded
        .iter()
        .filter_map(|recorded| match recorded.event.kind {
            EventKind::EntryForgotten { entry } => {
                Some((entry, recorded.event.about.timestamp().as_millisecond()))
            }
            _ => None,
        })
        .collect()
}

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
        let _lock = lock_file(&self.machine.join("lock"))?;
        let device_path = dir.join(DEVICE_FILE);
        let from = if device_path.is_file() {
            read_device_info_file(&device_path)?.split
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
            written.push(prepare_temporary_file(&dir.join(relative), &text)?);
        }
        let new_files: BTreeSet<PathBuf> = written.iter().map(|(_, path)| path.clone()).collect();
        for (temporary, path) in written {
            _ = temporary
                .persist(&path)
                .map_err(|error| io_error(FileOp::Write, &path, error.error))?;
        }
        let mut device_info = self.device_info_file(&dir, &self.clock.now())?;
        if device_info.split != split {
            device_info.split = split;
            write_device_info_file(&device_path, &device_info)?;
        }
        for path in old_files.iter().filter(|path| !new_files.contains(*path)) {
            std::fs::remove_file(path).map_err(|error| io_error(FileOp::Remove, path, error))?;
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

    /// Remove the events of forgotten entries from this device's own files,
    /// for each of `tombstones` this machine hasn't applied yet, and mark it
    /// in `cursors`' `cleaned`; the caller decides when to save them. A cleaning that fails
    /// becomes one of the returned warnings and stays unmarked, so the next
    /// append tries it again.
    pub(super) fn clean_own_log(
        &self,
        dir: &Path,
        index: &Index,
        tombstones: &[Tombstone],
        cursors: &mut Cursors,
    ) -> Vec<PaceError> {
        let mut warnings = Vec::new();
        for &(entry, about) in tombstones {
            if cursors.cleaned.contains(&entry) {
                continue;
            }
            match self.forget_in_own_log(dir, index, entry, about) {
                Ok(()) => _ = cursors.cleaned.insert(entry),
                Err(error) => warnings.push(
                    *PaceError::with_source(
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
        warnings
    }

    /// The text of the event file `path` with the batch's `events` appended,
    /// where `before` is its text if it exists, and the events the file then
    /// holds, so that the append can put them into the index. If
    /// `forgotten` names entries, their events but the tombstones leave the
    /// file in the same write. The events of `before` come from this
    /// process's parsed files when the read before the batch parsed it. If
    /// `before` doesn't parse and the batch forgets nothing, the batch is
    /// appended as text without its events, as before the index took them:
    /// the next read names the mistake.
    pub(super) fn with_batch(
        &self,
        path: &Path,
        before: Option<&str>,
        events: &[RecordedEvent],
        forgotten: &BTreeSet<EntryId>,
    ) -> PaceResult<(String, Option<Events>)> {
        let Some(before) = before else {
            let text = format!("{}\n{}", file_header(), serialize_events(events)?);
            return Ok((text, Some(events.into())));
        };
        let parsed = self.parsed.events(path, before.to_owned(), |text| {
            parse_events(path, text, self.device)
        });
        let old = match parsed {
            Ok(old) => old,
            Err(_) if forgotten.is_empty() => {
                return Ok((format!("{before}\n{}", serialize_events(events)?), None));
            }
            Err(error) => return Err(error),
        };
        let mut held: Vec<RecordedEvent> = old
            .iter()
            .filter(|recorded| !erased(&recorded.event.kind, forgotten))
            .cloned()
            .collect();
        let unchanged = held.len() == old.len();
        held.extend_from_slice(events);
        let text = if unchanged {
            format!("{before}\n{}", serialize_events(events)?)
        } else {
            format!("{}\n{}", file_header(), serialize_events(&held)?)
        };
        Ok((text, Some(held.into())))
    }

    /// Remove the events of a forgotten entry from every file of this
    /// device's own log that holds them; the tombstone stays. Other
    /// devices' files are theirs to clean. The index skips the files whose
    /// events are about other moments than `about`, the moment in
    /// milliseconds that the entry's events are filed under.
    fn forget_in_own_log(
        &self,
        dir: &Path,
        index: &Index,
        entry: EntryId,
        about: i64,
    ) -> PaceResult<()> {
        for path in event_files(dir)? {
            let summary = index.files.get(&relative_name(&self.events_dir(), &path));
            let elsewhere = summary.is_some_and(|summary| {
                summary
                    .abouts
                    .is_none_or(|(min, max)| about < min || max < about)
            });
            if !elsewhere {
                self.forget_in_file(&path, &BTreeSet::from([entry]))?;
            }
        }
        Ok(())
    }
}

impl FilesStore {
    /// Remove the events of the entries `forgotten` from this device's
    /// event file `path`, but their tombstones; a file left without events
    /// is removed.
    fn forget_in_file(&self, path: &Path, forgotten: &BTreeSet<EntryId>) -> PaceResult<()> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(io_error(FileOp::Read, path, error)),
        };
        let events = self
            .parsed
            .events(path, text, |text| parse_events(path, text, self.device))?;
        let kept: Vec<RecordedEvent> = events
            .iter()
            .filter(|recorded| !erased(&recorded.event.kind, forgotten))
            .cloned()
            .collect();
        if kept.len() == events.len() {
            return Ok(());
        }
        if kept.is_empty() {
            return std::fs::remove_file(path)
                .map_err(|error| io_error(FileOp::Remove, path, error));
        }
        let text = format!("{}\n{}", file_header(), serialize_events(&kept)?);
        write_atomically(path, &text)?;
        self.parsed.remember(path.to_path_buf(), text, kept.into());
        Ok(())
    }
}

/// Whether forgetting the entries `forgotten` takes an event of `kind` out
/// of the log: every event of theirs but the tombstone.
fn erased(kind: &EventKind, forgotten: &BTreeSet<EntryId>) -> bool {
    !matches!(kind, EventKind::EntryForgotten { .. })
        && kind.entry().is_some_and(|entry| forgotten.contains(&entry))
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
            let text = std::fs::read_to_string(path)
                .map_err(|error| io_error(FileOp::Read, path, error))?;
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pace_core::{
        Catalog, Clock, Command, Policy, ReadScope, Rules, State,
        command::{Begin, Forget},
        testing::SequentialIds,
        testing::TestClock,
    };

    use super::*;
    use crate::{EventStore, Tracker, machine::Cursors, renames::persist, testing::scratch_dir};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// A laptop and a desktop that share `PACE_HOME`, each with its own
    /// machine directory and monthly files.
    struct Pair {
        dir: tempfile::TempDir,
        clock: Arc<TestClock>,
        laptop: DeviceId,
        desktop: DeviceId,
    }

    impl Pair {
        fn new() -> Result<Self, Box<dyn std::error::Error>> {
            Ok(Self {
                dir: scratch_dir()?,
                clock: Arc::new(TestClock::new("2026-11-26T09:00[Europe/Berlin]".parse()?)),
                laptop: "0193a8d0-000d-7000-8000-000000000001".parse()?,
                desktop: "0193a8d0-000d-7000-8000-000000000002".parse()?,
            })
        }

        fn machine(&self, device: DeviceId) -> PathBuf {
            self.dir.path().join("machine").join(device.to_string())
        }

        fn store(&self, device: DeviceId) -> FilesStore {
            FilesStore::new(
                self.dir.path().join("home"),
                self.machine(device),
                device,
                device.to_string(),
                self.clock.clone(),
                Split::Month,
            )
        }

        fn tracker(&self, device: DeviceId, seed: u16) -> Tracker<FilesStore> {
            Tracker::new(
                self.store(device),
                self.clock.clone(),
                Arc::new(SequentialIds::starting_at(&self.clock.now(), seed)),
                Policy::default(),
                Catalog::default(),
                device,
            )
        }

        /// The entry the laptop begins at 09:00, "Write offer".
        fn write_offer(&self) -> Result<EntryId, Box<dyn std::error::Error>> {
            let tracker = self.tracker(self.laptop, 1);
            _ = tracker.run(&Command::Begin(Begin {
                note: Some("Write offer".parse()?),
                ..Begin::default()
            }))?;
            let state = tracker.state(&pace_time::TimeRange::everything())?;
            Ok(state.open().next().ok_or("the running entry")?.id)
        }

        /// The texts the laptop renames over its event files when it
        /// appends the batch of `command`, in the order of the renames.
        fn renamed_texts(
            &self,
            command: &Command,
        ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
            let store = self.store(self.laptop);
            let marker = store.marker()?;
            let now = self.clock.now();
            let batch = Rules {
                state: &State::from_events(&store.read(&ReadScope::everything())?),
                now: &now,
                device: self.laptop,
                ids: &SequentialIds::starting_at(&now, 3),
                policy: &Policy::default(),
                catalog: &Catalog::default(),
            }
            .decide(command)?;
            let mut texts = Vec::new();
            _ = store.append_with(batch, marker, |temporary, path| {
                texts.push(std::fs::read_to_string(temporary.path())?);
                persist(temporary, path)
            })?;
            Ok(texts)
        }
    }

    #[test]
    fn forget_takes_entry_out_of_its_file_in_same_rename() -> TestResult {
        let pair = Pair::new()?;
        let entry = pair.write_offer()?;
        pair.clock.set("2026-11-26T09:30[Europe/Berlin]".parse()?);

        let texts = pair.renamed_texts(&Command::Forget(Forget {
            entry,
            change_billed: false,
        }))?;

        let [text] = texts.as_slice() else {
            return Err(format!("one file renamed, not {}", texts.len()).into());
        };
        assert!(text.contains("type = \"entry.forgotten\""), "{text}");
        assert!(!text.contains("type = \"entry.begun\""), "{text}");
        Ok(())
    }

    #[test]
    fn tombstone_from_other_device_leaves_own_file_before_batch() -> TestResult {
        let pair = Pair::new()?;
        let entry = pair.write_offer()?;
        pair.clock.set("2026-11-26T09:30[Europe/Berlin]".parse()?);
        _ = pair.tracker(pair.desktop, 2).run(&Command::Forget(Forget {
            entry,
            change_billed: false,
        }))?;
        pair.clock.set("2026-11-26T09:45[Europe/Berlin]".parse()?);

        let texts = pair.renamed_texts(&Command::Begin(Begin::default()))?;

        let [text] = texts.as_slice() else {
            return Err(format!("one file renamed, not {}", texts.len()).into());
        };
        assert!(!text.contains(&entry.to_string()), "{text}");
        Ok(())
    }

    #[test]
    fn forgotten_entry_is_marked_cleaned_by_next_write() -> TestResult {
        let pair = Pair::new()?;
        let entry = pair.write_offer()?;
        let tracker = pair.tracker(pair.laptop, 2);
        pair.clock.set("2026-11-26T09:30[Europe/Berlin]".parse()?);
        _ = tracker.run(&Command::Forget(Forget {
            entry,
            change_billed: false,
        }))?;
        pair.clock.set("2026-11-26T09:45[Europe/Berlin]".parse()?);

        _ = tracker.run(&Command::Begin(Begin::default()))?;

        assert!(
            Cursors::load(&pair.machine(pair.laptop))?
                .cleaned
                .contains(&entry)
        );
        Ok(())
    }
}
