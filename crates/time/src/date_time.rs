use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use chrono::{
    DateTime, Duration, FixedOffset, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime,
    SubsecRound, TimeZone,
};

use serde_derive::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    date::PaceDate, duration::PaceDuration, time::PaceTime, time_zone::PaceTimeZoneKind, Validate,
};

use pace_error::{PaceError, PaceResult, TimeErrorKind};

impl TryFrom<PaceDate> for PaceDateTime {
    type Error = TimeErrorKind;

    fn try_from(_date: PaceDate) -> Result<Self, Self::Error> {
        // if the date is invalid because of the time, use the default time
        // Ok(Self::new(date.inner().and_hms_opt(0, 0, 0).ok_or_else(
        //     || TimeErrorKind::InvalidDate(date.to_string()),
        // )?))
        unimplemented!("Implement conversion from PaceDate to PaceDateTime")
    }
}

/// Wrapper for the start and end time of an activity to implement default
#[derive(Debug, Serialize, Deserialize, Hash, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct PaceDateTime(DateTime<FixedOffset>);

impl FromStr for PaceDateTime {
    type Err = TimeErrorKind;

    /// Parse a `PaceDateTime` from a string
    ///
    /// # Arguments
    ///
    /// * `s` - The string to parse
    ///
    /// # Errors
    ///
    /// Returns an error if the string can't be parsed
    ///
    /// # Returns
    ///
    /// Returns the parsed `PaceDateTime`
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(DateTime::parse_from_rfc3339(s).map_err(|e| {
            TimeErrorKind::ParseError(format!("{e:?}"))
        })?))
    }
}

impl TryFrom<(Option<&NaiveTime>, PaceTimeZoneKind, PaceTimeZoneKind)> for PaceDateTime {
    type Error = PaceError;

    /// Try to convert from a tuple of optional naive time, time zone and time zone offset
    ///
    /// # Arguments
    ///
    /// * `0` - The naive time
    /// * `1` - The time zone kind
    /// * `2` - The time zone kind from the config
    ///
    /// # Errors
    ///
    /// Returns an error if the time zone and time zone offset are both defined,
    /// or if the time zone offset can't be parsed
    ///
    /// # Returns
    ///
    /// Returns the time with the user defined time zone or the default time zone
    /// if no time zone is defined. If no time zone is defined, the time is converted
    /// to Utc.
    fn try_from(
        (naive_time, tz, tz_config): (Option<&NaiveTime>, PaceTimeZoneKind, PaceTimeZoneKind),
    ) -> Result<Self, Self::Error> {
        match (naive_time, tz, tz_config) {
            (None, PaceTimeZoneKind::NotSet, PaceTimeZoneKind::NotSet) => Ok(Self::now()),
            (None, tzk, _) | (None, PaceTimeZoneKind::NotSet, tzk) if !tzk.is_not_set() => {
                pace_date_time_from_date_and_time_and_tz(
                    Local::now().naive_local().date(),
                    Local::now().naive_local().time(),
                    tzk,
                )
            }
            (Some(time), PaceTimeZoneKind::NotSet, tzk) | (Some(time), tzk, _)
                if !tzk.is_not_set() =>
            {
                pace_date_time_from_date_and_time_and_tz(
                    Local::now().naive_local().date(),
                    time.to_owned(),
                    tzk,
                )
            }
            _ => {
                debug!("Conversion failed with time zones: {tz:?} and {tz_config:?}");
                Err(TimeErrorKind::ConversionToPaceDateTimeFailed.into())
            }
        }
    }
}

impl PaceDateTime {
    /// Create a new `PaceDateTime`
    ///
    /// # Arguments
    ///
    /// * `date` - The date
    /// * `time` - The time
    /// * `time_zone` - The time zone kind
    ///
    /// # Errors
    ///
    /// Returns an error if the date time can't be constructed
    ///
    /// # Returns
    ///
    /// Returns the date time with a time zone implementation
    pub fn new(date: NaiveDate, time: NaiveTime, time_zone: PaceTimeZoneKind) -> PaceResult<Self> {
        pace_date_time_from_date_and_time_and_tz(date, time, time_zone)
    }

    /// Add a [`TimeDelta`] to the [`PaceDateTime`] and return a new [`PaceDateTime`]
    ///
    /// # Arguments
    ///
    /// * `rhs` - The [`TimeDelta`] to add
    ///
    /// # Errors
    ///
    /// Returns an error if the addition fails
    ///
    /// # Returns
    ///
    /// Returns the new [`PaceDateTime`] with the added [`TimeDelta`]
    pub fn add_duration(self, rhs: PaceDuration) -> PaceResult<Self> {
        Ok(Self(
            self.0
                .checked_add_signed(
                    Duration::new(
                        i64::try_from(rhs.inner())
                            .map_err(TimeErrorKind::FailedToConvertDurationToI64)?,
                        0,
                    )
                    .ok_or_else(|| TimeErrorKind::ConversionToDurationFailed(format!("{rhs:?}")))?,
                )
                .ok_or_else(|| TimeErrorKind::AddingTimeDeltaFailed(format!("{self} + {rhs:?}")))?,
        ))
    }

