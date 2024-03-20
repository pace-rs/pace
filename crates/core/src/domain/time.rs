use std::{
    fmt::{Display, Formatter},
    str::FromStr,
    time::Duration,
};

use chrono::{
    DateTime, Datelike, FixedOffset, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime,
    SubsecRound, TimeZone, Timelike, Utc,
};

use derive_more::{Add, AddAssign, Sub, SubAssign};
use displaydoc::Display;
use getset::Getters;
use humantime::format_duration;
use serde_derive::{Deserialize, Serialize};
use tracing::debug;
use typed_builder::TypedBuilder;

use crate::{
    commands::reflect::{DateFlags, TimeFlags},
    error::{PaceError, PaceResult, PaceTimeErrorKind},
};

pub trait Validate {
    type Output;

    /// Validate a struct
    ///
    /// # Errors
    ///
    /// Returns an error if the validation was not successful
    ///
    /// # Returns
    ///
    /// Returns the struct if the validation was successful
    fn validate(self) -> PaceResult<Self::Output>;
}

/// `TimeRangeOptions` represents the start and end time of a time range
#[derive(
    Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TypedBuilder, Eq, Hash, Default, Getters,
)]
#[getset(get = "pub")]
pub struct TimeRangeOptions {
    #[builder(default = PaceDateTime::now())]
    start: PaceDateTime,
    #[builder(default = PaceDateTime::now())]
    end: PaceDateTime,
}

impl Display for TimeRangeOptions {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} - {}", self.start, self.end)
    }
}

impl TryFrom<PaceTimeFrame> for TimeRangeOptions {
    type Error = PaceError;

    fn try_from(time_frame: PaceTimeFrame) -> Result<Self, Self::Error> {
        match time_frame {
            PaceTimeFrame::DateRange(range) => Ok(range),
            PaceTimeFrame::CurrentMonth => Self::current_month(),
            PaceTimeFrame::CurrentWeek => Self::current_week(),
            PaceTimeFrame::CurrentYear => Self::current_year(),
            PaceTimeFrame::SpecificDate(date) => Self::specific_date(date),
            PaceTimeFrame::LastMonth => Self::last_month(),
            PaceTimeFrame::LastWeek => Self::last_week(),
            PaceTimeFrame::LastYear => Self::last_year(),
            PaceTimeFrame::Today => Self::today(),
            PaceTimeFrame::Yesterday => Self::yesterday(),
        }
    }
}

impl Validate for TimeRangeOptions {
    type Output = Self;

    /// Validate the time range
    ///
    /// # Errors
    ///
    /// Returns an error if the time range is invalid
    ///
    /// # Returns
    ///
    /// Returns the time range options if they are valid
    fn validate(self) -> PaceResult<Self> {
        if self.start > self.end {
            return Err(PaceTimeErrorKind::InvalidTimeRange(
                self.start.to_string(),
                self.end.to_string(),
            )
            .into());
        }

        Ok(self)
    }
}

impl TimeRangeOptions {
    /// Check if the given time is in the range
    #[must_use]
    pub fn is_in_range(&self, time: PaceDateTime) -> bool {
        time >= self.start && time <= self.end
    }

    /// Get the time range options for the current month
    ///
    /// # Errors
    ///
    /// Returns an error if the current month cannot be calculated or if the date is invalid
    ///
    /// # Returns
    ///
    /// Returns the time range options for the current month
    pub fn current_month() -> PaceResult<Self> {
        let now = Local::now();

        let start = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).ok_or_else(|| {
            PaceTimeErrorKind::InvalidDate(format!("{}/{}", now.year(), now.month()))
        })?;

