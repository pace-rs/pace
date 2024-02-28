use std::{
    fmt::{Display, Formatter},
    str::FromStr,
    time::Duration,
};

use crate::error::{ActivityLogErrorKind, PaceErrorKind, PaceOptResult, PaceResult};
use chrono::{
    DateTime, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, SubsecRound, TimeZone,
};
use getset::Getters;
use serde_derive::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, TypedBuilder, Eq, Hash, Default, Getters,
)]
#[getset(get = "pub")]
pub struct TimeRangeOptions {
    start: PaceDateTime,
    end: PaceDateTime,
}

impl From<(PaceDate, PaceDate)> for TimeRangeOptions {
    fn from((start, end): (PaceDate, PaceDate)) -> Self {
        Self::builder().start(start.into()).end(end.into()).build()
    }
}

impl From<PaceDate> for PaceDateTime {
    fn from(date: PaceDate) -> Self {
        // if the date is invalid because of the time, use the default time
        Self::new(
            date.0
                .and_hms_opt(0, 0, 0)
                .expect("Should be a valid date."),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PaceTimeFrame {
    CurrentMonth,
    CurrentWeek,
    CurrentYear,
    DateRange(TimeRangeOptions),
    SpecificDate(PaceDate),
    LastMonth,
    LastWeek,
    LastYear,
    #[default]
    Today,
    Yesterday,
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

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum PaceDurationRange {
    Short,
    #[default]
    Medium,
    Long,
}

/// The duration of an activity
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct PaceDuration(u64);

impl FromStr for PaceDuration {
    type Err = ActivityLogErrorKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<u64>() {
            Ok(duration) => Ok(Self(duration)),
            _ => Err(ActivityLogErrorKind::ParsingDurationFailed(s.to_string())),
        }
    }
}

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

/// Wrapper for a date of an activity
#[derive(Debug, Serialize, Deserialize, Hash, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct PaceDate(pub NaiveDate);

impl PaceDate {
    pub fn with_start() -> Self {
        Self(NaiveDate::default())
    }
}

impl Default for PaceDate {
    fn default() -> Self {
        Self(Local::now().naive_local().date())
    }
}

impl From<NaiveDate> for PaceDate {
    fn from(date: NaiveDate) -> Self {
        Self(date)
    }
}

impl std::ops::Deref for PaceDate {
    type Target = NaiveDate;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Wrapper for a time of an activity
#[derive(Debug, Serialize, Deserialize, Hash, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct PaceTime(pub NaiveTime);

impl From<NaiveTime> for PaceTime {
    fn from(time: NaiveTime) -> Self {
        Self(time)
    }
}

impl std::ops::Deref for PaceTime {
    type Target = NaiveTime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Wrapper for the start and end time of an activity to implement default
#[derive(Debug, Serialize, Deserialize, Hash, Clone, Copy, Eq, PartialEq)]
pub struct PaceDateTime(NaiveDateTime);

impl PaceDateTime {
    /// Get the date of the activity
    pub fn date(&self) -> PaceDate {
        PaceDate(self.0.date())
    }

    /// Get the time of the activity
    pub fn time(&self) -> PaceTime {
        PaceTime(self.0.time())
    }

    /// Create a new `PaceDateTime`
    pub fn new(time: NaiveDateTime) -> Self {
        Self(time.round_subsecs(0))
    }

    /// Convert to a naive date time
    pub fn naive_date_time(&self) -> NaiveDateTime {
        self.0
    }

    /// Convert to a local date time
    pub fn and_local_timezone<Tz: TimeZone>(&self, tz: Tz) -> LocalResult<DateTime<Tz>> {
        self.0.and_local_timezone(tz)
    }

    /// Alias for `Local::now()` and used by `Self::default()`
    pub fn now() -> Self {
        Self(Local::now().naive_local().round_subsecs(0))
    }
}

impl Display for PaceDateTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        <NaiveDateTime as Display>::fmt(&self.0, f)
    }
}

// Default BeginTime to now
impl Default for PaceDateTime {
    fn default() -> Self {
        Self::now()
    }
}

impl From<NaiveDateTime> for PaceDateTime {
    fn from(time: NaiveDateTime) -> Self {
        Self(time.round_subsecs(0))
    }
}

impl From<Option<NaiveDateTime>> for PaceDateTime {
    fn from(time: Option<NaiveDateTime>) -> Self {
        match time {
            Some(time) => Self(time.round_subsecs(0)),
            None => Self::default(),
        }
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
pub fn calculate_duration(begin: &PaceDateTime, end: PaceDateTime) -> PaceResult<PaceDuration> {
    let duration = end
        .0
        .signed_duration_since(begin.naive_date_time())
        .to_std()?;

    Ok(duration.into())
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
        let begin = PaceDateTime::new(NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).expect("Invalid date"),
            NaiveTime::from_hms_opt(0, 0, 0).expect("Invalid date"),
        ));
        let end = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).expect("Invalid date"),
            NaiveTime::from_hms_opt(0, 0, 1).expect("Invalid date"),
        );

        let duration = calculate_duration(&begin, end.into()).expect("Duration calculation failed");
        assert_eq!(duration, Duration::from_secs(1).into());
    }

    #[test]
    fn test_calculate_duration_fails() {
        let begin = PaceDateTime::new(NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).expect("Invalid date"),
            NaiveTime::from_hms_opt(0, 0, 1).expect("Invalid date"),
        ));
        let end = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).expect("Invalid date"),
            NaiveTime::from_hms_opt(0, 0, 0).expect("Invalid date"),
        );

        let duration = calculate_duration(&begin, end.into());
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
        let result = PaceDateTime::new(time);
        assert_eq!(result, PaceDateTime(time));
    }

    #[test]
    fn test_begin_date_time_naive_date_time_passes() {
        let time = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).expect("Invalid date"),
            NaiveTime::from_hms_opt(0, 0, 0).expect("Invalid date"),
        );
        let begin_date_time = PaceDateTime::new(time);
        let result = begin_date_time.naive_date_time();
        assert_eq!(result, time);
    }

    #[test]
    fn test_begin_date_time_default_passes() {
        let result = PaceDateTime::default();
        assert_eq!(
            result,
            PaceDateTime(Local::now().naive_local().round_subsecs(0))
        );
    }

    #[test]
    fn test_begin_date_time_from_naive_date_time_passes() {
        let time = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).expect("Invalid date"),
            NaiveTime::from_hms_opt(0, 0, 0).expect("Invalid date"),
        );
        let result = PaceDateTime::from(time);
        assert_eq!(result, PaceDateTime(time));
    }

    #[test]
    fn test_pace_duration_default_passes() {
        let result = PaceDuration::default();
        assert_eq!(result, PaceDuration(0));
    }
}
