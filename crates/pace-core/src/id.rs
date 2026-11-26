use std::{fmt, str::FromStr};

use pace_error::{ErrorKind, PaceError};
use serde::{Deserialize, Serialize};
use uuid::{Uuid, Version};

macro_rules! uuid_v7_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(Uuid);

        impl FromStr for $name {
            type Err = Box<PaceError>;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                parse_v7(text).map(Self)
            }
        }

        impl TryFrom<String> for $name {
            type Error = Box<PaceError>;

            fn try_from(text: String) -> Result<Self, Self::Error> {
                text.parse()
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> Self {
                id.to_string()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.hyphenated().fmt(f)
            }
        }
    };
}

fn parse_v7(text: &str) -> Result<Uuid, Box<PaceError>> {
    Uuid::parse_str(text)
        .ok()
        .filter(|uuid| uuid.get_version() == Some(Version::SortRand))
        .ok_or_else(|| {
            PaceError::new(ErrorKind::InvalidInput, "`{id}` is not a UUIDv7")
                .attach_context("id", text)
                .attach_hint("pace writes its IDs itself; copy the ID pace printed")
        })
}

uuid_v7_id!(
    /// Identity of an event.
    EventId
);
uuid_v7_id!(
    /// Identity of the events of one command, so they can be undone together.
    BatchId
);
uuid_v7_id!(
    /// Identity of a device, random per machine, never synced.
    DeviceId
);
uuid_v7_id!(
    /// Identity of an entry.
    EntryId
);
uuid_v7_id!(
    /// Identity of a project; its key can change, its ID never does.
    ProjectId
);