        Ok(Self::builder()
            .start(PaceDateTime::from(start.and_hms_opt(0, 0, 0).ok_or_else(
                || PaceTimeErrorKind::InvalidDate(start.to_string()),
            )?))
            .build())
    }

    /// Get the time range options for the current week
    ///
    /// # Errors
    ///
    /// Returns an error if the current week cannot be calculated or if the date is invalid
    ///
    /// # Returns
    ///
    /// Returns the time range options for the current week
    pub fn current_week() -> PaceResult<Self> {
        let now = Local::now();

        let start = now
            .date_naive()
            .pred_opt()
            .ok_or_else(|| PaceTimeErrorKind::InvalidDate(now.to_string()))?;

        let week = start.week(chrono::Weekday::Mon);

        Ok(Self::builder()
            .start(PaceDateTime::from(
                week.first_day()
                    .and_hms_opt(0, 0, 0)
                    .ok_or_else(|| PaceTimeErrorKind::InvalidDate(week.first_day().to_string()))?,
            ))
            .build())
    }

    /// Get the time range options for the current year
    ///
    /// # Errors
    ///
    /// Returns an error if the current year cannot be calculated or if the date is invalid
    ///
    /// # Returns
    ///
    /// Returns the time range options for the current year
    pub fn current_year() -> PaceResult<Self> {
        let now = Local::now();

        let start = NaiveDate::from_ymd_opt(now.year(), 1, 1)
            .ok_or_else(|| PaceTimeErrorKind::InvalidDate(format!("{}/{}", now.year(), 1)))?;

        Ok(Self::builder()
            .start(PaceDateTime::from(start.and_hms_opt(0, 0, 0).ok_or_else(
                || PaceTimeErrorKind::InvalidDate(start.to_string()),
            )?))
            .build())
    }

    /// Get the time range options for a specific date
    ///
    /// # Arguments
    ///
    /// * `date` - The specific date
    ///
    /// # Errors
    ///
    /// Returns an error if the date is invalid
    ///
    /// # Returns
    ///
    /// Returns the time range options for the specific date
    pub fn specific_date(date: PaceDate) -> PaceResult<Self> {
        // handle date if it's in the future
        let (start, end) = if date.is_future() {
            debug!("Date is in the future, using today.");
            (
                PaceDateTime::from(
                    PaceDate::default()
                        .0
                        .and_hms_opt(0, 0, 0)
                        .ok_or_else(|| PaceTimeErrorKind::InvalidDate(date.to_string()))?,
                ),
                PaceDateTime::now(),
            )
        } else {
            (
                PaceDateTime::from(
                    date.0
                        .and_hms_opt(0, 0, 0)
                        .ok_or_else(|| PaceTimeErrorKind::InvalidDate(date.to_string()))?,
                ),
                PaceDateTime::from(
                    date.0
                        .and_hms_opt(23, 59, 59)
                        .ok_or_else(|| PaceTimeErrorKind::InvalidDate(date.to_string()))?,
                ),
            )
        };

        Ok(Self::builder().start(start).end(end).build())
    }

    /// Get the time range options for the last month
    ///
    /// # Errors
    ///
    /// Returns an error if the last month cannot be calculated or if the date is invalid
    ///
    /// # Returns
    ///
    /// Returns the time range options for the last month
    pub fn last_month() -> PaceResult<Self> {
        let now = Local::now();

        let start = NaiveDate::from_ymd_opt(now.year(), now.month() - 1, 1).ok_or_else(|| {
            PaceTimeErrorKind::InvalidDate(format!("{}/{}", now.year(), now.month() - 1))
        })?;

        let end = start
            .with_day(1)
            .ok_or_else(|| PaceTimeErrorKind::InvalidDate(start.to_string()))?
            .with_month(start.month() + 1)
            .ok_or_else(|| PaceTimeErrorKind::InvalidDate(start.to_string()))?
            .pred_opt()
            .ok_or_else(|| PaceTimeErrorKind::InvalidDate(start.to_string()))?;

        Ok(Self::builder()
            .start(PaceDateTime::from(start.and_hms_opt(0, 0, 0).ok_or_else(
                || PaceTimeErrorKind::InvalidDate(start.to_string()),
            )?))
            .end(PaceDateTime::from(end.and_hms_opt(23, 59, 59).ok_or_else(
                || PaceTimeErrorKind::InvalidDate(end.to_string()),
            )?))
            .build())
    }

    /// Get the time range options for the last week
    ///
    /// # Errors
    ///
    /// Returns an error if the last week cannot be calculated or if the date is invalid
    ///
    /// # Returns
    ///
    /// Returns the time range options for the last week
    pub fn last_week() -> PaceResult<Self> {
        let now = Local::now();

        let last_week = now
            .date_naive()
            .iso_week()
            .week()
            .checked_sub(1)
            .ok_or_else(|| PaceTimeErrorKind::InvalidDate(now.date_naive().to_string()))?;

        let week = NaiveDate::from_isoywd_opt(now.year(), last_week, chrono::Weekday::Mon)
            .ok_or_else(|| PaceTimeErrorKind::InvalidDate(format!("{}/{}", now.year(), last_week)))?
            .week(chrono::Weekday::Mon);

        // handle first week of the year
        // FIXME: this is a hack, find a better way to handle this
        if week.first_day().year() != now.year() {
            let start = NaiveDate::from_ymd_opt(now.year() - 1, 12, 25).ok_or_else(|| {
                PaceTimeErrorKind::InvalidDate(format!("{}/{}", now.year() - 1, 12))
            })?;

            let end = NaiveDate::from_ymd_opt(now.year() - 1, 12, 31).ok_or_else(|| {
                PaceTimeErrorKind::InvalidDate(format!("{}/{}", now.year() - 1, 12))
            })?;

            return Ok(Self::builder()
                .start(PaceDateTime::from(start.and_hms_opt(0, 0, 0).ok_or_else(
                    || PaceTimeErrorKind::InvalidDate(start.to_string()),
                )?))
                .end(PaceDateTime::from(end.and_hms_opt(23, 59, 59).ok_or_else(
                    || PaceTimeErrorKind::InvalidDate(end.to_string()),
                )?))
                .build());
        }

        Ok(Self::builder()
            .start(PaceDateTime::from(
                week.first_day()
                    .and_hms_opt(0, 0, 0)
                    .ok_or_else(|| PaceTimeErrorKind::InvalidDate(week.first_day().to_string()))?,
            ))
            .end(PaceDateTime::from(
                week.last_day()
                    .and_hms_opt(23, 59, 59)
                    .ok_or_else(|| PaceTimeErrorKind::InvalidDate(week.last_day().to_string()))?,
            ))
            .build())
    }

    /// Get the time range options for the last year
    ///
    /// # Errors
    ///
    /// Returns an error if the last year cannot be calculated or if the date is invalid
    ///
    /// # Returns
    ///
    /// Returns the time range options for the last year
    pub fn last_year() -> PaceResult<Self> {
        let now = Local::now();

        let start = NaiveDate::from_ymd_opt(now.year() - 1, 1, 1)
            .ok_or_else(|| PaceTimeErrorKind::InvalidDate(format!("{}/{}", now.year() - 1, 1)))?;

        let end = NaiveDate::from_ymd_opt(now.year() - 1, 12, 31)
            .ok_or_else(|| PaceTimeErrorKind::InvalidDate(format!("{}/{}", now.year() - 1, 12)))?;

        Ok(Self::builder()
            .start(PaceDateTime::from(start.and_hms_opt(0, 0, 0).ok_or_else(
                || PaceTimeErrorKind::InvalidDate(start.to_string()),
            )?))
            .end(PaceDateTime::from(end.and_hms_opt(23, 59, 59).ok_or_else(
                || PaceTimeErrorKind::InvalidDate(end.to_string()),
            )?))
            .build())
    }

    /// Get the time range options for today
    ///
    /// # Errors
    ///
    /// Returns an error if the date is invalid
    ///
    /// # Returns
    ///
    /// Returns the time range options for today
    pub fn today() -> PaceResult<Self> {
        let now = Local::now();

        Ok(Self::builder()
            .start(PaceDateTime::from(
                now.date_naive()
                    .and_hms_opt(0, 0, 0)
                    .ok_or_else(|| PaceTimeErrorKind::InvalidDate(now.to_string()))?,
            ))
            .build())
    }

    /// Get the time range options for yesterday
    ///
    /// # Errors
    ///
    /// Returns an error if the date is invalid
    ///
    /// # Returns
    ///
    /// Returns the time range options for yesterday
    pub fn yesterday() -> PaceResult<Self> {
        let now = Local::now();

        let yesterday = now
            .date_naive()
            .pred_opt()
            .ok_or_else(|| PaceTimeErrorKind::InvalidDate(now.date_naive().to_string()))?;

        Ok(Self::builder()
            .start(PaceDateTime::from(
                yesterday
                    .and_hms_opt(0, 0, 0)
                    .ok_or_else(|| PaceTimeErrorKind::InvalidDate(yesterday.to_string()))?,
            ))
            .end(PaceDateTime::from(
                yesterday
                    .and_hms_opt(23, 59, 59)
                    .ok_or_else(|| PaceTimeErrorKind::InvalidDate(yesterday.to_string()))?,
            ))
            .build())
    }
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

    fn try_from(_date: PaceDate) -> Result<Self, Self::Error> {
        // if the date is invalid because of the time, use the default time
        // Ok(Self::new(date.inner().and_hms_opt(0, 0, 0).ok_or_else(
        //     || PaceTimeErrorKind::InvalidDate(date.to_string()),
        // )?))
        unimplemented!("Implement conversion from PaceDate to PaceDateTime")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Display)]
