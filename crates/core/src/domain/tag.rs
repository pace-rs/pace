use serde_derive::{Deserialize, Serialize};

use typed_builder::TypedBuilder;
use ulid::Ulid;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct TagId(Ulid);

impl Default for TagId {
    fn default() -> Self {
        Self(Ulid::new())
    }
}

impl rusqlite::types::FromSql for TagId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let bytes = <[u8; 16]>::column_result(value)?;
        Ok(Self(Ulid::from(u128::from_be_bytes(bytes))))
    }
}

impl rusqlite::types::ToSql for TagId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.0.to_string()))
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
