use std::num::TryFromIntError;

use chrono::OutOfRangeError;
use displaydoc::Display;
use thiserror::Error;

use crate::date_time::PaceDateTime;

pub type PaceTimeResult<T> = Result<T, PaceTimeErrorKind>;

/// [`PaceTimeErrorKind`] describes the errors that can happen while dealing with time.
#[non_exhaustive]
#[derive(Error, Debug, Display)]
pub enum PaceTimeErrorKind {
    /// {0}
    #[error(transparent)]
    OutOfRange(#[from] OutOfRangeError),

    /// Failed to parse time '{0}' from user input, please use the format HH:MM
    ParsingTimeFromUserInputFailed(String),

    /// The start time cannot be in the future, please use a time in the past: '{0}'
    StartTimeInFuture(PaceDateTime),

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
