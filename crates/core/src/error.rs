//! Error types and Result module.

use displaydoc::Display;
use miette::Diagnostic;
use pace_time::error::PaceTimeErrorKind;
use std::{error::Error, path::PathBuf};
use thiserror::Error;

use crate::domain::activity::{Activity, ActivityGuid};

/// Result type that is being returned from test functions and methods that can fail and thus have errors.
pub type TestResult<T> = Result<T, Box<dyn Error + 'static>>;

/// Result type that is being returned from methods that can fail and thus have [`PaceError`]s.
pub type PaceResult<T> = Result<T, PaceError>;

/// Result type that is being returned from methods that have optional return values and can fail thus having [`PaceError`]s.
pub type PaceOptResult<T> = PaceResult<Option<T>>;

/// User message type that is being returned from methods that need to print a message to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserMessage {
    /// The message to be printed to the user
    msg: String,
}

impl std::fmt::Display for UserMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl UserMessage {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { msg: msg.into() }
    }

    pub fn display(&self) {
        println!("{}", self.msg);
    }
}

impl std::ops::DerefMut for UserMessage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.msg
    }
}

impl std::ops::Deref for UserMessage {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.msg
    }
}

// [`Error`] is public, but opaque and easy to keep compatible.
/// Errors that can result from pace.
#[derive(Error, Debug, Diagnostic)]
#[diagnostic(url(docsrs))]
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

    /// Is this error related to a resumable activity so that we can prompt the user?
    ///
    /// This is useful for matching on the error kind.
    #[must_use]
    pub const fn possible_new_activity_from_resume(&self) -> bool {
        matches!(
            self.0,
            PaceErrorKind::ActivityLog(ActivityLogErrorKind::NoHeldActivityFound(_))
        ) || matches!(
            self.0,
            PaceErrorKind::ActivityLog(ActivityLogErrorKind::ActivityAlreadyEnded(_))
        ) || matches!(
            self.0,
            PaceErrorKind::ActivityLog(ActivityLogErrorKind::ActivityAlreadyArchived(_))
        )
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

    /// Serialization to TOML failed: `{0}`
    #[error(transparent)]
    SerializationToTomlFailed(#[from] toml::ser::Error),

    /// Deserialization from TOML failed: `{0}`
    #[error(transparent)]
    DeserializationFromTomlFailed(#[from] toml::de::Error),

    /// Activity store error: `{0}`
    #[error(transparent)]
    ActivityStore(#[from] ActivityStoreErrorKind),

    /// Activity log error: `{0}`
    #[error(transparent)]
    ActivityLog(#[from] ActivityLogErrorKind),

    /// Time related error: `{0}`
    #[error(transparent)]
    PaceTime(#[from] PaceTimeErrorKind),

    /// JSON error: `{0}`
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    // /// SQLite error: {0}
    // #[error(transparent)]
    // #[cfg(feature = "sqlite")]
    // SQLite(#[from] diesel::ConnectionError),
    /// Chrono parse error: `{0}`
    #[error(transparent)]
    ChronoParse(#[from] chrono::ParseError),

    /// Time chosen is not valid, because it lays before the current activity's beginning: `{0}`
    #[error(transparent)]
    ChronoDurationIsNegative(#[from] chrono::OutOfRangeError),

    /// Config file {file_name} not found in directory hierarchy starting from {current_dir}
    ConfigFileNotFound {
        /// The current directory
        current_dir: String,

        /// The file name
        file_name: String,
    },

    /// Configuration file not found, please run `pace setup config` to initialize `pace`
    ParentDirNotFound(PathBuf),

    /// Database storage not implemented, yet!
    DatabaseStorageNotImplemented,

    /// There is no path available to store the activity log
    NoPathAvailable,
}

/// [`ActivityLogErrorKind`] describes the errors that can happen while dealing with the activity log.
#[non_exhaustive]
#[derive(Error, Debug, Display)]
pub enum ActivityLogErrorKind {
    /// No activities found in the activity log
    NoActivitiesFound,

    /// Activity with ID {0} not found
    FailedToReadActivity(ActivityGuid),

    /// Negative duration for activity
    NegativeDuration,

    /// There are no activities to hold
    NoActivityToHold,

    /// Failed to unwrap Arc
    ArcUnwrapFailed,

    /// There are no unfinished activities to end
    NoUnfinishedActivities,

    /// There is no cache to sync
    NoCacheToSync,

    /// Cache not available
    CacheNotAvailable,

    /// `Activity` with id '{0}' not found
    ActivityNotFound(ActivityGuid),

    /// `Activity` with id '{0}' can't be removed from the activity log
    ActivityCantBeRemoved(usize),

    /// This activity has no id
    ActivityIdNotSet,

    /// `Activity` with id '{0}' already in use, can't create a new activity with the same id
    ActivityIdAlreadyInUse(ActivityGuid),

    /// `Activity` in the `ActivityLog` has a different id than the one provided: {0} != {1}
    ActivityIdMismatch(ActivityGuid, ActivityGuid),

    /// `Activity` already has an intermission: {0}
    ActivityAlreadyHasIntermission(Box<Activity>),

    /// There have been some activities that have not been ended
    ActivityNotEnded,

    /// No active activity found with id '{0}'
    NoActiveActivityFound(ActivityGuid),

    /// `Activity` with id '{0}' already ended
    ActivityAlreadyEnded(ActivityGuid),

    /// Activity with id '{0}' already has been archived
    ActivityAlreadyArchived(ActivityGuid),

    /// Active activity with id '{0}' found, although we wanted a held activity
    ActiveActivityFound(ActivityGuid),

    /// Activity with id '{0}' is not held, but we wanted to resume it
    NoHeldActivityFound(ActivityGuid),

    /// No activity kind options found for activity with id '{0}'
    ActivityKindOptionsNotFound(ActivityGuid),

    /// `ParentId` not set for activity with id '{0}'
    ParentIdNotSet(ActivityGuid),

    /// Category not set for activity with id '{0}'
    CategoryNotSet(ActivityGuid),

    /// No active activity to adjust
    NoActiveActivityToAdjust,

    /// Failed to group activities by keywords
    FailedToGroupByKeywords,

    /// No end options found for activity
    NoEndOptionsFound,
}

/// [`PaceTimeErrorKind`] describes the errors that can happen while dealing with time.
#[non_exhaustive]
#[derive(Error, Debug, Display)]
pub enum ActivityStoreErrorKind {
    /// Failed to list activities by id
    ListActivitiesById,

    /// Failed to group activities by duration range
    GroupByDurationRange,

    /// Failed to group activities by start date
    GroupByStartDate,

    /// Failed to list activities with intermissions
    ListActivitiesWithIntermissions,

    /// Failed to group activities by keywords
    GroupByKeywords,

    /// Failed to group activities by kind
    GroupByKind,

    /// Failed to list activities by time range
    ListActivitiesByTimeRange,

    /// Failed to populate `ActivityStore` cache
    PopulatingCache,

    /// Failed to list activities for activity: {0}
    ListIntermissionsForActivity(ActivityGuid),

    /// Missing category for activity: {0}
    MissingCategoryForActivity(ActivityGuid),
}

trait PaceErrorMarker: Error {}

impl PaceErrorMarker for std::io::Error {}
impl PaceErrorMarker for toml::de::Error {}
impl PaceErrorMarker for toml::ser::Error {}
impl PaceErrorMarker for serde_json::Error {}
#[cfg(feature = "sqlite")]
impl PaceErrorMarker for diesel::ConnectionError {}
impl PaceErrorMarker for chrono::ParseError {}
impl PaceErrorMarker for chrono::OutOfRangeError {}
impl PaceErrorMarker for ActivityLogErrorKind {}
impl PaceErrorMarker for PaceTimeErrorKind {}
impl PaceErrorMarker for ActivityStoreErrorKind {}

impl<E> From<E> for PaceError
where
    E: PaceErrorMarker,
    PaceErrorKind: From<E>,
{
    fn from(value: E) -> Self {
        Self(PaceErrorKind::from(value))
    }
}