    // TODO! Implement this
    // pub fn with_date_and_time(
    //     year: i32,
    //     month: u32,
    //     day: u32,
    //     hour: u32,
    //     minute: u32,
    //     time_zone: TimeZoneKind,
    // ) -> PaceTimeResult<Self> {
    //     Ok(Self::with_date_time_fixed_offset(
    //         TimeZone::from_utc_datetime(
    //             &NaiveDate::from_ymd_opt(year, month, day)
    //                 .ok_or(PaceTimeErrorKind::InvalidDate(format!(
    //                     "year: {year}, month: {month}, day: {day}"
    //                 )))?
    //                 .and_hms_opt(hour, minute, 0)
    //                 .ok_or(PaceTimeErrorKind::InvalidTime(format!(
    //                     "hour: {hour}, minute: {minute}"
    //                 )))?,
    //             &Local::now().naive_local(),
    //         ),
    //     ))
    // }

    /// Set the time to the start of the day
    ///
    /// # Errors
    ///
    /// Returns an error if the time can't be set to the start of the day
    /// and the time is ambiguous
    pub fn start_of_day(mut self) -> PaceResult<Self> {
        let time_zone = self.0.offset();
        let time = self.0.date_naive().and_time(
            NaiveTime::from_hms_opt(0, 0, 0).ok_or(TimeErrorKind::SettingStartOfDayFailed)?,
        );

        if let LocalResult::Single(datetime) = time_zone.from_local_datetime(&time) {
            self.0 = datetime;
        } else {
            return Err(TimeErrorKind::AmbiguousConversionResult.into());
        }

        Ok(self)
    }

    /// Set the time to the end of the day
    ///
    /// # Errors
    ///
    /// Returns an error if the time can't be set to the end of the day
    /// and the time is ambiguous
    pub fn end_of_day(mut self) -> PaceResult<Self> {
        let time_zone = self.0.offset();
        let time = self.0.date_naive().and_time(
            NaiveTime::from_hms_opt(23, 59, 59).ok_or(TimeErrorKind::SettingStartOfDayFailed)?,
        );

        if let LocalResult::Single(datetime) = time_zone.from_local_datetime(&time) {
            self.0 = datetime;
        } else {
            return Err(TimeErrorKind::AmbiguousConversionResult.into());
        }

        Ok(self)
    }

    /// Inner time
    #[must_use]
    pub const fn inner(&self) -> DateTime<FixedOffset> {
        self.0
    }

    /// Get the date of the activity
    #[must_use]
    pub fn date_naive(&self) -> PaceDate {
        PaceDate::from(self)
    }

    /// Get the time of the activity
    #[must_use]
    pub fn time(&self) -> PaceTime {
        PaceTime::from(self)
    }

    /// Create a new `PaceDateTime`
    #[must_use]
    pub fn with_date_time_fixed_offset(time: DateTime<FixedOffset>) -> Self {
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

    /// Alias for `Utc::now()` with `FixedOffset` and used by `Self::default()`
    #[must_use]
    pub fn now() -> Self {
        Self(Local::now().round_subsecs(0).fixed_offset())
    }

    /// Create a new `PaceDateTime` with a [`FixedOffset`]
    ///
    /// # Arguments
    ///
    /// * `offset` - The [`FixedOffset`] to use
    ///
    /// # Returns
    ///
    /// Returns the new `PaceDateTime` with the given offset
    #[must_use]
    pub fn now_with_offset(offset: FixedOffset) -> Self {
        Self(Local::now().round_subsecs(0).with_timezone(&offset))
    }
}

impl Validate for PaceDateTime {
    type Output = Self;
    type Error = PaceError;

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
            Err(TimeErrorKind::StartTimeInFuture(self.to_string()).into())
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

impl TryFrom<NaiveDateTime> for PaceDateTime {
    type Error = PaceError;

