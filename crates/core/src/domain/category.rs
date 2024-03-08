//! Category entity and business logic

use serde_derive::{Deserialize, Serialize};
use typed_builder::TypedBuilder;
use ulid::Ulid;

use crate::GeneralConfig;

/// The category entity
#[derive(Debug, Serialize, Deserialize, TypedBuilder, Clone)]
pub struct Category {
    /// The category description
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,

    /// The category id
    #[builder(default = Some(CategoryGuid::default()), setter(strip_option))]
    #[serde(rename = "id")]
    guid: Option<CategoryGuid>,

    /// The category name
    name: String,

    /// The category's subcategories
    // TODO: Add support for subcategories
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    subcategories: Option<Vec<Category>>,
}

/// Extracts the category and subcategory from a string
///
/// # Arguments
///
/// * `category` - The category string
/// * `separator` - The separator used to separate the category and subcategory
///
/// # Returns
///
/// A tuple containing the category and subcategory
#[must_use]
pub fn extract_categories(category_string: &str, separator: &str) -> (Category, Option<Category>) {
    let parts: Vec<_> = category_string.split(separator).collect();
    if parts.len() > 1 {
        // if there are more than one part, the first part is the category
        // and the rest is the subcategory
        (
            Category::builder().name(parts[0].to_string()).build(),
            Some(Category::builder().name(parts[1..].join(separator)).build()),
        )
    } else {
        // if there is only one part, it's the category
        (
            Category::builder().name(category_string.to_owned()).build(),
            None,
        )
    }
}

/// Splits the category by the category separator or the default
/// separator from `GeneralConfig`
///
/// # Arguments
///
/// * `category_string` - The category string
/// * `separator` - The separator used to separate the category and subcategory
///
/// # Returns
///
/// A tuple containing the category and and optional subcategory
#[must_use]
pub fn split_category_by_category_separator(
    category_string: &str,
    separator: Option<&str>,
) -> (String, Option<String>) {
    let default_separator = GeneralConfig::default()
        .category_separator()
        .clone()
        .unwrap_or_else(|| "::".to_string());

    let separator = separator.unwrap_or(default_separator.as_str());

    let parts: Vec<_> = category_string.split(separator).collect();

    if parts.len() > 1 {
        // if there are more than one part, the first part is the category
        // and the rest is the subcategory
        (parts[0].to_string(), Some(parts[1..].concat()))
    } else {
        // if there is only one part, it's the category
        (parts[0].to_string(), None)
    }
}

/// The category id
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct CategoryGuid(Ulid);

impl Default for CategoryGuid {
    fn default() -> Self {
        Self(Ulid::new())
    }
}

impl Default for Category {
    fn default() -> Self {
        Self {
            guid: Some(CategoryGuid::default()),
            name: "Uncategorized".to_string(),
            description: Some("Uncategorized category".to_string()),
            subcategories: Option::default(),
        }
    }
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::FromSql for CategoryGuid {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let bytes = <[u8; 16]>::column_result(value)?;
        Ok(Self(Ulid::from(u128::from_be_bytes(bytes))))
    }
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::ToSql for CategoryGuid {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.0.to_string()))
    }
}
