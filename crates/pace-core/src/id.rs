//! The IDs of events, batches, devices, entries, projects and roles, each a
//! UUIDv7.

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

        impl TryFrom<Uuid> for $name {
            type Error = Box<PaceError>;

            fn try_from(uuid: Uuid) -> Result<Self, Self::Error> {
                if uuid.get_version() == Some(Version::SortRand) {
                    Ok(Self(uuid))
                } else {
                    Err(PaceError::new(ErrorKind::Internal, "an ID source gave `{id}`, which is no UUIDv7")
                        .attach_context("id", uuid.to_string())
                        .ask_report())
                }
            }
        }

        impl $name {
            /// The UUID.
            #[must_use]
            pub const fn uuid(&self) -> Uuid {
                self.0
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

/// The UUIDv7 `text` stands for. Text that is no UUID keeps uuid's error as
/// the cause.
fn parse_v7(text: &str) -> Result<Uuid, Box<PaceError>> {
    let not_v7 = || {
        PaceError::new(ErrorKind::InvalidInput, "`{id}` is not a UUIDv7")
            .attach_context("id", text)
            .attach_hint("pace writes its IDs itself; copy the ID pace printed")
    };
    let uuid = Uuid::parse_str(text).map_err(|error| not_v7().attach_source(error))?;
    if uuid.get_version() == Some(Version::SortRand) {
        Ok(uuid)
    } else {
        Err(not_v7())
    }
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
uuid_v7_id!(
    /// Identity of a role; its key can change, its ID never does.
    RoleId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_that_is_no_uuid_keeps_uuids_error_as_its_cause() {
        let error = "acme".parse::<EntryId>().err();

        let cause = error
            .as_deref()
            .and_then(std::error::Error::source)
            .map(ToString::to_string);
        assert!(cause.is_some_and(|cause| !cause.is_empty()), "{error:?}");
    }

    #[test]
    fn uuid_of_another_version_is_no_id() {
        let error = "67e55044-10b1-426f-9247-bb680e5fe0c8"
            .parse::<EntryId>()
            .err();

        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("`67e55044-10b1-426f-9247-bb680e5fe0c8` is not a UUIDv7")
        );
    }
}
