//! The HLC as pace's files keep it: as text in the event files, as
//! milliseconds since the epoch and the counter in the machine directory.

use std::path::Path;

use pace_core::Hlc;
use pace_error::{ErrorKind, PaceError, PaceResult};

/// An HLC as milliseconds since the epoch and the counter.
pub type HlcParts = (i64, u32);

/// The parts of `hlc`, to write it.
pub fn hlc_parts(hlc: Hlc) -> HlcParts {
    (hlc.wall().as_millisecond(), hlc.counter())
}

/// `hlc` as the event files write it: `2026-11-26T08:02:11.204Z/0`.
pub fn hlc_text(hlc: Hlc) -> String {
    format!(
        "{}/{}",
        hlc.wall().strftime("%Y-%m-%dT%H:%M:%S%.3fZ"),
        hlc.counter()
    )
}

/// The HLC of `parts`, read from the file `path`.
///
/// # Errors
///
/// If the milliseconds lie outside the range of a timestamp. The error
/// names the file, so that a broken file never passes for one without a
/// reading.
pub fn hlc_from_parts((millis, counter): HlcParts, path: &Path) -> PaceResult<Hlc> {
    let wall = jiff::Timestamp::from_millisecond(millis).map_err(|error| {
        PaceError::with_source(
            ErrorKind::InputOutput,
            "{path} holds the clock reading {millis} ms, outside the range pace knows",
            error,
        )
        .attach_context("path", path.display().to_string())
        .attach_context("millis", millis.to_string())
        .attach_hint(format!("remove {}; pace writes it again", path.display()))
    })?;
    Ok(Hlc::new(wall, counter))
}
