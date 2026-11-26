//! The names people type: project keys such as `acme-webshop`, and tags.

use std::{fmt, str::FromStr};

use pace_error::{ErrorKind, PaceError};
use serde::{Deserialize, Serialize};

/// The key of a project, as people type it: `acme-webshop`.
///
/// Lowercase ASCII letters and digits in words joined by single dashes, so a
/// key works on every command line and as a file name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProjectKey(String);

impl ProjectKey {
    /// The key as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ProjectKey {
    type Err = Box<PaceError>;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let word = |word: &str| {
            !word.is_empty()
                && word
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        };
        if text.split('-').all(word) {
            Ok(Self(text.to_owned()))
        } else {
            Err(
                PaceError::new(ErrorKind::InvalidInput, "`{key}` is not a project key")
                    .attach_context("key", text)
                    .attach_hint(
                        "a key has lowercase letters, digits and single dashes, like \
                         `acme-webshop`",
                    ),
            )
        }
    }
}

impl TryFrom<String> for ProjectKey {
    type Error = Box<PaceError>;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<ProjectKey> for String {
    fn from(key: ProjectKey) -> Self {
        key.0
    }
}

impl fmt::Display for ProjectKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A label on an entry: one word without spaces or commas, such as `dev`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Tag(String);

impl Tag {
    /// The tag as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Tag {
    type Err = Box<PaceError>;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if !text.is_empty() && !text.contains(|c: char| c.is_whitespace() || c == ',') {
            Ok(Self(text.to_owned()))
        } else {
            Err(
                PaceError::new(ErrorKind::InvalidInput, "`{tag}` is not a tag")
                    .attach_context("tag", text)
                    .attach_hint("a tag is one word without spaces or commas, like `dev`"),
            )
        }
    }
}

impl TryFrom<String> for Tag {
    type Error = Box<PaceError>;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Tag> for String {
    fn from(tag: Tag) -> Self {
        tag.0
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
