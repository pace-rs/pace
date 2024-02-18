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

impl rusqlite::types::FromSql for TagGuid {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let bytes = <[u8; 16]>::column_result(value)?;
        Ok(Self(Ulid::from(u128::from_be_bytes(bytes))))
    }
}

impl rusqlite::types::ToSql for TagGuid {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.0.to_string()))
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
    pub fn new(guid: Option<TagGuid>, text: String) -> Self {
        Self { guid, text }
    }
}
