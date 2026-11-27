//! The use cases as data; see [`Command`].

use std::{collections::BTreeSet, fmt};

use jiff::ZonedDateTime;
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{Error as _, IgnoredAny, MapAccess, SeqAccess, Visitor},
};

use crate::{
    Change, ClientKey, DeterminedBy, EntryId, HoldSource, InvoiceRef, PauseKind, ProjectRef,
    RoleId, Tag, Text,
};

/// A use case, as data.
///
/// The CLI hands a `Command` to the core directly; a later daemon receives
/// the same value as JSON over a local socket. A command carries what the
/// user said, already resolved by the adapter: `--at` as a moment, the
/// project from the discovery, a short ID or `--last` as an entry ID. A
/// missing `at` means now, by the core's [`Clock`](crate::Clock); a missing
/// `entry` means the open entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "kebab-case")]
pub enum Command {
    /// Begin an entry; ends a running entry at the new entry's start.
    Begin(Begin),
    /// Interrupt the running entry.
    Hold(Hold),
    /// Take a planned break from the running entry.
    Pause(Pause),
    /// End the hold or the pause.
    Resume(Resume),
    /// Write a distraction down, without breaking off.
    Catch(Catch),
    /// End the entry.
    End(End),
    /// Enter an entry afterwards.
    Log(Log),
    /// Change an entry.
    Adjust(Adjust),
    /// Revert this device's last command.
    // A unit variant of an internally tagged enum ignores other fields, so
    // it reads them with `no_fields`, which refuses them as the structs do.
    #[serde(deserialize_with = "no_fields")]
    Undo,
    /// Remove an entry for good.
    Forget(Forget),
    /// Mark a client's bill over a period as billed on an invoice.
    MarkBilled(MarkBilled),
}

/// `pace begin`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Begin {
    /// When the entry began; `None` means now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<ZonedDateTime>,
    /// What the entry is about.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<Text>,
    /// The project, from `--project` or the project discovery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectRef>,
    /// Tags of the entry.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub tags: BTreeSet<Tag>,
    /// `--role`, resolved to the role's ID; `None` takes the project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<RoleId>,
    /// `--by-me` or `--by-others`; `None` leaves it unmarked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub determined_by: Option<DeterminedBy>,
    /// `--billable` or `--not-billable`; `None` takes the project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billable: Option<bool>,
    /// `--allow-overlap`: overlapping another entry is intended.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub allow_overlap: bool,
}

/// `pace hold`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Hold {
    /// When the interruption began; `None` means now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<ZonedDateTime>,
    /// `--inner` or `--outer`; `None` takes the setting `holds.default-source`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<HoldSource>,
    /// Why, for example "phone call".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<Text>,
}

/// `pace pause`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Pause {
    /// When the break began; `None` means now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<ZonedDateTime>,
    /// The kind of break, for example `--lunch`.
    #[serde(default, skip_serializing_if = "PauseKind::is_default")]
    pub kind: PauseKind,
    /// Why.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<Text>,
}

/// `pace resume`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Resume {
    /// When the work went on; `None` means now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<ZonedDateTime>,
    /// The entry; `None` means the open one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<EntryId>,
}

/// `pace catch`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Catch {
    /// The thought, as written down.
    pub text: Text,
}

/// `pace end`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct End {
    /// When the entry ended; `None` means now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<ZonedDateTime>,
    /// The entry; `None` means the open one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<EntryId>,
}

/// `pace log`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Log {
    /// When the entry began.
    pub start: ZonedDateTime,
    /// When it ended; `--duration` is resolved by the adapter.
    pub end: ZonedDateTime,
    /// What it was about.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<Text>,
    /// Its project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectRef>,
    /// Its tags.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub tags: BTreeSet<Tag>,
    /// `--role`, resolved to the role's ID; `None` takes the project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<RoleId>,
    /// `--by-me` or `--by-others`; `None` leaves it unmarked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub determined_by: Option<DeterminedBy>,
    /// `--billable` or `--not-billable`; `None` takes the project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billable: Option<bool>,
    /// `--allow-overlap`
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub allow_overlap: bool,
}

