use std::{
    fmt::{Display, Formatter},
    time::Duration,
};

use crate::error::{PaceErrorKind, PaceOptResult, PaceResult};
use chrono::{DateTime, Local, NaiveDateTime, NaiveTime, SubsecRound, TimeZone};
use serde_derive::{Deserialize, Serialize};

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

impl From<Duration> for PaceDuration {
    fn from(duration: Duration) -> Self {
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
    pub fn new(time: NaiveDateTime) -> Self {
        Self(time)
    }

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

/// Calculate the duration of the activity
///
/// # Arguments
///
/// * `end` - The end date and time of the activity
///
/// # Errors
///
/// Returns an error if the duration can't be calculated or is negative
///
/// # Returns
///
/// Returns the duration of the activity
pub fn calculate_duration(begin: &BeginDateTime, end: NaiveDateTime) -> PaceResult<Duration> {
    let duration = end
        .signed_duration_since(begin.naive_date_time())
        .to_std()?;

    Ok(duration)
}

#[cfg(test)]
mod tests {

    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn test_duration_to_str_passes() {
        let initial_time = Local::now();
        let result = duration_to_str(initial_time);
        assert_eq!(result, "just now");
    }

    #[test]
    fn test_extract_time_or_now_passes() {
        let time = Some("12:00".to_string());
        let result = extract_time_or_now(&time).expect("Time extraction failed");
        assert_eq!(
            result,
            NaiveDateTime::new(
                Local::now().date_naive(),
                NaiveTime::from_hms_opt(12, 0, 0).expect("Invalid date"),
            )
        );
    }

    #[test]
    fn test_parse_time_from_user_input_passes() {
        let time = Some("12:00".to_string());
        let result = parse_time_from_user_input(&time).expect("Time parsing failed");
        assert_eq!(
            result,
            Some(NaiveDateTime::new(
                Local::now().date_naive(),
                NaiveTime::from_hms_opt(12, 0, 0).expect("Invalid date"),
            ))
        );
    }

    #[test]
    fn test_calculate_duration_passes() {
        let begin = BeginDateTime::new(NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).expect("Invalid date"),
            NaiveTime::from_hms_opt(0, 0, 0).expect("Invalid date"),
        ));
        let end = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).expect("Invalid date"),
            NaiveTime::from_hms_opt(0, 0, 1).expect("Invalid date"),
        );

        let duration = calculate_duration(&begin, end).expect("Duration calculation failed");
        assert_eq!(duration, Duration::from_secs(1));
    }

    #[test]
    fn test_calculate_duration_fails() {
        let begin = BeginDateTime::new(NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).expect("Invalid date"),
            NaiveTime::from_hms_opt(0, 0, 1).expect("Invalid date"),
        ));
        let end = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).expect("Invalid date"),
            NaiveTime::from_hms_opt(0, 0, 0).expect("Invalid date"),
        );

        let duration = calculate_duration(&begin, end);
        assert!(duration.is_err());
    }

    #[test]
    fn test_pace_duration_from_duration_passes() {
        let duration = Duration::from_secs(1);
        let result = PaceDuration::from(duration);
        assert_eq!(result, PaceDuration(1));
    }

    #[test]
    fn test_pace_duration_from_chrono_duration_passes() {
        let duration = chrono::Duration::seconds(1);
        let result = PaceDuration::from(duration);
        assert_eq!(result, PaceDuration(1));
    }

    #[test]
    fn test_begin_date_time_new_passes() {
        let time = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).expect("Invalid date"),
            NaiveTime::from_hms_opt(0, 0, 0).expect("Invalid date"),
        );
        let result = BeginDateTime::new(time);
        assert_eq!(result, BeginDateTime(time));
    }

    #[test]
    fn test_begin_date_time_naive_date_time_passes() {
        let time = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).expect("Invalid date"),
            NaiveTime::from_hms_opt(0, 0, 0).expect("Invalid date"),
        );
        let begin_date_time = BeginDateTime::new(time);
        let result = begin_date_time.naive_date_time();
        assert_eq!(result, time);
    }

    #[test]
    fn test_begin_date_time_default_passes() {
        let result = BeginDateTime::default();
        assert_eq!(
            result,
            BeginDateTime(Local::now().naive_local().round_subsecs(0))
        );
    }

    #[test]
    fn test_begin_date_time_from_naive_date_time_passes() {
        let time = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).expect("Invalid date"),
            NaiveTime::from_hms_opt(0, 0, 0).expect("Invalid date"),
        );
        let result = BeginDateTime::from(time);
        assert_eq!(result, BeginDateTime(time));
    }

    #[test]
    fn test_pace_duration_default_passes() {
        let result = PaceDuration::default();
        assert_eq!(result, PaceDuration(0));
    }
}