pub enum PaceTimeFrame {
    /// Current Month
    CurrentMonth,

    /// Current Week
    CurrentWeek,

    /// Current Year
    CurrentYear,

    /// Date Range: {0}
    DateRange(TimeRangeOptions),

    /// Specific Date: {0}
    SpecificDate(PaceDate),

    /// Last Month
    LastMonth,

    /// Last Week
    LastWeek,

    /// Last Year
    LastYear,

    /// Today
    #[default]
    Today,

    /// Yesterday
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
// TODO: Check if it makes sense to switch this out with `chrono-humanize` crate
#[tracing::instrument]
pub fn duration_to_str(initial_time: DateTime<Local>) -> String {
    let now = Local::now();
    let delta = now.signed_duration_since(initial_time);

    let delta = (
        delta.num_days(),
        delta.num_hours(),
        delta.num_minutes(),
        delta.num_seconds(),
    );

    debug!("Time Delta: {:?}", delta);

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

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum PaceDurationRange {
    Short,
    #[default]
    Medium,
    Long,
}

/// The duration of an activity
#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
)]
pub struct PaceDuration(u64);

impl Display for PaceDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", format_duration(Duration::from_secs(self.0)))
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
impl PaceDuration {
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub const fn as_secs(&self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn as_duration(&self) -> Duration {
        Duration::from_secs(self.0)
    }

    #[must_use]
    // We allow this because it's unlikely, that we will reach this case
    #[allow(clippy::cast_precision_loss)]
    pub fn as_minutes(&self) -> f64 {
        self.0 as f64 / 60.0
    }

    #[must_use]
    // We allow this because it's unlikely, that we will reach this case
    #[allow(clippy::cast_precision_loss)]
    pub fn as_hours(&self) -> f64 {
        self.0 as f64 / 3600.0
    }

    #[must_use]
    // We allow this because it's unlikely, that we will reach this case
    #[allow(clippy::cast_precision_loss)]
    pub fn as_days(&self) -> f64 {
        self.0 as f64 / 86400.0
    }

    #[must_use]
    // We allow this because it's unlikely, that we will reach this case
    #[allow(clippy::cast_precision_loss)]
    pub fn as_weeks(&self) -> f64 {
        self.0 as f64 / 604_800.0
    }

    #[must_use]
    pub const fn from_seconds(seconds: u64) -> Self {
        Self(seconds)
    }
}

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

/// `PaceDate`: {0}
#[derive(
    Debug,
    Serialize,
    Deserialize,
    Hash,
    Clone,
    Copy,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    displaydoc::Display,
)]
pub struct PaceDate(NaiveDate);

