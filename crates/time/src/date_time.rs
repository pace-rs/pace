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
    flags::{DateFlags, TimeFlags},
    time::PaceTime,
    time_frame::PaceTimeFrame,
    time_range::TimeRangeOptions,
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

impl<Tz: TimeZone> TryFrom<(Option<&NaiveTime>, Option<&Tz>, Option<&String>)> for PaceDateTime {
    type Error = PaceTimeErrorKind;

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
            (Some(_) | None, Some(_), Some(_)) => Err(PaceTimeErrorKind::TzAndTzOffsetDefined),
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

    /// Alias for `Local::now()` and used by `Self::default()`
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

impl<Tz: TimeZone> TryFrom<(&TimeFlags, &DateFlags, Option<&Tz>, Option<&String>)>
    for PaceTimeFrame
{
    type Error = PaceTimeErrorKind;

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

        let construct_with_tz_offset = |date: &NaiveDate| -> PaceTimeResult<PaceDateTime> {
            let Some(fixed_offset) = fixed_offset else {
                return Err(PaceTimeErrorKind::UndefinedTimeZone);
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
        let construct_with_utc = |_date: &NaiveDate| -> PaceTimeResult<PaceDateTime> {
            unimplemented!("Implement conversion from NaiveDate to PaceDateTime")
        };

        let construct_with_tz = |date: &NaiveDate| -> PaceTimeResult<PaceDateTime> {
            construct_pace_date_time(time_zone, date.to_owned())
        };

        #[allow(clippy::type_complexity)]
        let date_time_fn: Box<
            dyn Fn(&NaiveDate) -> Result<PaceDateTime, PaceTimeErrorKind>,
        > = if time_zone_offset.is_some() {
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
) -> PaceTimeResult<PaceDateTime> {
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
) -> PaceTimeResult<DateTime<Tz>>
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
        return Err(PaceTimeErrorKind::InvalidUserInput);
    };

    Ok(datetime)
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
        let time = DateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or(eyre!("Invalid date."))?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or(eyre!("Invalid date."))?,
        );

        let result = PaceDateTime::from(time);

        assert_eq!(result, PaceDateTime(time));

        Ok(())
    }
}
