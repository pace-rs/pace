use std::fmt::{Display, Formatter};

use chrono::{DateTime, Local, NaiveDateTime, NaiveTime, SubsecRound, TimeZone};
use serde_derive::{Deserialize, Serialize};

use crate::error::{PaceErrorKind, PaceOptResult, PaceResult};

pub enum TimeFrame {
    Custom {
        start: DateTime<Local>,
        end: DateTime<Local>,
    },
    Daily,
    DaysInThePast(u32),
    Monthly,
    MonthsInThePast(u32),
    Weekly,
    WeeksInThePast(u32),
    Yearly,
    YearsInThePast(u32),
}

/// Converts timespec to nice readable relative time string
///
/// # Arguments
///
/// * `initial_time` - The initial time to calculate the relative time from
///
/// # Returns
///
/// A string representing the relative time from the initial time
pub fn duration_to_str(initial_time: DateTime<Local>) -> String {
    let now = Local::now();
    let delta = now.signed_duration_since(initial_time);

    let delta = (
        delta.num_days(),
        delta.num_hours(),
        delta.num_minutes(),
        delta.num_seconds(),
    );

    match delta {
        (days, ..) if days > 5 => format!("{}", initial_time.format("%b %d, %Y")),
        (days @ 2..=5, ..) => format!("{days} days ago"),
        (1, ..) => "one day ago".to_string(),

        (_, hours, ..) if hours > 1 => format!("{hours} hours ago"),
        (_, 1, ..) => "an hour ago".to_string(),

        (_, _, minutes, _) if minutes > 1 => format!("{minutes} minutes ago"),
        (_, _, 1, _) => "one minute ago".to_string(),

        (_, _, _, seconds) if seconds > 0 => format!("{seconds} seconds ago"),
        _ => "just now".to_string(),
    }
}

/// Extracts time from the given string or returns the current time
///
/// # Arguments
///
/// * `time` - The time to extract or None
///
/// # Errors
///
/// [`chrono::ParseError`] - If the time cannot be parsed
///
/// # Returns
///
/// A tuple containing the time and date
pub fn extract_time_or_now(time: &Option<String>) -> PaceResult<NaiveDateTime> {
    Ok(if let Some(ref time) = time {
        NaiveDateTime::new(
            Local::now().date_naive(),
            NaiveTime::parse_from_str(time, "%H:%M")?,
        )
    } else {
        // if no time is given, use the current time
        Local::now().naive_local().round_subsecs(0)
    })
}

/// Parses time from user input
///
/// # Arguments
///
/// * `time` - The time to parse
///
/// # Errors
///
/// [`PaceErrorKind::ParsingTimeFromUserInputFailed`] - If the time cannot be parsed
///
/// # Returns
///
/// The parsed time or None
pub fn parse_time_from_user_input(time: &Option<String>) -> PaceOptResult<NaiveDateTime> {
    time.as_ref()
        .map(|time| -> PaceResult<NaiveDateTime> {
            let Ok(time) = NaiveTime::parse_from_str(time, "%H:%M") else {
                return Err(PaceErrorKind::ParsingTimeFromUserInputFailed(time.clone()).into());
            };

            Ok(NaiveDateTime::new(Local::now().date_naive(), time))
        })
        .transpose()
}

/// The duration of an activity
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct PaceDuration(u64);

impl From<std::time::Duration> for PaceDuration {
    fn from(duration: std::time::Duration) -> Self {
        Self(duration.as_secs())
    }
}

impl From<chrono::Duration> for PaceDuration {
    fn from(duration: chrono::Duration) -> Self {
        Self(
            duration
                .num_seconds()
                .try_into()
                .expect("Can't convert chrono duration to pace duration"),
        )
    }
}

/// Wrapper for the start time of an activity to implement default
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Eq, PartialEq)]
pub struct BeginDateTime(NaiveDateTime);

impl BeginDateTime {
    /// Convert to a naive date time
    pub fn naive_date_time(&self) -> NaiveDateTime {
        self.0
    }

    pub fn and_local_timezone<Tz: TimeZone>(&self, tz: Tz) -> chrono::LocalResult<DateTime<Tz>> {
        self.0.and_local_timezone(tz)
    }
}

impl Display for BeginDateTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        <NaiveDateTime as Display>::fmt(&self.0, f)
    }
}

// Default BeginTime to now
impl Default for BeginDateTime {
    fn default() -> Self {
        Self(Local::now().naive_local().round_subsecs(0))
    }
}

impl From<NaiveDateTime> for BeginDateTime {
    fn from(time: NaiveDateTime) -> Self {
        Self(time)
    }
}
