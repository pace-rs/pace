//! Short IDs: the last characters of an entry ID that tell it apart.

use std::collections::HashMap;

use crate::EntryId;

/// The shortest unique form of each ID: its last hex characters, at least
/// four.
///
/// The first characters of a UUIDv7 are its timestamp and repeat between
/// entries recorded close together; the last ones are random.
#[must_use]
pub fn short_ids(ids: &[EntryId]) -> HashMap<EntryId, String> {
    let hex: Vec<(EntryId, String)> = ids
        .iter()
        .map(|id| (*id, id.uuid().simple().to_string()))
        .collect();
    hex.iter()
        .map(|(id, own)| {
            let length = (4..=own.len())
                .find(|&length| {
                    let suffix = &own[own.len() - length..];
                    hex.iter()
                        .filter(|(other, _)| other != id)
                        .all(|(_, other)| !other.ends_with(suffix))
                })
                .unwrap_or(own.len());
            (*id, own[own.len() - length..].to_owned())
        })
        .collect()
}
