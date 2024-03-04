use std::{
    fmt::{Display, Formatter},
    str::FromStr,
    time::Duration,
};

use crate::{
    commands::review::{DateFlags, TimeFlags},
    error::{PaceOptResult, PaceResult, PaceTimeErrorKind},
};
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

impl TryFrom<(PaceDate, PaceDate)> for TimeRangeOptions {
    type Error = PaceTimeErrorKind;

    fn try_from((start, end): (PaceDate, PaceDate)) -> Result<Self, Self::Error> {
        Ok(Self::builder()
            .start(start.try_into()?)
            .end(end.try_into()?)
            .build())
    }
}

impl TryFrom<PaceDate> for PaceDateTime {
    type Error = PaceTimeErrorKind;

    fn try_from(date: PaceDate) -> Result<Self, Self::Error> {
        // if the date is invalid because of the time, use the default time
        Ok(Self::new(
            date.0
                .and_hms_opt(0, 0, 0)
                .ok_or(PaceTimeErrorKind::InvalidDate(date.to_string()))?,
        ))
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
/// [`PaceTimeErrorKind::StartTimeInFuture`] - If the time is in the future
///
/// # Returns
///
/// A tuple containing the time and date
pub fn extract_time_or_now(time: &Option<String>) -> PaceResult<PaceDateTime> {
    Ok(if let Some(ref time) = time {
        PaceDateTime::new(NaiveDateTime::new(
            Local::now().date_naive(),
            NaiveTime::parse_from_str(time, "%H:%M")?,
        ))
    } else {
        // if no time is given, use the current time
        PaceDateTime::now()
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
                return Err(PaceTimeErrorKind::ParsingTimeFromUserInputFailed(time.clone()).into());
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
    type Err = PaceTimeErrorKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u64>().map_or_else(
            |_| Err(PaceTimeErrorKind::ParsingDurationFailed(s.to_string())),
            |duration| Ok(Self(duration)),
        )
    }
}

impl From<Duration> for PaceDuration {
    fn from(duration: Duration) -> Self {
        Self(duration.as_secs())
    }
}

impl TryFrom<chrono::Duration> for PaceDuration {
    type Error = PaceTimeErrorKind;

    fn try_from(duration: chrono::Duration) -> Result<Self, Self::Error> {
        Ok(Self(duration.num_seconds().try_into().map_err(|_| {
            PaceTimeErrorKind::ParsingDurationFailed(duration.to_string())
        })?))
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
#[derive(Debug, Serialize, Deserialize, Hash, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
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

    /// Check if time is in the future
    pub fn is_future(self) -> PaceResult<Self> {
        if self > Self::now() {
            Err(PaceTimeErrorKind::StartTimeInFuture(self).into())
        } else {
            Ok(self)
        }
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
        time.map_or_else(Self::default, |time| Self(time.round_subsecs(0)))
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

/// Convert the time and date flags into a `PaceTimeFrame`
///
/// # Arguments
///
/// * `time_flags` - The time flags
/// * `date_flags` - The date flags
///
/// # Returns
///
/// A `PaceTimeFrame` representing the time frame
pub fn get_time_frame_from_flags(
    time_flags: &TimeFlags,
    date_flags: &DateFlags,
) -> PaceResult<PaceTimeFrame> {
    let time_frame = match (time_flags, date_flags) {
        (val, _) if *val.today() => PaceTimeFrame::Today,
        (val, _) if *val.yesterday() => PaceTimeFrame::Yesterday,
        (val, _) if *val.current_week() => PaceTimeFrame::CurrentWeek,
        (val, _) if *val.last_week() => PaceTimeFrame::LastWeek,
        (val, _) if *val.current_month() => PaceTimeFrame::CurrentMonth,
        (val, _) if *val.last_month() => PaceTimeFrame::LastMonth,
        (_, val) if val.date().is_some() => PaceTimeFrame::SpecificDate(PaceDate::from(
            val.date().ok_or(PaceTimeErrorKind::DateShouldBePresent)?,
        )),
        (_, val) if val.from().is_some() && val.to().is_none() => PaceTimeFrame::DateRange(
            (
                PaceDate::from(val.from().ok_or(PaceTimeErrorKind::DateShouldBePresent)?),
                PaceDate::default(),
            )
                .try_into()?,
        ),
        (_, val) if val.to().is_some() && val.from().is_none() => PaceTimeFrame::DateRange(
            (
                PaceDate::with_start(),
                PaceDate::from(val.to().ok_or(PaceTimeErrorKind::DateShouldBePresent)?),
            )
                .try_into()?,
        ),
        (_, val) if val.to().is_some() && val.from().is_some() => PaceTimeFrame::DateRange(
            (
                PaceDate::from(val.from().ok_or(PaceTimeErrorKind::DateShouldBePresent)?),
                PaceDate::from(val.to().ok_or(PaceTimeErrorKind::DateShouldBePresent)?),
            )
                .try_into()?,
        ),
        _ => PaceTimeFrame::default(),
    };

    Ok(time_frame)
}

#[cfg(test)]
mod tests {

    use chrono::NaiveDate;

    use crate::TestResult;

    use super::*;

    #[test]
    fn test_duration_to_str_passes() {
        let initial_time = Local::now();
        let result = duration_to_str(initial_time);
        assert_eq!(result, "just now");
    }

    #[test]
    fn test_extract_time_or_now_is_now_passes() -> TestResult<()> {
        let time = None;

        let result = extract_time_or_now(&time)?;

        assert_eq!(result, PaceDateTime::now());

        Ok(())
    }

    #[test]
    fn test_extract_time_or_now_passes() -> TestResult<()> {
        let time = Some("12:00".to_string());

        let result = extract_time_or_now(&time)?;

        assert_eq!(
            result,
            PaceDateTime(NaiveDateTime::new(
                Local::now().date_naive(),
                NaiveTime::from_hms_opt(12, 0, 0).ok_or("Invalid date.")?,
            ))
        );

        Ok(())
    }

    #[test]
    fn test_pace_date_time_is_future_fails() -> TestResult<()> {
        let future = Local::now() + chrono::Duration::days(1);
        let time = PaceDateTime::new(future.naive_local());

        let result = time.is_future();
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_parse_time_from_user_input_passes() -> TestResult<()> {
        let time = Some("12:00".to_string());

        let result = parse_time_from_user_input(&time)?.ok_or("No time.")?;

        assert_eq!(
            result,
            NaiveDateTime::new(
                Local::now().date_naive(),
                NaiveTime::from_hms_opt(12, 0, 0).ok_or("Invalid date.")?,
            )
        );

        Ok(())
    }

    #[test]
    fn test_calculate_duration_passes() -> TestResult<()> {
        let begin = PaceDateTime::new(NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or("Invalid date.")?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
        ));
        let end = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or("Invalid date.")?,
            NaiveTime::from_hms_opt(0, 0, 1).ok_or("Invalid date.")?,
        );

        let duration = calculate_duration(&begin, end.into())?;
        assert_eq!(duration, Duration::from_secs(1).into());

        Ok(())
    }

    #[test]
    fn test_calculate_duration_fails() -> TestResult<()> {
        let begin = PaceDateTime::new(NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or("Invalid date.")?,
            NaiveTime::from_hms_opt(0, 0, 1).ok_or("Invalid date.")?,
        ));
        let end = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or("Invalid date.")?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
        );

        let duration = calculate_duration(&begin, end.into());

        assert!(duration.is_err());

        Ok(())
    }

    #[test]
    fn test_pace_duration_from_duration_passes() -> TestResult<()> {
        let duration = Duration::from_secs(1);
        let result = PaceDuration::from(duration);
        assert_eq!(result, PaceDuration(1));

        Ok(())
    }

    #[test]
    fn test_pace_duration_from_chrono_duration_passes() -> TestResult<()> {
        let duration = chrono::Duration::seconds(1);
        let result = PaceDuration::try_from(duration)?;
        assert_eq!(result, PaceDuration(1));

        Ok(())
    }

    #[test]
    fn test_begin_date_time_new_passes() -> TestResult<()> {
        let time = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or("Invalid date.")?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
        );
        let result = PaceDateTime::new(time);
        assert_eq!(result, PaceDateTime(time));

        Ok(())
    }

    #[test]
    fn test_begin_date_time_naive_date_time_passes() -> TestResult<()> {
        let time = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or("Invalid date.")?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
        );
        let begin_date_time = PaceDateTime::new(time);

        let result = begin_date_time.naive_date_time();

        assert_eq!(result, time);

        Ok(())
    }

    #[test]
    fn test_begin_date_time_default_passes() -> TestResult<()> {
        let result = PaceDateTime::default();

        assert_eq!(
            result,
            PaceDateTime(Local::now().naive_local().round_subsecs(0))
        );

        Ok(())
    }

    #[test]
    fn test_begin_date_time_from_naive_date_time_passes() -> TestResult<()> {
        let time = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or("Invalid date.")?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
        );

        let result = PaceDateTime::from(time);

        assert_eq!(result, PaceDateTime(time));

        Ok(())
    }

    #[test]
    fn test_pace_duration_default_passes() -> TestResult<()> {
        let result = PaceDuration::default();

        assert_eq!(result, PaceDuration(0));

        Ok(())
    }
}
