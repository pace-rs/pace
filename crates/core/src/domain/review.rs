use serde_derive::{Deserialize, Serialize};
use strum_macros::EnumString;

pub struct ActivityStats {}

/// The kind of review format
/// Default: `console`
///
/// Options: `console`, `html`, `markdown`, `plain-text`
#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default, EnumString)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ReviewFormatKind {
    #[default]
    Console,
    Html,
    Csv,
    #[serde(rename = "md")]
    Markdown,
    #[serde(rename = "txt")]
    PlainText,
}
