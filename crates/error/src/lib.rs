//! Error types and Result module.

use displaydoc::Display;
use miette::Diagnostic;
use std::num::TryFromIntError;
use std::{error::Error, io, path::PathBuf};
use thiserror::Error;

macro_rules! impl_pace_error_marker {
    ($error:ty) => {
        impl PaceErrorMarker for $error {}
    };
}

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
    /// [`std::io::Error`]
    #[error(transparent)]
    StdIo(#[from] std::io::Error),

    /// Serialization to TOML failed: {0}
    #[error(transparent)]
    SerializationToTomlFailed(#[from] toml::ser::Error),

    /// Deserialization from TOML failed: {0}
    #[error(transparent)]
    DeserializationFromTomlFailed(#[from] toml::de::Error),

    /// Activity store error: {0}
    #[error(transparent)]
    ActivityStore(#[from] ActivityStoreErrorKind),

    /// Activity log error: {0}
    #[error(transparent)]
    ActivityLog(#[from] ActivityLogErrorKind),

    /// Time related error: {0}
    #[error(transparent)]
    Time(#[from] TimeErrorKind),

    /// JSON error: {0}
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Chrono parse error: {0}
    #[error(transparent)]
    ChronoParse(#[from] chrono::ParseError),

    /// Time chosen is not valid, because it lays before the current activity's beginning: {0}
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

    /// There is no path available to store the activity log
    NoPathAvailable,

    /// Templating error: {0}
    #[error(transparent)]
    Template(#[from] TemplatingErrorKind),

    /// Database error: {0}
    #[error(transparent)]
    Database(#[from] DatabaseStorageErrorKind),

    /// Toml file error: {0}
    #[error(transparent)]
    TomlFile(#[from] TomlFileStorageErrorKind),

    /// Invalid Ulid parsed from string: {value} due to {source}
    InvalidGuid {
        value: String,
        #[source]
        source: ulid::DecodeError,
    },
}

/// [`DatabaseErrorKind`] describes the errors that can happen while dealing with the `SQLite` database.
#[non_exhaustive]
#[derive(Error, Debug, Display)]
pub enum DatabaseStorageErrorKind {
    /// Error connecting to database: {0} - {1}
    ConnectionFailed(String, String),

    /// No connection string provided
    NoConnectionString,

    /// This database engine is currently not supported: {0}
    UnsupportedDatabaseEngine(String),

    /// Activity with id {0} not found
    ActivityNotFound(String),

    /// Failed to create activity: {0}
    ActivityCreationFailed(String),

    /// Failed to delete activity: {0}
    ActivityDeletionFailed(String),

    /// Database storage not configured
    DatabaseStorageNotConfigured,

    /// Database storage not implemented, yet!
    StorageNotImplemented,

    /// Database migration failed. Version: {version}, Query: {query}, Source: {source}
    MigrationFailed {
        version: String,
        query: String,
        #[source]
        source: rusqlite::Error,
    },

    /// No migrations found for table: {table}
    NoMigrationsFound { table: String },

    /// Building migration query failed. Version: {version}, Table: {table}, Query: {query}, Source: {source}
    BuildingMigrationQueryFailed {
        version: String,
        table: String,
        query: String,
        #[source]
        source: sea_query::error::Error,
    },

    /// No migrations to rollback
    NoMigrationsToRollback,

    /// Migration affected multiple rows
    MigrationAffectedMultipleRows,
}

/// [`TomlFileStorageErrorKind`] describes the errors that can happen while dealing with the Toml file storage.
#[non_exhaustive]
#[derive(Error, Debug, Display)]
pub enum TomlFileStorageErrorKind {
    /// Parent directory not found: {0}
    ParentDirNotFound(PathBuf),
}

/// [`ActivityLogErrorKind`] describes the errors that can happen while dealing with the activity log.
#[non_exhaustive]
#[derive(Error, Debug, Display)]
pub enum ActivityLogErrorKind {
    /// No activities found in the activity log
    NoActivitiesFound,

    /// Activity with ID {0} not found
    FailedToReadActivity(String),

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

    /// `Activity` with id {0} not found
    ActivityNotFound(String),

    /// `Activity` with id {0} can't be removed from the activity log
    ActivityCantBeRemoved(usize),

    /// This activity has no id
    ActivityIdNotSet,

    /// `Activity` with id {0} already in use, can't create a new activity with the same id
    ActivityIdAlreadyInUse(String),

    /// `Activity` in the `ActivityLog` has a different id than the one provided: {0} != {1}
    ActivityIdMismatch(String, String),

    /// `Activity` already has an intermission: {0}
    ActivityAlreadyHasIntermission(Box<String>),

    /// There have been some activities that have not been ended
    ActivityNotEnded,

    /// No active activity found with id {0}
    NoActiveActivityFound(String),

    /// `Activity` with id {0} already ended
    ActivityAlreadyEnded(String),

    /// Activity with id {0} already has been archived
    ActivityAlreadyArchived(String),

    /// Active activity with id {0} found, although we wanted a held activity
    ActiveActivityFound(String),

    /// Activity with id {0} is not held, but we wanted to resume it
    NoHeldActivityFound(String),

    /// No activity kind options found for activity with id {0}
    ActivityKindOptionsNotFound(String),

    /// `ParentId` not set for activity with id {0}
    ParentIdNotSet(String),

    /// Category not set for activity with id {0}
    CategoryNotSet(String),

    /// No active activity to adjust
    NoActiveActivityToAdjust,

    /// Failed to group activities by keywords
    FailedToGroupByKeywords,

    /// No end options found for activity
    NoEndOptionsFound,
}

/// [`TemplatingErrorKind`] describes the errors that can happen while dealing with templating.
#[non_exhaustive]
#[derive(Error, Debug, Display)]
pub enum TemplatingErrorKind {
    /// Failed to generate context from serializable struct: {0}
    FailedToGenerateContextFromSerialize(tera::Error),

    /// Failed to render template: {0}
    RenderingToTemplateFailed(tera::Error),

    /// Failed to read template file: {0}
    FailedToReadTemplateFile(io::Error),

    /// Template file not specified
    TemplateFileNotSpecified,
}

/// [`ActivityStoreErrorKind`] describes the errors that can happen while dealing with time.
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
    ListIntermissionsForActivity(String),

    /// Missing category for activity: {0}
    MissingCategoryForActivity(String),

    /// Creating ActivityStore from storage failed
    CreatingFromStorageFailed,
}

/// [`TimeErrorKind`] describes the errors that can happen while dealing with time.
#[non_exhaustive]
#[derive(Error, Debug, Display)]
pub enum TimeErrorKind {
    /// {0}
    #[error(transparent)]
    OutOfRange(#[from] chrono::OutOfRangeError),

    /// Failed to parse time '{0}' from user input, please use the format HH:MM
    ParsingTimeFromUserInputFailed(String),

    /// The start time cannot be in the future, please use a time in the past: '{0}'
    StartTimeInFuture(String),

    /// Failed to parse duration '{0}', please use only numbers >= 0
    ParsingDurationFailed(String),

    /// Failed to parse date '{0}', please use the format YYYY-MM-DD
    InvalidDate(String),
    /// Date is not present!
    DateShouldBePresent,

    /// Failed to parse date '{0}'
    ParsingDateFailed(String),

    /// Invalid time range: Start '{0}' - End '{1}'
    InvalidTimeRange(String, String),

    /// Invalid time zone: '{0}'
    InvalidTimeZone(String),

    /// Failed to parse fixed offset '{0}' from user input, please use the format ±HHMM
    ParsingFixedOffsetFailed(String),

    /// Failed to create PaceDateTime from user input, please use the format HH:MM and ±HHMM
    InvalidUserInput,

    /// Time zone not found
    UndefinedTimeZone,

    /// Both time zone and time zone offset are defined, please use only one
    AmbiguousTimeZones,

    /// Ambiguous conversion result
    AmbiguousConversionResult,

    /// Conversion to PaceDateTime failed
    ConversionToPaceDateTimeFailed,

    /// Failed to parse time '{0}', please use the format HH:MM
    InvalidTime(String),

    /// Failed to parse time '{0}', please use rfc3339 format
    ParseError(String),

    /// Setting start of day failed
    SettingStartOfDayFailed,

    /// Adding time delta failed: '{0}'
    AddingTimeDeltaFailed(String),

    /// Failed to convert duration to i64: '{0}'
    FailedToConvertDurationToI64(TryFromIntError),

    /// Failed to convert PaceDuration to Standard Duration: '{0}'
    ConversionToDurationFailed(String),
}

trait PaceErrorMarker: Error {}

impl_pace_error_marker!(std::io::Error);
impl_pace_error_marker!(toml::de::Error);
impl_pace_error_marker!(toml::ser::Error);
impl_pace_error_marker!(serde_json::Error);
impl_pace_error_marker!(chrono::ParseError);
impl_pace_error_marker!(chrono::OutOfRangeError);
impl_pace_error_marker!(ActivityLogErrorKind);
impl_pace_error_marker!(ActivityStoreErrorKind);
impl_pace_error_marker!(TimeErrorKind);
impl_pace_error_marker!(TemplatingErrorKind);
impl_pace_error_marker!(DatabaseStorageErrorKind);
impl_pace_error_marker!(TomlFileStorageErrorKind);

impl<E> From<E> for PaceError
where
    E: PaceErrorMarker,
    PaceErrorKind: From<E>,
{
    fn from(value: E) -> Self {
        Self(PaceErrorKind::from(value))
    }
}
