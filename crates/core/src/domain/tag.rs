use chrono::NaiveDateTime;
use chrono::{NaiveDate, NaiveTime};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;
use typed_builder::TypedBuilder;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct TagId(Uuid);

impl Default for TagId {
    fn default() -> Self {
        Self(Uuid::now_v7())
    }
}

#[derive(Debug, TypedBuilder, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Tag {
    #[builder(default, setter(strip_option))]
    id: Option<TagId>,
    text: String,
}

impl Tag {
    pub fn new(id: Option<TagId>, text: String) -> Self {
        Self { id, text }
    }
}