impl Log {
    /// An entry from `start` to `end` without note, project, tags, role or
    /// billable flag, unmarked, its overlap refused.
    #[must_use]
    pub const fn new(start: ZonedDateTime, end: ZonedDateTime) -> Self {
        Self {
            start,
            end,
            note: None,
            project: None,
            tags: BTreeSet::new(),
            role: None,
            determined_by: None,
            billable: None,
            allow_overlap: false,
        }
    }
}

/// `pace adjust`, and what `pace edit` turns a change into.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Adjust {
    /// The entry; `None` means the open one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<EntryId>,
    /// A new start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<ZonedDateTime>,
    /// A new end, for an entry that has ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<ZonedDateTime>,
    /// A new note; `null` empties it.
    #[serde(default, skip_serializing_if = "Change::is_keep", with = "change")]
    pub note: Change<Text>,
    /// A new project; `null` takes it away.
    #[serde(default, skip_serializing_if = "Change::is_keep", with = "change")]
    pub project: Change<ProjectRef>,
    /// Tags to add.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub tag: BTreeSet<Tag>,
    /// Tags to remove.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub untag: BTreeSet<Tag>,
    /// A new role; `null` takes the project's role again.
    #[serde(default, skip_serializing_if = "Change::is_keep", with = "change")]
    pub role: Change<RoleId>,
    /// Who determined the time; `null` unmarks it.
    #[serde(default, skip_serializing_if = "Change::is_keep", with = "change")]
    pub determined_by: Change<DeterminedBy>,
    /// Billable or not; `null` takes the project's flag again.
    #[serde(default, skip_serializing_if = "Change::is_keep", with = "change")]
    pub billable: Change<bool>,
    /// `--allow-overlap`; the entry keeps it from now on.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub allow_overlap: bool,
}

/// `pace forget`; the adapter asks first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Forget {
    /// The entry.
    pub entry: EntryId,
}

/// `pace bill --mark-billed`: the bill of `client` over the period from
/// `start` to `end`, frozen on `invoice`. The rules decide the bill on the
/// state they read, so what they mark is the bill as it is now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MarkBilled {
    /// The client, by its key.
    pub client: ClientKey,
    /// The period's start; `None` means from the first entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<ZonedDateTime>,
    /// The period's end, which is not part of it; `None` means up to now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<ZonedDateTime>,
    /// The reference of the invoice the bill goes on.
    pub invoice: InvoiceRef,
}

/// A [`Change`] in JSON: absent keeps, `null` clears, a value sets.
mod change {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::Change;

    pub(super) fn serialize<T: Serialize, S: Serializer>(
        change: &Change<T>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match change {
            Change::Set(value) => value.serialize(serializer),
            Change::Keep | Change::Clear => serializer.serialize_none(),
        }
    }

    pub(super) fn deserialize<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Change<T>, D::Error> {
        Ok(Option::<T>::deserialize(deserializer)?.map_or(Change::Clear, Change::Set))
    }
}

/// The fields of a command without fields: none, so an unknown one is an
/// error, as `deny_unknown_fields` makes it for the other commands. serde hands
/// over the fields as a map, or as a sequence in its sequence form (`["undo"]`).
fn no_fields<'de, D: Deserializer<'de>>(deserializer: D) -> Result<(), D::Error> {
    deserializer.deserialize_any(NoFields)
}

/// Reads a map or a sequence that must be empty; see [`no_fields`].
struct NoFields;

impl<'de> Visitor<'de> for NoFields {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("no fields")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut fields: A) -> Result<(), A::Error> {
        fields
            .next_key::<String>()?
            .map_or(Ok(()), |field| Err(A::Error::unknown_field(&field, &[])))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut elements: A) -> Result<(), A::Error> {
        elements
            .next_element::<IgnoredAny>()?
            .map_or(Ok(()), |_| Err(A::Error::invalid_length(1, &self)))
    }
}
