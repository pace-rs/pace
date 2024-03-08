use serde_derive::{Deserialize, Serialize};

use typed_builder::TypedBuilder;
use ulid::Ulid;

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
