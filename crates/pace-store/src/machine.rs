//! The machine directory: this machine's device ID and how far it wrote its
//! own log. Never synced.

use std::{collections::BTreeSet, path::Path};

use pace_core::{DeviceId, EntryId, Hlc, Ids};
use pace_error::{ErrorKind, PaceError, PaceResult};
use serde::{Deserialize, Serialize};

const DEVICE_FILE: &str = "device.toml";
const CURSORS_FILE: &str = "cursors.toml";

/// This machine's device ID, and whether this run created it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceIdentity {
    /// The ID.
    pub id: DeviceId,
    /// Whether it was created just now, on the first run.
    pub created: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeviceFile {
    version: u32,
    id: DeviceId,
}

/// The device ID in the machine directory `machine`, created on first use:
/// a random UUIDv7, not the hostname (it changes and repeats) and not the OS
/// machine ID (cloned machines share it).
///
/// # Errors
///
/// If the machine directory or its `device.toml` can't be read or written.
pub fn device_id(machine: &Path, ids: &dyn Ids) -> PaceResult<DeviceIdentity> {
    let path = machine.join(DEVICE_FILE);
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let file: DeviceFile = toml::from_str(&text).map_err(|error| {
                PaceError::with_source(
                    ErrorKind::InputOutput,
                    "{path} holds no valid device ID",
                    error,
                )
                .attach_context("path", path.display().to_string())
                .attach_hint(format!(
                    "remove {} and pace gives this machine a new device ID",
                    path.display()
                ))
            })?;
            Ok(DeviceIdentity {
                id: file.id,
                created: false,
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let id = DeviceId::try_from(ids.next_id())?;
            write(&path, &DeviceFile { version: 1, id })?;
            Ok(DeviceIdentity { id, created: true })
        }
        Err(error) => Err(io_error("read", &path, error)),
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CursorsFile {
    version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    own: Option<Own>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct Own {
    /// The HLC of the last event this machine wrote, as milliseconds and
    /// counter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_hlc: Option<(i64, u32)>,
    /// Forgotten entries whose events this machine removed from its log.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    cleaned: BTreeSet<EntryId>,
}

/// How far this machine wrote its own log, and which tombstones it applied.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Cursors {
    /// The HLC of the last event this machine wrote, if it remembers one.
    pub last_written: Option<Hlc>,
    /// Forgotten entries whose events this machine removed from its log.
    pub cleaned: BTreeSet<EntryId>,
}

impl Cursors {
    /// The cursors in the machine directory; none if there is no file yet.
    pub fn load(machine: &Path) -> PaceResult<Self> {
        let path = machine.join(CURSORS_FILE);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(io_error("read", &path, error)),
        };
        let file: CursorsFile = toml::from_str(&text).map_err(|error| {
            PaceError::with_source(
                ErrorKind::InputOutput,
                "{path} is no valid cursors file",
                error,
            )
            .attach_context("path", path.display().to_string())
            .attach_hint(format!("remove {}; pace writes it again", path.display()))
        })?;
        let own = file.own.unwrap_or_default();
        Ok(Self {
            last_written: own.last_hlc.and_then(|(millis, counter)| {
                jiff::Timestamp::from_millisecond(millis)
                    .ok()
                    .map(|wall| Hlc::new(wall, counter))
            }),
            cleaned: own.cleaned,
        })
    }

    /// Write the cursors to the machine directory.
    pub fn save(&self, machine: &Path) -> PaceResult<()> {
        write(
            &machine.join(CURSORS_FILE),
            &CursorsFile {
                version: 1,
                own: Some(Own {
                    last_hlc: self
                        .last_written
                        .map(|hlc| (hlc.wall().as_millisecond(), hlc.counter())),
                    cleaned: self.cleaned.clone(),
                }),
            },
        )
    }
}

/// The error when this machine's own log has events it didn't write.
pub fn clone_detected(machine: &Path) -> Box<PaceError> {
    PaceError::new(
        ErrorKind::InputOutput,
        "another machine writes with this machine's device ID",
    )
    .attach_hint(format!(
        "pace stopped writing. If this machine was restored from a backup or \
         copied, remove {} so it gets a new device ID",
        machine.join(DEVICE_FILE).display()
    ))
    .attach_error_code("pace::store::device-cloned")
}

fn write<T: Serialize>(path: &Path, value: &T) -> PaceResult<()> {
    let text = toml::to_string(value).map_err(|error| {
        PaceError::with_source(ErrorKind::Internal, "cannot write {path}", error)
            .attach_context("path", path.display().to_string())
            .ask_report()
    })?;
    crate::files::write_atomically(path, &text)
}

fn io_error(verb: &str, path: &Path, error: std::io::Error) -> Box<PaceError> {
    PaceError::with_source(ErrorKind::InputOutput, "cannot {verb} {path}", error)
        .attach_context("verb", verb)
        .attach_context("path", path.display().to_string())
        .attach_hint("check that the machine directory exists and that pace may write there")
}
