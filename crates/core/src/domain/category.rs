//! Category entity and business logic

use std::{convert::Infallible, str::FromStr};

use serde_derive::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use crate::{config::GeneralConfig, domain::description::PaceDescription, prelude::CategoryGuid};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, Hash, PartialEq, Default, PartialOrd, Ord)]
pub struct PaceCategory(String);

impl PaceCategory {
    #[must_use]
    pub fn new(category: &str) -> Self {
        Self(category.to_owned())
    }
}
impl<'a, T: AsRef<&'a str>> From<T> for PaceCategory {
    fn from(category: T) -> Self {
        Self::new(category.as_ref())
    }
}

impl FromStr for PaceCategory {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

impl std::fmt::Display for PaceCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::ops::Deref for PaceCategory {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The category entity
#[derive(Debug, Serialize, Deserialize, TypedBuilder, Clone)]
struct NewCategory {
    /// The category description
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<PaceDescription>,

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
    subcategories: Option<Vec<NewCategory>>,
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
fn extract_categories(
    category_string: &str,
    separator: &str,
) -> (NewCategory, Option<NewCategory>) {
    let parts: Vec<_> = category_string.split(separator).collect();
    if parts.len() > 1 {
        // if there are more than one part, the first part is the category
        // and the rest is the subcategory
        (
            NewCategory::builder().name(parts[0].to_string()).build(),
            Some(
                NewCategory::builder()
                    .name(parts[1..].join(separator))
                    .build(),
            ),
        )
    } else {
        // if there is only one part, it's the category
        (
            NewCategory::builder()
                .name(category_string.to_owned())
                .build(),
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

impl Default for NewCategory {
    fn default() -> Self {
        Self {
            guid: Some(CategoryGuid::default()),
            name: "Uncategorized".to_string(),
            description: Some(PaceDescription::new("Uncategorized category")),
            subcategories: Option::default(),
        }
    }
}
