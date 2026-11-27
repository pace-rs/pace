//! The catalog: the roles, clients and project records that rules, reports
//! and bills take as a value, with their money and rounding.

/// Declares an enum whose values people write as words, such as `area` or
/// `area-project`: `as_str`, `Display` and a `FromStr` whose error names
/// every word.
macro_rules! keyword_enum {
    (
        $(#[$doc:meta])* $name:ident, $noun:literal, {
            $($(#[$variant_doc:meta])* $variant:ident => $word:literal,)+
        }
    ) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $($(#[$variant_doc])* $variant,)+
        }

        impl $name {
            /// The word people write for it.
            #[must_use]
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $word,)+
                }
            }
        }

        impl std::str::FromStr for $name {
            type Err = Box<pace_error::PaceError>;

            fn from_str(word: &str) -> Result<Self, Self::Err> {
                match word {
                    $($word => Ok(Self::$variant),)+
                    _ => Err(pace_error::PaceError::new(
                        pace_error::ErrorKind::InvalidInput,
                        concat!("`{word}` is not a ", $noun),
                    )
                    .attach_context("word", word)
                    .attach_hint(concat!(
                        "a ", $noun, " is one of ",
                        keyword_enum!(@words $($word),+)
                    ))
                    .attach_error_code("pace::core::unknown-word")),
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
    (@words $first:literal $(, $word:literal)*) => {
        concat!("`", $first, "`" $(, ", `", $word, "`")*)
    };
}

mod money;
mod role;
mod rounding;

pub use money::{Currency, Money, Totals};
pub use role::{ArchiveReason, Archived, Role, RoleKind};
pub use rounding::{Rounding, RoundingMode, RoundingPer};
