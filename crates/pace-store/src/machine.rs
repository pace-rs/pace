//! The machine directory: this machine's device ID and how far it wrote its
//! own log. Never synced.

use std::{collections::BTreeSet, path::Path};

use pace_core::{DeviceId, EntryId, Hlc, Ids};
use pace_error::{ErrorKind, PaceError, PaceResult};
use serde::{Deserialize, Serialize};

use crate::disk::{DEVICE_FILE, FileOp, check_version, io_error, prepare, write_atomically};
use crate::hlc::{HlcParts, hlc_from_parts, hlc_parts, hlc_text};

const CURSORS_FILE: &str = "cursors.toml";

/// The version of the machine's `device.toml` this pace reads and writes.
const DEVICE_VERSION: u32 = 1;

/// The version of `cursors.toml` this pace reads and writes.
const CURSORS_VERSION: u32 = 1;

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
/// Two first runs at once agree on one ID: the file is created without
/// replacing one that appeared meanwhile, and then that one holds.
///
/// # Errors
///
/// If the machine directory or its `device.toml` can't be read or written.
pub fn device_id(machine: &Path, ids: &dyn Ids) -> PaceResult<DeviceIdentity> {
    if let Some(id) = existing_device_id(machine)? {
        return Ok(DeviceIdentity { id, created: false });
    }
    let id = DeviceId::try_from(ids.next_id())?;
    let path = machine.join(DEVICE_FILE);
    let (temporary, path) = prepare(
        &path,
        &toml_text(
            &path,
            &DeviceFile {
                version: DEVICE_VERSION,
                id,
            },
        )?,
    )?;
    match temporary.persist_noclobber(&path) {
        Ok(_) => Ok(DeviceIdentity { id, created: true }),
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
            let id = existing_device_id(machine)?.ok_or_else(|| {
                io_error(FileOp::Read, &path, std::io::ErrorKind::NotFound.into())
            })?;
            Ok(DeviceIdentity { id, created: false })
        }
        Err(error) => Err(io_error(FileOp::Write, &path, error.error)),
    }
}

/// The device ID in the machine directory `machine`, if the machine has one
/// yet. Writes nothing.
///
/// # Errors
///
/// If its `device.toml` can't be read or holds no valid ID.
pub fn existing_device_id(machine: &Path) -> PaceResult<Option<DeviceId>> {
    let path = machine.join(DEVICE_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_error(FileOp::Read, &path, error)),
    };
    let new_id = format!(
        "remove {} and pace gives this machine a new device ID",
        path.display()
    );
    check_version(&path, &text, DEVICE_VERSION, &new_id)?;
    let file: DeviceFile = toml::from_str(&text).map_err(|error| {
        PaceError::with_source(
            ErrorKind::InputOutput,
            "the device file {path} holds no valid device ID",
            error,
        )
        .attach_context("path", path.display().to_string())
        .attach_hint(new_id)
    })?;
    Ok(Some(file.id))
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
    last_hlc: Option<HlcParts>,
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
            Err(error) => return Err(io_error(FileOp::Read, &path, error)),
        };
        let rewrite = format!("remove {}; pace writes it again", path.display());
        check_version(&path, &text, CURSORS_VERSION, &rewrite)?;
        let file: CursorsFile = toml::from_str(&text).map_err(|error| {
            PaceError::with_source(
                ErrorKind::InputOutput,
                "the cursors file {path} is not valid",
                error,
            )
            .attach_context("path", path.display().to_string())
            .attach_hint(rewrite)
        })?;
        let own = file.own.unwrap_or_default();
        Ok(Self {
            last_written: own
                .last_hlc
                .map(|parts| hlc_from_parts(parts, &path))
                .transpose()?,
            cleaned: own.cleaned,
        })
    }

    /// Write the cursors to the machine directory.
    pub fn save(&self, machine: &Path) -> PaceResult<()> {
        write(
            &machine.join(CURSORS_FILE),
            &CursorsFile {
                version: CURSORS_VERSION,
                own: Some(Own {
                    last_hlc: self.last_written.map(hlc_parts),
                    cleaned: self.cleaned.clone(),
                }),
            },
        )
    }
}

/// The error when this machine's own log reaches `own_last`, beyond the
/// last event it wrote, `remembered`.
pub fn clone_detected(machine: &Path, own_last: Hlc, remembered: Hlc) -> Box<PaceError> {
    PaceError::new(
        ErrorKind::InputOutput,
        "another machine writes with this machine's device ID: the log of this ID reaches {own_last}, but this machine wrote up to {remembered}",
    )
    .attach_context("own_last", hlc_text(own_last))
    .attach_context("remembered", hlc_text(remembered))
    .attach_hint(format!(
        "if this machine was restored from a backup or copied, remove {} so it \
         gets a new device ID",
        machine.join(DEVICE_FILE).display()
    ))
    .attach_error_code("pace::store::device-cloned")
}

fn write<T: Serialize>(path: &Path, value: &T) -> PaceResult<()> {
    write_atomically(path, &toml_text(path, value)?)
}

fn toml_text<T: Serialize>(path: &Path, value: &T) -> PaceResult<String> {
    toml::to_string(value).map_err(|error| {
        PaceError::with_source(ErrorKind::Internal, "cannot write {path}", error)
            .attach_context("path", path.display().to_string())
            .ask_report()
    })
}
