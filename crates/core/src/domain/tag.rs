use typed_builder::TypedBuilder;
use ulid::Ulid;

use std::{collections::HashSet, convert::Infallible, str::FromStr};

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct PaceTagCollection(HashSet<PaceTag>);

impl FromIterator<PaceTag> for PaceTagCollection {
    fn from_iter<T: IntoIterator<Item = PaceTag>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl FromIterator<String> for PaceTagCollection {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        Self(
            iter.into_iter()
                .map(|tag_string| PaceTag::new(&tag_string))
                .collect(),
        )
    }
}

impl std::ops::DerefMut for PaceTagCollection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for PaceTagCollection {
    type Target = HashSet<PaceTag>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PaceTagCollection {
    #[must_use]
    pub const fn new(tags: HashSet<PaceTag>) -> Self {
        Self(tags)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, Hash, PartialEq, Default, PartialOrd, Ord)]
pub struct PaceTag(String);

impl PaceTag {
    #[must_use]
    pub fn new(tag: &str) -> Self {
        Self(tag.to_owned())
    }
}
impl<'a, T: AsRef<&'a str>> From<T> for PaceTag {
    fn from(tag: T) -> Self {
        Self::new(tag.as_ref())
    }
}

impl FromStr for PaceTag {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

impl std::fmt::Display for PaceTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::ops::Deref for PaceTag {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct TagGuid(Ulid);

impl Default for TagGuid {
    fn default() -> Self {
        Self(Ulid::new())
    }
}

#[derive(Debug, TypedBuilder, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Tag {
    #[builder(default, setter(strip_option))]
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    guid: Option<TagGuid>,

    text: String,
}

impl Tag {
    pub const fn new(guid: Option<TagGuid>, text: String) -> Self {
        Self { guid, text }
    }
}
