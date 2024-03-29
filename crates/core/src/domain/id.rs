use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use pace_error::PaceErrorKind;
use serde_derive::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialEq, PartialOrd, Eq, Copy, Hash)]
pub struct Guid(Ulid);

impl Guid {
    #[must_use]
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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ulid: {}", self.0.to_string())
    }
}

/// The unique identifier of an activity
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialEq, PartialOrd, Eq, Copy, Hash)]
pub struct ActivityGuid(Guid);

impl ActivityGuid {
    #[must_use]
    pub fn new() -> Self {
        Self(Guid::new())
    }

    #[must_use]
    pub const fn with_id(id: Guid) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn inner(&self) -> &Guid {
        &self.0
    }
}

impl Display for ActivityGuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for ActivityGuid {
    fn default() -> Self {
        Self(Guid::new())
    }
}

/// The unique identifier of a category
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialEq, PartialOrd, Eq, Copy, Hash)]
pub struct CategoryGuid(Guid);

impl CategoryGuid {
    #[must_use]
    pub fn new() -> Self {
        Self(Guid::new())
    }

    #[must_use]
    pub const fn with_id(id: Guid) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn inner(&self) -> &Guid {
        &self.0
    }
}

impl Display for CategoryGuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for CategoryGuid {
    fn default() -> Self {
        Self(Guid::new())
    }
}

/// The unique identifier of a description
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialEq, PartialOrd, Eq, Copy, Hash)]
pub struct DescriptionGuid(Guid);

impl DescriptionGuid {
    #[must_use]
    pub fn new() -> Self {
        Self(Guid::new())
    }

    #[must_use]
    pub const fn with_id(id: Guid) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn inner(&self) -> &Guid {
        &self.0
    }
}

impl Display for DescriptionGuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for DescriptionGuid {
    fn default() -> Self {
        Self(Guid::new())
    }
}

/// The unique identifier of an activity kind
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialEq, PartialOrd, Eq, Copy, Hash)]
pub struct ActivityKindGuid(Guid);

impl ActivityKindGuid {
    #[must_use]
    pub fn new() -> Self {
        Self(Guid::new())
    }

    #[must_use]
    pub const fn with_id(id: Guid) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn inner(&self) -> &Guid {
        &self.0
    }
}

impl Display for ActivityKindGuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for ActivityKindGuid {
    fn default() -> Self {
        Self(Guid::new())
    }
}

/// The unique identifier of an activity status
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialEq, PartialOrd, Eq, Copy, Hash)]
pub struct ActivityStatusGuid(Guid);

impl ActivityStatusGuid {
    #[must_use]
    pub fn new() -> Self {
        Self(Guid::new())
    }

    #[must_use]
    pub const fn with_id(id: Guid) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn inner(&self) -> &Guid {
        &self.0
    }
}

impl Display for ActivityStatusGuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for ActivityStatusGuid {
    fn default() -> Self {
        Self(Guid::new())
    }
}

/// The unique identifier of a tag
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialEq, PartialOrd, Eq, Copy, Hash)]
pub struct TagGuid(Guid);

impl TagGuid {
    #[must_use]
    pub fn new() -> Self {
        Self(Guid::new())
    }

    #[must_use]
    pub const fn with_id(id: Guid) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn inner(&self) -> &Guid {
        &self.0
    }
}

impl Display for TagGuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for TagGuid {
    fn default() -> Self {
        Self(Guid::new())
    }
}