impl PaceDate {
    #[must_use]
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn is_future(&self) -> bool {
        self.0 > Local::now().naive_local().date()
    }

    #[must_use]
    pub const fn inner(&self) -> &NaiveDate {
        &self.0
    }
}

impl FromStr for PaceDate {
    type Err = PaceTimeErrorKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| PaceTimeErrorKind::ParsingDateFailed(format!("Invalid date: {s}")))?;

        Ok(Self(date))
    }
}

impl TryFrom<(i32, u32, u32)> for PaceDate {
    type Error = PaceTimeErrorKind;

    fn try_from((year, month, day): (i32, u32, u32)) -> Result<Self, Self::Error> {
        NaiveDate::from_ymd_opt(year, month, day).map_or_else(
            || {
                Err(PaceTimeErrorKind::InvalidDate(format!(
                    "{year}/{month}/{day}"
                )))
            },
            |date| Ok(Self(date)),
        )
    }
}

impl PaceDate {
    #[must_use]
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
pub struct PaceDateTime(DateTime<FixedOffset>);

impl<Tz: TimeZone> TryFrom<(Option<&NaiveTime>, Option<&Tz>, Option<&String>)> for PaceDateTime {
    type Error = PaceError;

    fn try_from(
        (naive_time, tz, tz_offset): (Option<&NaiveTime>, Option<&Tz>, Option<&String>),
    ) -> Result<Self, Self::Error> {
        match (naive_time, tz, tz_offset) {
            (None, None, Some(fixed_offset)) => {
                // Now with user defined tz offset
                let offset = fixed_offset.parse::<FixedOffset>().map_err(|_| {
                    PaceTimeErrorKind::ParsingFixedOffsetFailed(fixed_offset.clone())
                })?;

                Ok(Utc::now().with_timezone(&offset).round_subsecs(0).into())
            }
            (None, Some(tz), None) => {
                // Now with user defined tz or default tz from config
                Ok(Utc::now()
                    .with_timezone(tz)
                    .round_subsecs(0)
                    .fixed_offset()
                    .into())
            }
            (Some(time), Some(tz), None) => {
                let date = Utc::now().naive_local().date();

                // construct datetime from time and time zone
                let date_time = construct_date_time_tz(tz, date, time.to_owned())?;

                Ok(date_time.round_subsecs(0).fixed_offset().into())
            }
            (Some(time), None, Some(fixed_offset)) => {
                // User time with tz offset
                let offset = fixed_offset.parse::<FixedOffset>().map_err(|_| {
                    PaceTimeErrorKind::ParsingFixedOffsetFailed(fixed_offset.clone())
                })?;

                let date = Utc::now().naive_local().date();

                // construct date time from time and time zone
                let date_time: DateTime<_> =
                    construct_date_time_tz(&offset, date, time.to_owned())?;

                Ok(date_time.round_subsecs(0).fixed_offset().into())
            }
            (None, None, None) => Ok(Self::now()),
            (Some(_), None, None) => {
                // User time with Utc as default tz
                Ok(Utc::now().round_subsecs(0).fixed_offset().into())
            }
            (Some(_) | None, Some(_), Some(_)) => {
                Err(PaceTimeErrorKind::TzAndTzOffsetDefined.into())
            }
        }
    }
}

/// Construct a date time with a time zone
///
/// # Type Parameters
///
/// * `Tz` - A type implementing [`TimeZone`]
///
/// # Arguments
///
/// * `tz` - A type implementing [`TimeZone`]
/// * `date` - The date
/// * `time` - The time
///
/// # Errors
///
/// Returns an error if the date time can't be constructed
///
/// # Returns
///
/// Returns the date time with a time zone implementation
pub fn construct_date_time_tz<Tz>(
    tz: &Tz,
    date: NaiveDate,
    time: NaiveTime,
) -> PaceResult<DateTime<Tz>>
where
    Tz: TimeZone,
{
    let LocalResult::Single(datetime) = tz.with_ymd_and_hms(
        date.year(),
        date.month(),
        date.day(),
        time.hour(),
        time.minute(),
        time.second(), // This is 0 essentially
    ) else {
        return Err(PaceTimeErrorKind::InvalidUserInput.into());
    };

    Ok(datetime)
}

impl PaceDateTime {
    /// Inner time
    #[must_use]
    pub const fn inner(&self) -> DateTime<FixedOffset> {
        self.0
    }

