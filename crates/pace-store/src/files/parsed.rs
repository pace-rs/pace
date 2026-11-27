//! The events of the event files this process parsed, by file and text.
//!
//! One command reads the current month's file more than once: the CLI for
//! `--last`, the core for its range, the clean-up of a forgotten entry. The
//! cache parses each text of a file once. It keys on the whole text, not on
//! the file's length and modification time, so a file that changed under
//! the same stamp is parsed again; and it lives only as long as the
//! process, so it is no second copy of the log on disk.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, PoisonError},
};

use pace_core::RecordedEvent;
use pace_error::PaceResult;

/// The events of a file's text, shared by the readers of one process.
pub(super) type Events = Arc<[RecordedEvent]>;

/// The parsed event files of this process.
#[derive(Default)]
pub(super) struct Parsed {
    files: Mutex<HashMap<PathBuf, ParsedFile>>,
}

/// Names the files, not their texts and events, which are long.
impl std::fmt::Debug for Parsed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut paths: Vec<PathBuf> = self.lock().keys().cloned().collect();
        paths.sort();
        formatter
            .debug_struct("Parsed")
            .field("files", &paths)
            .finish()
    }
}

/// The text a file had when it was parsed, and its events.
struct ParsedFile {
    text: String,
    events: Events,
}

impl Parsed {
    /// The events of `path` with the content `text`: those parsed before,
    /// if this process parsed this text of the file, else what `parse`
    /// gives, which the cache then keeps.
    pub(super) fn events(
        &self,
        path: &Path,
        text: String,
        parse: impl FnOnce(&str) -> PaceResult<Vec<RecordedEvent>>,
    ) -> PaceResult<Events> {
        if let Some(events) = self.cached(path, &text) {
            return Ok(events);
        }
        let events: Events = parse(&text)?.into();
        self.remember(path.to_path_buf(), text, events.clone());
        Ok(events)
    }

    /// The events of `path` with the content `text`, if this process
    /// parsed that text.
    fn cached(&self, path: &Path, text: &str) -> Option<Events> {
        self.lock()
            .get(path)
            .filter(|file| file.text == text)
            .map(|file| file.events.clone())
    }

    /// Keep `events` as the events of `path` with the content `text`, which
    /// this process wrote.
    pub(super) fn remember(&self, path: PathBuf, text: String, events: Events) {
        _ = self.lock().insert(path, ParsedFile { text, events });
    }

    /// The map; a panic while another thread held it leaves no half entry,
    /// so a poisoned lock is still good.
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<PathBuf, ParsedFile>> {
        self.files.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    /// A parse that counts its calls and finds no events.
    fn counting(calls: &Cell<usize>) -> impl FnOnce(&str) -> PaceResult<Vec<RecordedEvent>> {
        move |_| {
            calls.set(calls.get() + 1);
            Ok(Vec::new())
        }
    }

    #[test]
    fn same_text_of_file_is_parsed_once() {
        let parsed = Parsed::default();
        let calls = Cell::new(0);

        _ = parsed.events(Path::new("2026-11.toml"), "a".to_owned(), counting(&calls));
        _ = parsed.events(Path::new("2026-11.toml"), "a".to_owned(), counting(&calls));

        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn changed_text_is_parsed_again() {
        let parsed = Parsed::default();
        let calls = Cell::new(0);

        _ = parsed.events(Path::new("2026-11.toml"), "a".to_owned(), counting(&calls));
        _ = parsed.events(Path::new("2026-11.toml"), "b".to_owned(), counting(&calls));

        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn same_text_in_other_file_is_parsed_for_it() {
        let parsed = Parsed::default();
        let calls = Cell::new(0);

        _ = parsed.events(Path::new("2026-10.toml"), "a".to_owned(), counting(&calls));
        _ = parsed.events(Path::new("2026-11.toml"), "a".to_owned(), counting(&calls));

        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn written_text_is_not_parsed() {
        let parsed = Parsed::default();
        let calls = Cell::new(0);
        parsed.remember(PathBuf::from("2026-11.toml"), "a".to_owned(), Arc::new([]));

        _ = parsed.events(Path::new("2026-11.toml"), "a".to_owned(), counting(&calls));

        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn failed_parse_is_not_kept() {
        let parsed = Parsed::default();
        let calls = Cell::new(0);
        let failing = |_: &str| -> PaceResult<Vec<RecordedEvent>> {
            Err(pace_error::PaceError::new(
                pace_error::ErrorKind::InputOutput,
                "broken",
            ))
        };

        assert!(
            parsed
                .events(Path::new("2026-11.toml"), "a".to_owned(), failing)
                .is_err()
        );
        _ = parsed.events(Path::new("2026-11.toml"), "a".to_owned(), counting(&calls));

        assert_eq!(calls.get(), 1);
    }
}
