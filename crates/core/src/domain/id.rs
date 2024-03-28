use std::{fmt::Display, str::FromStr};

use pace_error::PaceErrorKind;
use serde_derive::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialEq, PartialOrd, Eq, Copy, Hash)]
pub struct Guid(Ulid);

impl Guid {
    pub fn new() -> Self {
        Self(Ulid::new())
    }
}

impl Default for Guid {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for Guid {
    type Err = PaceErrorKind;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(Ulid::from_string(value).map_err(|source| {
            PaceErrorKind::InvalidGuid {
                value: value.to_string(),
                source,
            }
        })?))
    }
}

impl Display for Guid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ulid: {}", self.0.to_string())
    }
}