    /// Get the date of the activity
    #[must_use]
    pub fn date_naive(&self) -> PaceDate {
        PaceDate(self.inner().date_naive())
    }

    /// Get the time of the activity
    #[must_use]
    pub fn time(&self) -> PaceTime {
        PaceTime(self.0.time())
    }

    /// Create a new `PaceDateTime`
    #[must_use]
    pub fn new(time: DateTime<FixedOffset>) -> Self {
        Self(time.round_subsecs(0))
    }

    /// Convert to a naive date time
    #[must_use]
    pub const fn date_time_naive(&self) -> NaiveDateTime {
        self.inner().naive_utc()
    }

    /// Convert to a local date time
    pub fn and_local_timezone<Tz: TimeZone>(&self, tz: &Tz) -> DateTime<Tz> {
        self.inner().with_timezone(tz)
    }

    /// Alias for `Local::now()` and used by `Self::default()`
    #[must_use]
    pub fn now() -> Self {
        Self(Local::now().round_subsecs(0).fixed_offset())
    }
}

impl Validate for PaceDateTime {
    type Output = Self;

    /// Check if time is in the future
    ///
    /// # Errors
    ///
    /// Returns an error if the time is in the future
    ///
    /// # Returns
    ///
    /// Returns the time if it's not in the future
    fn validate(self) -> PaceResult<Self> {
        if self > Self::now() {
            Err(PaceTimeErrorKind::StartTimeInFuture(self).into())
        } else {
            Ok(self)
        }
    }
}

impl Display for PaceDateTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        <DateTime<FixedOffset> as Display>::fmt(&self.0, f)
    }
}

// Default BeginTime to now
impl Default for PaceDateTime {
    fn default() -> Self {
        Self::now()
    }
}

impl From<DateTime<FixedOffset>> for PaceDateTime {
    fn from(time: DateTime<FixedOffset>) -> Self {
        Self(time.round_subsecs(0))
    }
}

impl From<Option<DateTime<FixedOffset>>> for PaceDateTime {
    fn from(time: Option<DateTime<FixedOffset>>) -> Self {
        time.map_or_else(Self::default, |time| Self(time.round_subsecs(0)))
    }
}

