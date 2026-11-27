//! Short IDs: the last characters of an entry ID that tell it apart.

use crate::EntryId;

/// The shortest unique form of `id` among the IDs of `among`: its last hex
/// characters, at least four.
///
/// The first characters of a UUIDv7 are its timestamp and repeat between
/// entries recorded close together; the last ones are random. It takes one
/// pass over `among`, so a sentence that names one entry costs no more than
/// that.
pub fn short_id_among(id: EntryId, among: impl IntoIterator<Item = EntryId>) -> String {
    let own = id.uuid().as_u128();
    // Each hex character is four bits: the trailing zero bits of the XOR
    // count the trailing characters two IDs share.
    let shared = among
        .into_iter()
        .filter(|other| *other != id)
        .map(|other| (own ^ other.uuid().as_u128()).trailing_zeros() / 4)
        .max()
        .unwrap_or(0);
    let hex = id.uuid().simple().to_string();
    let length = usize::try_from(shared)
        .map_or(hex.len(), |shared| shared + 1)
        .clamp(4, hex.len());
    hex[hex.len() - length..].to_owned()
}
