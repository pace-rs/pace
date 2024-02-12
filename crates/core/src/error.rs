//! Error types and Result module.

use std::{
    error::Error,
    path::{Display, PathBuf},
};
use strum_macros::Display;
use thiserror::Error;

use crate::domain::activity::ActivityId;

/// Result type that is being returned from test functions and methods that can fail and thus have errors.
pub type TestResult<T> = Result<T, Box<dyn Error + 'static>>;

/// Result type that is being returned from methods that can fail and thus have [`PaceError`]s.
pub type PaceResult<T> = Result<T, PaceError>;

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
    /// Toml serialization error: {0}
    TomlSerialize(#[from] toml::ser::Error),
    #[error(transparent)]
    TomlDeserialize(#[from] toml::de::Error),
    #[error(transparent)]
    ActivityLog(#[from] ActivityLogErrorKind),
    #[error(transparent)]
    SQLite(#[from] rusqlite::Error),
    #[error(transparent)]
    ChronoParse(#[from] chrono::ParseError),
    /// Config file {file_name} not found in directory hierarchy starting from {current_dir}
    ConfigFileNotFound {
        current_dir: String,
        file_name: String,
    },
    /// Parent directory not found in directory hierarchy: {0}
    ParentDirNotFound(PathBuf),
}

/// [`ActivityLogErrorKind`] describes the errors that can happen while dealing with the activity log.
#[non_exhaustive]
#[derive(Error, Debug, Display)]
pub enum ActivityLogErrorKind {
    NoActivityToEnd,
    NoActivitiesFound,
    FailedToReadActivity(ActivityId),
}

trait PaceErrorMarker: Error {}

impl PaceErrorMarker for std::io::Error {}
impl PaceErrorMarker for toml::de::Error {}
impl PaceErrorMarker for toml::ser::Error {}
impl PaceErrorMarker for rusqlite::Error {}
impl PaceErrorMarker for chrono::ParseError {}
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
