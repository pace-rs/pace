//! Error types and Result module.

use displaydoc::Display;
use std::{
    error::Error,
    path::{Display, PathBuf},
};
use thiserror::Error;

use crate::domain::{activity::ActivityId, activity_log::ActivityLog};

/// Result type that is being returned from test functions and methods that can fail and thus have errors.
pub type TestResult<T> = Result<T, Box<dyn Error + 'static>>;

/// Result type that is being returned from methods that can fail and thus have [`PaceError`]s.
pub type PaceResult<T> = Result<T, PaceError>;

/// Result type that is being returned from methods that have optional return values and can fail thus having [`PaceError`]s.
pub type PaceOptResult<T> = PaceResult<Option<T>>;

// [`Error`] is public, but opaque and easy to keep compatible.
#[derive(Error, Debug)]
/// Errors that can result from pace.
pub struct PaceError(#[from] PaceErrorKind);

impl std::fmt::Display for PaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Accessors for anything we do want to expose publicly.
impl PaceError {
    /// Expose the inner error kind.
    ///
    /// This is useful for matching on the error kind.
    #[must_use]
    pub fn into_inner(self) -> PaceErrorKind {
        self.0
    }
}

/// [`PaceErrorKind`] describes the errors that can happen while executing a high-level command.
///
/// This is a non-exhaustive enum, so additional variants may be added in future. It is
/// recommended to match against the wildcard `_` instead of listing all possible variants,
/// to avoid problems when new variants are added.
#[non_exhaustive]
#[derive(Error, Debug, Display)]
pub enum PaceErrorKind {
    // /// [`CommandErrorKind`] describes the errors that can happen while executing a high-level command
    // #[error(transparent)]
    // Command(#[from] CommandErrorKind),
    /// [`std::io::Error`]
    #[error(transparent)]
    StdIo(#[from] std::io::Error),
    /// Serialization to TOML failed: {0}
    #[error(transparent)]
    SerializationToTomlFailed(#[from] toml::ser::Error),
    /// Deserialization from TOML failed: {0}
    #[error(transparent)]
    DeserializationFromTomlFailed(#[from] toml::de::Error),
    /// Activity log error: {0}
    #[error(transparent)]
    ActivityLog(#[from] ActivityLogErrorKind),
    /// SQLite error: {0}
    #[error(transparent)]
    SQLite(#[from] rusqlite::Error),
    /// Chrono parse error: {0}
    #[error(transparent)]
    ChronoParse(#[from] chrono::ParseError),
    /// Chrono duration is negative: {0}
    #[error(transparent)]
    ChronoDurationIsNegative(#[from] chrono::OutOfRangeError),
    /// Config file {file_name} not found in directory hierarchy starting from {current_dir}
    ConfigFileNotFound {
        /// The current directory
        current_dir: String,

        /// The file name
        file_name: String,
    },
    /// Configuration file not found, please run `pace craft setup` to initialize `pace`
    ParentDirNotFound(PathBuf),
    /// Database storage not implemented, yet!
    DatabaseStorageNotImplemented,
    /// Failed to parse time '{0}' from user input, please use the format HH:MM
    ParsingTimeFromUserInputFailed(String),
}

/// [`ActivityLogErrorKind`] describes the errors that can happen while dealing with the activity log.
#[non_exhaustive]
#[derive(Error, Debug, Display)]
pub enum ActivityLogErrorKind {
    /// No activities found in the activity log
    NoActivitiesFound,
    /// Activity with ID {0} not found
    FailedToReadActivity(ActivityId),
    /// Negative duration for activity
    NegativeDuration,
    /// There are no activities to hold
    NoActivityToHold,
    /// Failed to unwrap Arc
    ArcUnwrapFailed,
    /// Mutex lock failed, it has been poisoned
    MutexHasBeenPoisoned,
    /// There are no unfinished activities to end
    NoUnfinishedActivities,
    /// There is no cache to sync
    NoCacheToSync,
    /// Cache not available
    CacheNotAvailable,
    /// Activity with id '{0}' not found
    ActivityNotFound(ActivityId),
    /// Activity with id '{0}' can't be removed from the activity log
    ActivityCantBeRemoved(usize),
    /// This activity has no id
    ActivityIdNotSet,
    /// Activity with id '{0}' already in use, can't create a new activity with the same id
    ActivityIdAlreadyInUse(ActivityId),
}

trait PaceErrorMarker: Error {}

impl PaceErrorMarker for std::io::Error {}
impl PaceErrorMarker for toml::de::Error {}
impl PaceErrorMarker for toml::ser::Error {}
impl PaceErrorMarker for rusqlite::Error {}
impl PaceErrorMarker for chrono::ParseError {}
impl PaceErrorMarker for chrono::OutOfRangeError {}
impl PaceErrorMarker for ActivityLogErrorKind {}

impl<E> From<E> for PaceError
where
    E: PaceErrorMarker,
    PaceErrorKind: From<E>,
{
    fn from(value: E) -> Self {
        Self(PaceErrorKind::from(value))
    }
}
