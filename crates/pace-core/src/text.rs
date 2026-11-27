//! Text that people write: the note of an entry, the reason of a hold or a
//! pause, a caught thought.

use std::{fmt, str::FromStr};

use pace_error::{ErrorKind, PaceError};
use serde::{Deserialize, Serialize};

/// Text with at least one word, kept as written.
///
/// A note, a reason or a thought without words would record nothing, so a
/// command refuses it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Text(String);

impl Text {
    /// The text as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Text {
    type Err = Box<PaceError>;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::try_from(text.to_owned())
    }
}

impl TryFrom<String> for Text {
    type Error = Box<PaceError>;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        if text.trim().is_empty() {
            return Err(PaceError::new(
                ErrorKind::InvalidInput,
                "the text \"{text}\" has no words",
            )
            .attach_context("text", text));
        }
        Ok(Self(text))
    }
}

impl From<Text> for String {
    fn from(text: Text) -> Self {
        text.0
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