    fn try_from(time: NaiveDateTime) -> PaceResult<Self> {
        // get local time zone
        let local = Local::now();
        let local = local.offset();

        // combine NaiveDateTime with local time zone
        let LocalResult::Single(datetime) = local.from_local_datetime(&time) else {
            Err(TimeErrorKind::AmbiguousConversionResult)?
        };

        Ok(Self::from(datetime.round_subsecs(0).fixed_offset()))
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
pub(crate) fn pace_date_time_from_date_and_tz_with_zero_hms(
    date: NaiveDate,
    time_zone: PaceTimeZoneKind,
) -> PaceResult<PaceDateTime> {
    pace_date_time_from_date_and_time_and_tz(
        date,
        NaiveTime::from_hms_opt(0, 0, 0)
            .ok_or_else(|| TimeErrorKind::InvalidDate(date.to_string()))?,
        time_zone,
    )?
    .validate()
}

/// Construct a date time with a time zone
///
/// # Arguments
///
/// * `tz` - The time zone kind
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
pub fn pace_date_time_from_date_and_time_and_tz(
    date: NaiveDate,
    time: NaiveTime,
    tz: PaceTimeZoneKind,
) -> PaceResult<PaceDateTime> {
    let date_time = match tz {
        PaceTimeZoneKind::TimeZone(ref tz) => {
            let LocalResult::Single(datetime) = tz.from_local_datetime(&date.and_time(time)) else {
                return Err(TimeErrorKind::AmbiguousConversionResult.into());
            };

            PaceDateTime::from(datetime.round_subsecs(0).fixed_offset())
        }
        PaceTimeZoneKind::TimeZoneOffset(ref tz) => {
            let LocalResult::Single(datetime) = tz.from_local_datetime(&date.and_time(time)) else {
                return Err(TimeErrorKind::AmbiguousConversionResult.into());
            };

            PaceDateTime::from(datetime.round_subsecs(0).fixed_offset())
        }
        PaceTimeZoneKind::NotSet => {
            let LocalResult::Single(datetime) = Local.from_local_datetime(&date.and_time(time))
            else {
                return Err(TimeErrorKind::AmbiguousConversionResult.into());
            };

            PaceDateTime::from(datetime.round_subsecs(0).fixed_offset())
        }
    };

    debug!("Constructed date time: {date_time}");

    date_time.validate()
}

impl TryFrom<(NaiveDate, NaiveTime, PaceTimeZoneKind)> for PaceDateTime {
    type Error = PaceError;

    fn try_from(
        (date, time, tz): (NaiveDate, NaiveTime, PaceTimeZoneKind),
    ) -> Result<Self, Self::Error> {
        pace_date_time_from_date_and_time_and_tz(date, time, tz)?.validate()
    }
}

impl TryFrom<(NaiveDate, PaceTimeZoneKind)> for PaceDateTime {
    type Error = PaceError;

    fn try_from((date, tz): (NaiveDate, PaceTimeZoneKind)) -> Result<Self, Self::Error> {
        pace_date_time_from_date_and_time_and_tz(date, Local::now().time(), tz)?.validate()
    }
}

impl TryFrom<(NaiveTime, PaceTimeZoneKind)> for PaceDateTime {
    type Error = PaceError;

    fn try_from((time, tz): (NaiveTime, PaceTimeZoneKind)) -> Result<Self, Self::Error> {
        pace_date_time_from_date_and_time_and_tz(Local::now().date_naive(), time, tz)?.validate()
    }
}

impl TryFrom<(NaiveDate, NaiveTime)> for PaceDateTime {
    type Error = PaceError;

    fn try_from((date, time): (NaiveDate, NaiveTime)) -> Result<Self, Self::Error> {
        pace_date_time_from_date_and_time_and_tz(
            date,
            time,
            PaceTimeZoneKind::TimeZoneOffset(*Local::now().offset()),
        )?
        .validate()
    }
}

impl TryFrom<(NaiveDateTime, PaceTimeZoneKind)> for PaceDateTime {
    type Error = PaceError;

    fn try_from((date_time, tz): (NaiveDateTime, PaceTimeZoneKind)) -> Result<Self, Self::Error> {
        pace_date_time_from_date_and_time_and_tz(date_time.date(), date_time.time(), tz)?.validate()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use eyre::{eyre, Result};

    #[test]
    fn test_pace_date_time_is_future_fails() -> Result<()> {
        let future = PaceDateTime::from(
            Local::now().fixed_offset()
                + chrono::TimeDelta::try_days(1).ok_or(eyre!("Invalid time delta."))?,
        );

        assert!(future.validate().is_err());

        Ok(())
    }

    // TODO: Rewrite to PaceDateTime
    // #[test]
    // fn test_parse_time_from_user_input_passes() -> PaceTimeResult<()> {
    //     let time = Some("12:00".to_string());

    //     let result = parse_time_from_user_input(&time)?.ok_or("No time.")?;

    //     assert_eq!(
    //         result,
    //         DateTime<Utc>::new(
    //             Local::now().date_naive(),
    //             NaiveTime::from_hms_opt(12, 0, 0).ok_or(eyre!("Invalid date."))?,
    //         )
    //     );

    //     Ok(())
    // }

    #[test]
    fn test_begin_date_time_new_passes() -> Result<()> {
        _ = PaceDateTime::try_from((
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or(eyre!("Invalid date."))?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or(eyre!("Invalid date."))?,
        ))?
        .validate()?;

        Ok(())
    }

    #[test]
    fn test_begin_date_time_default_passes() -> Result<()> {
        let result = PaceDateTime::default().validate()?;

        assert_eq!(
            result,
            PaceDateTime(Local::now().round_subsecs(0).fixed_offset())
        );

        Ok(())
    }
}
