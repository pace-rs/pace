use std::collections::BTreeSet;

use jiff::ZonedDateTime;
use serde::{Deserialize, Serialize};

use crate::{HoldSource, PauseKind, ProjectKey, Tag};

/// A use case, as data.
///
/// The CLI hands a `Command` to the core directly; a later daemon receives
/// the same value as JSON over a local socket. A command carries what the
/// user said: keys as typed, and `--at` already resolved to a moment. A
/// missing `at` means now, by the core's [`Clock`](crate::Clock).
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
    /// Write a distraction down and send it to the inbox, without breaking off.
    Catch(Catch),
    /// End the entry.
    End(End),
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
    pub note: Option<String>,
    /// The project key from `--project`; `None` leaves it to the project
    /// discovery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectKey>,
    /// Tags of the entry.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub tags: BTreeSet<Tag>,
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
    pub reason: Option<String>,
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
    pub reason: Option<String>,
}

/// `pace resume`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Resume {
    /// When the work went on; `None` means now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<ZonedDateTime>,
}

/// `pace catch`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Catch {
    /// The thought, as written down.
    pub text: String,
}

/// `pace end`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct End {
    /// When the entry ended; `None` means now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<ZonedDateTime>,
}
