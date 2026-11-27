//! Where a hold came from, what kind of break a pause is, and who determined
//! an entry's time.

use serde::{Deserialize, Serialize};

/// Where an interruption came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HoldSource {
    /// From inside: you broke off yourself.
    Inner,
    /// From outside: someone expected a reaction.
    Outer,
    /// Not said.
    Unknown,
}

/// The kind of a planned break. A pause never counts as an interruption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PauseKind {
    /// A short break.
    #[default]
    Break,
    /// Lunch.
    Lunch,
    /// The break of a Pomodoro round.
    Pomodoro,
    /// A recovery break.
    Recovery,
}

impl PauseKind {
    /// Whether this is the default kind, left out when serialized.
    #[must_use]
    pub const fn is_default(&self) -> bool {
        matches!(self, Self::Break)
    }
}

/// Who determined the time of an entry: the book's self-determined and
/// other-determined time. Written `self` and `other` in the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeterminedBy {
    /// Self-determined: you chose what to do with the time.
    #[serde(rename = "self")]
    Me,
    /// Other-determined: someone else chose it for you.
    #[serde(rename = "other")]
    Others,
}