impl From<NaiveDateTime> for PaceDateTime {
    fn from(_time: NaiveDateTime) -> Self {
        unimplemented!("convert from NaiveDateTime to DateTime<FixedOffset>")
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
#[tracing::instrument]
pub fn calculate_duration(begin: &PaceDateTime, end: PaceDateTime) -> PaceResult<PaceDuration> {
    let duration = end.inner().signed_duration_since(begin.inner()).to_std()?;

    debug!("Duration: {:?}", duration);

    Ok(duration.into())
}

impl<Tz: TimeZone> TryFrom<(&TimeFlags, &DateFlags, Option<&Tz>, Option<&String>)>
    for PaceTimeFrame
{
    type Error = PaceError;

    fn try_from(
        (time_flags, date_flags, time_zone, time_zone_offset): (
            &TimeFlags,
            &DateFlags,
            Option<&Tz>,
            Option<&String>,
        ),
    ) -> Result<Self, Self::Error> {
        let fixed_offset = time_zone_offset
            .map(|offset| {
                offset
                    .parse::<FixedOffset>()
                    .map_err(|_| PaceTimeErrorKind::ParsingFixedOffsetFailed(offset.clone()))
            })
            .transpose()?;

        let construct_with_tz_offset = |date: &NaiveDate| -> PaceResult<PaceDateTime> {
            let Some(fixed_offset) = fixed_offset else {
                return Err(PaceTimeErrorKind::UndefinedTimeZone.into());
            };

            Ok(PaceDateTime::from(
                fixed_offset.from_utc_datetime(
                    &date
                        .and_hms_opt(0, 0, 0)
                        .ok_or_else(|| PaceTimeErrorKind::InvalidDate(date.to_string()))?,
                ),
            ))
        };

        // TODO!: Implement conversion from NaiveDate to PaceDateTime
        let construct_with_utc = |_date: &NaiveDate| -> PaceResult<PaceDateTime> {
            unimplemented!("Implement conversion from NaiveDate to PaceDateTime")
        };

        let construct_with_tz = |date: &NaiveDate| -> PaceResult<PaceDateTime> {
            construct_pace_date_time(time_zone, date.to_owned())
        };

        #[allow(clippy::type_complexity)]
        let date_time_fn: Box<dyn Fn(&NaiveDate) -> Result<PaceDateTime, PaceError>> =
            if time_zone_offset.is_some() {
                Box::new(construct_with_tz_offset)
            } else if time_zone.is_some() {
                Box::new(construct_with_tz)
            } else {
                Box::new(construct_with_utc)
            };

        let time_frame = match (time_flags, date_flags) {
            (val, _) if *val.today() => Self::Today,
            (val, _) if *val.yesterday() => Self::Yesterday,
            (val, _) if *val.current_week() => Self::CurrentWeek,
            (val, _) if *val.last_week() => Self::LastWeek,
            (val, _) if *val.current_month() => Self::CurrentMonth,
            (val, _) if *val.last_month() => Self::LastMonth,
            (
                _,
                DateFlags {
                    date: Some(specific_date),
                    from: None,
                    to: None,
                },
            ) => {
                // We have a specific date, but no date range
                Self::SpecificDate(PaceDate::from(specific_date.to_owned()))
            }
            (
                _,
                DateFlags {
                    date: None,
                    from: Some(from),
                    to: None,
                },
            ) => {
                // We have a from date, but no end date
                Self::DateRange(
                    TimeRangeOptions::builder()
                        .start(date_time_fn(from)?)
                        .build(),
                )
            }
            (
                _,
                DateFlags {
                    date: None,
                    from: None,
                    to: Some(to),
                },
            ) => {
                // We have an end date, but no start date
                Self::DateRange(TimeRangeOptions::builder().end(date_time_fn(to)?).build())
            }
            (
                _,
                DateFlags {
                    date: None,
                    from: Some(from),
                    to: Some(to),
                },
            ) => {
                // We have a date range
                Self::DateRange(
                    TimeRangeOptions::builder()
                        .start(date_time_fn(from)?)
                        .end(date_time_fn(to)?)
                        .build(),
                )
            }
            _ => Self::default(),
        };

        debug!("Time frame: {:?}", time_frame);

        Ok(time_frame)
    }
}

/// Construct a `PaceDateTime` from a date and a time zone
///
/// # Type Parameters
///
/// * `Tz` - A type implementing [`TimeZone`]
///
/// # Arguments
///
/// * `time_zone` - The time zone
/// * `date` - The date
///
/// # Errors
///
/// Returns an error if the date time can't be constructed
///
/// # Returns
///
/// Returns the date time with a time zone implementation
fn construct_pace_date_time<Tz: TimeZone>(
    time_zone: Option<&Tz>,
    date: NaiveDate,
) -> PaceResult<PaceDateTime> {
    Ok(PaceDateTime::from(
        construct_date_time_tz(
            time_zone.ok_or(PaceTimeErrorKind::UndefinedTimeZone)?,
            date,
            NaiveTime::from_hms_opt(0, 0, 0)
                .ok_or_else(|| PaceTimeErrorKind::InvalidDate(date.to_string()))?,
        )?
        .fixed_offset(),
    ))
}

/// Get the local time zone offset to UTC to guess the time zones
///
/// # Returns
///
/// The local time zone offset
#[must_use]
pub fn get_local_time_zone_offset() -> i32 {
    Local::now().offset().local_minus_utc()
}

impl std::ops::AddAssign<u64> for PaceDuration {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

impl std::ops::SubAssign<u64> for PaceDuration {
    fn sub_assign(&mut self, rhs: u64) {
        self.0 -= rhs;
    }
}

#[cfg(test)]
mod tests {

    use chrono::NaiveDate;

    use crate::error::TestResult;

    use super::*;

    #[test]
    fn test_duration_to_str_passes() {
        let initial_time = Local::now();
        let result = duration_to_str(initial_time);
        assert_eq!(result, "just now");
    }

    #[test]
    fn test_pace_date_time_is_future_fails() -> TestResult<()> {
        let future = Local::now() + chrono::TimeDelta::try_days(1).ok_or("Invalid time delta.")?;
        let time = PaceDateTime::new(future.naive_local());

        let result = time.validate();
        assert!(result.is_err());

        Ok(())
    }

    // TODO: Rewrite to PaceDateTime
    // #[test]
    // fn test_parse_time_from_user_input_passes() -> TestResult<()> {
    //     let time = Some("12:00".to_string());

    //     let result = parse_time_from_user_input(&time)?.ok_or("No time.")?;

    //     assert_eq!(
    //         result,
    //         DateTime<Utc>::new(
    //             Local::now().date_naive(),
    //             NaiveTime::from_hms_opt(12, 0, 0).ok_or("Invalid date.")?,
    //         )
    //     );

    //     Ok(())
    // }

    #[test]
    fn test_calculate_duration_passes() -> TestResult<()> {
        let begin = PaceDateTime::new(DateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or("Invalid date.")?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
        ));
        let end = DateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or("Invalid date.")?,
            NaiveTime::from_hms_opt(0, 0, 1).ok_or("Invalid date.")?,
        );

        let duration = calculate_duration(&begin, end.into())?;
        assert_eq!(duration, Duration::from_secs(1).into());

        Ok(())
    }

    #[test]
    fn test_calculate_duration_fails() -> TestResult<()> {
        let begin = PaceDateTime::new(DateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or("Invalid date.")?,
            NaiveTime::from_hms_opt(0, 0, 1).ok_or("Invalid date.")?,
        ));
        let end = DateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or("Invalid date.")?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
        );

