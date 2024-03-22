use std::fmt::{Display, Formatter};

use chrono::{
    DateTime, Datelike, FixedOffset, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime,
    SubsecRound, TimeZone, Timelike, Utc,
};

use serde_derive::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    date::PaceDate,
    error::{PaceTimeErrorKind, PaceTimeResult},
    time::PaceTime,
    time_zone::TimeZoneKind,
    Validate,
};

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

/// Wrapper for the start and end time of an activity to implement default
#[derive(Debug, Serialize, Deserialize, Hash, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct PaceDateTime(DateTime<FixedOffset>);

impl TryFrom<(Option<&NaiveTime>, TimeZoneKind, TimeZoneKind)> for PaceDateTime {
    type Error = PaceTimeErrorKind;

    /// Try to convert from a tuple of optional naive time, time zone and time zone offset
    ///
    /// # Arguments
    ///
    /// * `0` - The naive time
    /// * `1` - The time zone
    /// * `2` - The time zone offset
    /// * `3` - The time zone from the config
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
        (naive_time, tz, tz_config): (Option<&NaiveTime>, TimeZoneKind, TimeZoneKind),
    ) -> Result<Self, Self::Error> {
        match (naive_time, tz.as_tz(), tz_config.as_tz()) {
            (None, None, None) => Ok(Self::now()),
            (None, None, Some(tz)) | (None, Some(tz), None | Some(_)) => Ok(Utc::now()
                .with_timezone(&tz)
                .round_subsecs(0)
                .fixed_offset()
                .into()),
            (Some(time), None, None) => pace_date_time_from_date_and_time_and_tz(
                &Local,
                Utc::now().naive_local().date(),
                time.to_owned(),
            ),
            (Some(time), Some(tz), None | Some(_)) | (Some(time), None, Some(tz)) => {
                pace_date_time_from_date_and_time_and_tz(
                    tz,
                    Utc::now().naive_local().date(),
                    time.to_owned(),
                )
            }
        }
    }
}

impl PaceDateTime {
    pub fn new<T>(_date: &NaiveDate, _time: &NaiveTime, _time_zone: &T) -> Self
    where
        T: TimeZone,
    {
        todo!("Implement new for PaceDateTime")
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
}

impl Validate for PaceDateTime {
    type Output = Self;
    type Error = PaceTimeErrorKind;

    /// Check if time is in the future
    ///
    /// # Errors
    ///
    /// Returns an error if the time is in the future
    ///
    /// # Returns
    ///
    /// Returns the time if it's not in the future
    fn validate(self) -> PaceTimeResult<Self> {
        if self > Self::now() {
            Err(PaceTimeErrorKind::StartTimeInFuture(self))
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
    time_zone: TimeZoneKind,
    date: NaiveDate,
) -> PaceTimeResult<PaceDateTime> {
    pace_date_time_from_date_and_time_and_tz(
        time_zone,
        date,
        NaiveTime::from_hms_opt(0, 0, 0)
            .ok_or_else(|| PaceTimeErrorKind::InvalidDate(date.to_string()))?,
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
    tz: TimeZoneKind,
    date: NaiveDate,
    time: NaiveTime,
) -> PaceTimeResult<PaceDateTime> {
    let date_time = match tz {
        TimeZoneKind::TimeZone(ref tz) => {
            let LocalResult::Single(datetime) = tz.with_ymd_and_hms(
                date.year(),
                date.month(),
                date.day(),
                time.hour(),
                time.minute(),
                time.second(),
            ) else {
                return Err(PaceTimeErrorKind::AmbiguousConversionResult);
            };

            PaceDateTime::from(datetime.round_subsecs(0).fixed_offset())
        }
        TimeZoneKind::TimeZoneOffset(ref tz) => {
            let LocalResult::Single(datetime) = tz.from_local_datetime(&date.and_time(time)) else {
                return Err(PaceTimeErrorKind::AmbiguousConversionResult);
            };

            PaceDateTime::from(datetime.round_subsecs(0).fixed_offset())
        }
        TimeZoneKind::NotSet => {
            let LocalResult::Single(datetime) = Local.with_ymd_and_hms(
                date.year(),
                date.month(),
                date.day(),
                time.hour(),
                time.minute(),
                time.second(),
            ) else {
                return Err(PaceTimeErrorKind::AmbiguousConversionResult);
            };

            PaceDateTime::from(datetime.round_subsecs(0).fixed_offset())
        }
    };

    debug!("Constructed date time: {date_time}");

    Ok(date_time.validate()?)
}

impl<Tz: TimeZone> TryFrom<(NaiveDate, NaiveTime, &Tz)> for PaceDateTime {
    type Error = PaceTimeErrorKind;

    fn try_from((date, time, tz): (NaiveDate, NaiveTime, &Tz)) -> Result<Self, Self::Error> {
        pace_date_time_from_date_and_time_and_tz(tz, date, time)?.validate()
    }
}

impl<Tz: TimeZone> TryFrom<(NaiveDate, &Tz)> for PaceDateTime {
    type Error = PaceTimeErrorKind;

    fn try_from((date, tz): (NaiveDate, &Tz)) -> Result<Self, Self::Error> {
        pace_date_time_from_date_and_time_and_tz(tz, date, Local::now().time())?.validate()
    }
}

impl<Tz: TimeZone> TryFrom<(NaiveTime, &Tz)> for PaceDateTime {
    type Error = PaceTimeErrorKind;

    fn try_from((time, tz): (NaiveTime, &Tz)) -> Result<Self, Self::Error> {
        pace_date_time_from_date_and_time_and_tz(tz, Local::now().date_naive(), time)?.validate()
    }
}

impl TryFrom<(NaiveDate, NaiveTime)> for PaceDateTime {
    type Error = PaceTimeErrorKind;

    fn try_from((date, time): (NaiveDate, NaiveTime)) -> Result<Self, Self::Error> {
        pace_date_time_from_date_and_time_and_tz(
            Local::now().offset(),
            Local::now().date_naive(),
            Local::now().time(),
        )?
        .validate()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use eyre::{eyre, Result};

    #[test]
    fn test_pace_date_time_is_future_fails() -> Result<()> {
        let future =
            Local::now() + chrono::TimeDelta::try_days(1).ok_or(eyre!("Invalid time delta."))?;
        let time = PaceDateTime::with_date_time_fixed_offset(future.naive_local());

        let result = time.validate();
        assert!(result.is_err());

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
        let time = DateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or(eyre!("Invalid date."))?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or(eyre!("Invalid date."))?,
        );
        let result = PaceDateTime::with_date_time_fixed_offset(time);
        assert_eq!(result, PaceDateTime(time));

        Ok(())
    }

    #[test]
    fn test_begin_date_time_naive_date_time_passes() -> Result<()> {
        let time = DateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or(eyre!("Invalid date."))?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or(eyre!("Invalid date."))?,
        );
        let begin_date_time = PaceDateTime::with_date_time_fixed_offset(time);

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
    fn test_begin_date_time_from_naive_date_time_passes() -> Result<()> {
        let time = PaceDateTime::from((
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or(eyre!("Invalid date."))?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or(eyre!("Invalid date."))?,
        ));

        let result = PaceDateTime::from(time);

        assert_eq!(result, PaceDateTime(time));

        Ok(())
    }
}
