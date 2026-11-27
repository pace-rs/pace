//! The names people type: the keys of projects, roles and clients such as
//! `acme-webshop`, and tags.

use std::{fmt, str::FromStr};

use pace_error::{ErrorKind, PaceError};
use serde::{Deserialize, Serialize};

/// Declares a key type with the slug rules: lowercase ASCII letters and
/// digits in words joined by single dashes, so a key works on every command
/// line and as a file name.
macro_rules! slug_key {
    ($(#[$doc:meta])* $name:ident, $noun:literal, $example:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// The key as text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl FromStr for $name {
            type Err = Box<PaceError>;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                if is_slug(text) {
                    Ok(Self(text.to_owned()))
                } else {
                    Err(PaceError::new(
                        ErrorKind::InvalidInput,
                        concat!("`{key}` is not a ", $noun, " key"),
                    )
                    .attach_context("key", text)
                    .attach_hint(concat!(
                        "a key has lowercase letters, digits and single dashes, like `",
                        $example,
                        "`"
                    )))
                }
            }
        }

        impl TryFrom<String> for $name {
            type Error = Box<PaceError>;

            fn try_from(text: String) -> Result<Self, Self::Error> {
                text.parse()
            }
        }

        impl From<$name> for String {
            fn from(key: $name) -> Self {
                key.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

/// Whether `text` is words of lowercase ASCII letters and digits, joined by
/// single dashes.
fn is_slug(text: &str) -> bool {
    let word = |word: &str| {
        !word.is_empty()
            && word
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    };
    text.split('-').all(word)
}

slug_key!(
    /// The key of a project, as people type it: `acme-webshop`.
    ProjectKey,
    "project",
    "acme-webshop"
);
slug_key!(
    /// The key of a role, as people type it: `work`.
    RoleKey,
    "role",
    "work"
);
slug_key!(
    /// The key of a client, as people type it: `acme`.
    ClientKey,
    "client",
    "acme"
);

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
