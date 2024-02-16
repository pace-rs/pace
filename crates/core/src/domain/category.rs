//! Category entity and business logic

use std::str::FromStr;

use serde_derive::{Deserialize, Serialize};
use typed_builder::TypedBuilder;
use uuid::Uuid;

/// The category entity
#[derive(Debug, Serialize, Deserialize, TypedBuilder, Clone)]
pub struct Category {
    /// The category description
    #[builder(default, setter(strip_option))]
    description: Option<String>,

    /// The category id
    #[builder(default = Some(CategoryId::default()), setter(strip_option))]
    id: Option<CategoryId>,

    /// The category name
    name: String,

    /// The category's subcategories
    // TODO: Add support for subcategories
    #[builder(default, setter(strip_option))]
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

/// The category id
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct CategoryId(Uuid);

impl Default for CategoryId {
    fn default() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for Category {
    fn default() -> Self {
        Self {
            id: Some(CategoryId::default()),
            name: "Uncategorized".to_string(),
            description: Some("Uncategorized category".to_string()),
            subcategories: Option::default(),
        }
    }
}