        let duration = calculate_duration(&begin, end.into());

        assert!(duration.is_err());

        Ok(())
    }

    #[test]
    fn test_pace_duration_from_duration_passes() {
        let duration = Duration::from_secs(1);
        let result = PaceDuration::from(duration);
        assert_eq!(result, PaceDuration(1));
    }

    #[test]
    fn test_pace_duration_from_chrono_duration_passes() -> TestResult<()> {
        let duration = chrono::TimeDelta::try_seconds(1).ok_or("Invalid time delta.")?;
        let result = PaceDuration::try_from(duration)?;
        assert_eq!(result, PaceDuration(1));

        Ok(())
    }

    #[test]
    fn test_begin_date_time_new_passes() -> TestResult<()> {
        let time = DateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or("Invalid date.")?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
        );
        let result = PaceDateTime::new(time);
        assert_eq!(result, PaceDateTime(time));

        Ok(())
    }

    #[test]
    fn test_begin_date_time_naive_date_time_passes() -> TestResult<()> {
        let time = DateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or("Invalid date.")?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
        );
        let begin_date_time = PaceDateTime::new(time);

        let result = begin_date_time.date_time_naive();

        assert_eq!(result, time);

        Ok(())
    }

    #[test]
    fn test_begin_date_time_default_passes() {
        let result = PaceDateTime::default();

        assert_eq!(
            result,
            PaceDateTime(Local::now().round_subsecs(0).fixed_offset())
        );
    }

    #[test]
    fn test_begin_date_time_from_naive_date_time_passes() -> TestResult<()> {
        let time = DateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or("Invalid date.")?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
        );

        let result = PaceDateTime::from(time);

        assert_eq!(result, PaceDateTime(time));

        Ok(())
    }

    #[test]
    fn test_pace_duration_default_passes() {
        let result = PaceDuration::default();

        assert_eq!(result, PaceDuration(0));
    }

    #[test]
    fn test_pace_duration_zero_passes() {
        let result = PaceDuration::zero();

        assert_eq!(result, PaceDuration(0));
    }

    #[test]
    fn test_pace_duration_add_assign_passes() {
        let mut duration = PaceDuration(1);
        duration += PaceDuration(1);

        assert_eq!(duration, PaceDuration(2));
    }

    #[test]
    fn test_pace_duration_sub_passes() {
        let duration = PaceDuration(2) - PaceDuration(1);

        assert_eq!(duration, PaceDuration(1));
    }

    #[test]
    fn test_pace_duration_sub_assign_passes() {
        let mut duration = PaceDuration(2);
        duration -= PaceDuration(1);

        assert_eq!(duration, PaceDuration(1));
    }

    #[test]
    fn test_pace_duration_sub_assign_with_u64_passes() {
        let mut duration = PaceDuration(2);
        duration -= 1;

        assert_eq!(duration, PaceDuration(1));
    }

    #[test]
    fn test_pace_duration_sub_assign_below_zero_passes() {
        let mut duration = PaceDuration(2);
        duration -= PaceDuration(3);

        assert_eq!(duration, PaceDuration(0));
    }

    #[test]
    fn test_pace_duration_add_passes() {
        let duration = PaceDuration(1) + PaceDuration(1);

        assert_eq!(duration, PaceDuration(2));
    }

    #[test]
    fn test_pace_duration_from_str_passes() -> TestResult<()> {
        let duration = "1".parse::<PaceDuration>()?;

        assert_eq!(duration, PaceDuration(1));

        Ok(())
    }

    #[test]
    fn test_pace_duration_from_str_fails() {
        let duration = "a".parse::<PaceDuration>();

        assert!(duration.is_err());
    }

    #[test]
    fn test_pace_date_time_is_in_range_options_passes() -> TestResult<()> {
        let activity_date_time = PaceDateTime::from(DateTime::new(
            NaiveDate::from_ymd_opt(2021, 2, 3).ok_or("Invalid date.")?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
        ));

        let time_range = TimeRangeOptions::builder()
            .start(PaceDateTime::from(DateTime::new(
                NaiveDate::from_ymd_opt(2021, 2, 2).ok_or("Invalid date.")?,
                NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
            )))
            .end(PaceDateTime::from(DateTime::new(
                NaiveDate::from_ymd_opt(2021, 2, 4).ok_or("Invalid date.")?,
                NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
            )))
            .build();

        assert!(time_range.is_in_range(activity_date_time));

        Ok(())
    }

    #[test]
    fn test_pace_date_time_is_in_range_options_fails() -> TestResult<()> {
        assert!(TimeRangeOptions::builder()
            .start(PaceDateTime::from(DateTime<Utc>::new(
                NaiveDate::from_ymd_opt(2021, 2, 4).ok_or("Invalid date.")?,
                NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
            )))
            .end(PaceDateTime::from(DateTime<Utc>::new(
                NaiveDate::from_ymd_opt(2021, 2, 2).ok_or("Invalid date.")?,
                NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
            )))
            .build()
            .validate()
            .is_err());

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_date_range_to_time_range_options_passes() -> TestResult<()> {
        let time_frame = PaceTimeFrame::DateRange(
            TimeRangeOptions::builder()
                .start(PaceDateTime::from(
                    Local::new(
                        NaiveDate::from_ymd_opt(2021, 2, 2).ok_or("Invalid date.")?,
                        NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
                    )
                    .with_timezone(&Utc),
                ))
                .end(PaceDateTime::from(
                    Local::new(
                        NaiveDate::from_ymd_opt(2021, 2, 4).ok_or("Invalid date.")?,
                        NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
                    )
                    .with_timezone(&Utc),
                ))
                .build(),
        );

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::builder()
                .start(PaceDateTime::from(DateTime<Utc>::new(
                    NaiveDate::from_ymd_opt(2021, 2, 2).ok_or("Invalid date.")?,
                    NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
                )))
                .end(PaceDateTime::from(DateTime<Utc>::new(
                    NaiveDate::from_ymd_opt(2021, 2, 4).ok_or("Invalid date.")?,
                    NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
                )))
                .build()
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_specific_date_to_time_range_options_passes() -> TestResult<()> {
        let time_frame = PaceTimeFrame::SpecificDate(PaceDate(
            NaiveDate::from_ymd_opt(2021, 2, 2).ok_or("Invalid date.")?,
        ));

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::builder()
                .start(PaceDateTime::from(DateTime<Utc>::new(
                    NaiveDate::from_ymd_opt(2021, 2, 2).ok_or("Invalid date.")?,
                    NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
                )))
                .end(PaceDateTime::from(DateTime<Utc>::new(
                    NaiveDate::from_ymd_opt(2021, 2, 2).ok_or("Invalid date.")?,
                    NaiveTime::from_hms_opt(23, 59, 59).ok_or("Invalid date.")?,
                )))
                .build()
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_current_month_to_time_range_options_passes() -> TestResult<()> {
        let time_frame = PaceTimeFrame::CurrentMonth;

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::current_month()?
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_current_week_to_time_range_options_passes() -> TestResult<()> {
        let time_frame = PaceTimeFrame::CurrentWeek;

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::current_week()?
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_current_year_to_time_range_options_passes() -> TestResult<()> {
        let time_frame = PaceTimeFrame::CurrentYear;

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::current_year()?
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_last_month_to_time_range_options_passes() -> TestResult<()> {
        let time_frame = PaceTimeFrame::LastMonth;

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::last_month()?
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_last_week_to_time_range_options_passes() -> TestResult<()> {
        let time_frame = PaceTimeFrame::LastWeek;

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::last_week()?
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_last_year_to_time_range_options_passes() -> TestResult<()> {
        let time_frame = PaceTimeFrame::LastYear;

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::last_year()?
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_today_to_time_range_options_passes() -> TestResult<()> {
        let time_frame = PaceTimeFrame::Today;

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::today()?
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_yesterday_to_time_range_options_passes() -> TestResult<()> {
        let time_frame = PaceTimeFrame::Yesterday;

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::yesterday()?
        );

        Ok(())
    }

    #[test]
    fn test_get_time_frame_from_flags_today_passes() -> TestResult<()> {
        let time_flags = TimeFlags::builder().today().build();
        let date_flags = DateFlags::default();

        let result = PaceTimeFrame::try_from((&time_flags, &date_flags))?;

        assert_eq!(result, PaceTimeFrame::Today);

        Ok(())
    }
}
