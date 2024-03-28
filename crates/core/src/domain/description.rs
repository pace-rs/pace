use std::{convert::Infallible, str::FromStr};

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, Hash, PartialEq, Default, PartialOrd, Ord)]
pub struct PaceDescription(String);

impl PaceDescription {
    #[must_use]
    pub fn new(description: &str) -> Self {
        Self(description.to_owned())
    }
}
impl<'a, T: AsRef<&'a str>> From<T> for PaceDescription {
    fn from(description: T) -> Self {
        Self::new(description.as_ref())
    }
}

impl FromStr for PaceDescription {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

impl std::fmt::Display for PaceDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::ops::Deref for PaceDescription {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
