//! Choosing an entry by its ID or short ID.

use pace_core::{Entry, EntryId, State};
use pace_error::{ErrorKind, PaceError, PaceResult};

use crate::output;

/// The entry of `state` that `text` names: its full ID, or the end of it,
/// at least four hex characters.
///
/// # Errors
///
/// If `text` names no entry, or several.
pub fn resolve(state: &State, text: &str) -> PaceResult<EntryId> {
    if let Ok(id) = text.parse::<EntryId>()
        && state.entry(id).is_some()
    {
        return Ok(id);
    }
    let suffix = text.to_lowercase();
    if suffix.len() < 4 || !suffix.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(
            PaceError::new(ErrorKind::InvalidInput, "`{text}` is no entry ID")
                .attach_context("text", text)
                .attach_hint("use the short ID pace shows, at least four characters"),
        );
    }
    let matches: Vec<&Entry> = state
        .entries()
        .filter(|entry| entry.id.uuid().simple().to_string().ends_with(&suffix))
        .collect();
    match matches.as_slice() {
        [entry] => Ok(entry.id),
        [] => Err(
            PaceError::new(ErrorKind::InvalidInput, "pace knows no entry {text}")
                .attach_context("text", text)
                .attach_hint("`pace now` shows the IDs of the open entries"),
        ),
        several => {
            let names: Vec<String> = several
                .iter()
                .map(|entry| output::name(state, entry))
                .collect();
            Err(PaceError::new(
                ErrorKind::InvalidInput,
                "`{text}` fits {count} entries: {names}",
            )
            .attach_context("text", text)
            .attach_context("count", several.len().to_string())
            .attach_context("names", names.join(", "))
            .attach_hint("type more of the ID"))
        }
    }
}
