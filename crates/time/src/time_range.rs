use std::fmt::{Display, Formatter};

use chrono::{Datelike, Local, NaiveDate};

use getset::Getters;
use serde_derive::{Deserialize, Serialize};
use tracing::debug;
use typed_builder::TypedBuilder;

use crate::{
    date::PaceDate,
    date_time::PaceDateTime,
    error::{PaceTimeErrorKind, PaceTimeResult},
    time_frame::PaceTimeFrame,
    Validate,
};

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
    type Error = PaceTimeErrorKind;

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
    type Error = PaceTimeErrorKind;

    /// Validate the time range
    ///
    /// # Errors
    ///
    /// Returns an error if the time range is invalid
    ///
    /// # Returns
    ///
    /// Returns the time range options if they are valid
    fn validate(self) -> PaceTimeResult<Self> {
        if self.start > self.end {
            return Err(PaceTimeErrorKind::InvalidTimeRange(
                self.start.to_string(),
                self.end.to_string(),
            ));
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
    pub fn current_month() -> PaceTimeResult<Self> {
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
    pub fn current_week() -> PaceTimeResult<Self> {
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
    pub fn current_year() -> PaceTimeResult<Self> {
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
    pub fn specific_date(date: PaceDate) -> PaceTimeResult<Self> {
        // handle date if it's in the future
        let (start, end) = if date.is_future() {
            debug!("Date is in the future, using today.");
            (
                PaceDateTime::from(
                    PaceDate::default()
                        .inner()
                        .and_hms_opt(0, 0, 0)
                        .ok_or_else(|| PaceTimeErrorKind::InvalidDate(date.to_string()))?,
                ),
                PaceDateTime::now(),
            )
        } else {
            (
                PaceDateTime::from(
                    date.inner()
                        .and_hms_opt(0, 0, 0)
                        .ok_or_else(|| PaceTimeErrorKind::InvalidDate(date.to_string()))?,
                ),
                PaceDateTime::from(
                    date.inner()
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
    pub fn last_month() -> PaceTimeResult<Self> {
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
    pub fn last_week() -> PaceTimeResult<Self> {
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
    pub fn last_year() -> PaceTimeResult<Self> {
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
    pub fn today() -> PaceTimeResult<Self> {
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
    pub fn yesterday() -> PaceTimeResult<Self> {
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

#[cfg(test)]
mod tests {

    use super::*;

    use chrono::{DateTime, NaiveTime};
    use eyre::{eyre, Result};

    #[test]
    fn test_pace_date_time_is_in_range_options_passes() -> Result<()> {
        let activity_date_time = PaceDateTime::from(DateTime::new(
            NaiveDate::from_ymd_opt(2021, 2, 3).ok_or(eyre!("Invalid date."))?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or(eyre!("Invalid date."))?,
        ));

        let time_range = TimeRangeOptions::builder()
            .start(PaceDateTime::from(DateTime::new(
                NaiveDate::from_ymd_opt(2021, 2, 2).ok_or(eyre!("Invalid date."))?,
                NaiveTime::from_hms_opt(0, 0, 0).ok_or(eyre!("Invalid date."))?,
            )))
            .end(PaceDateTime::from(DateTime::new(
                NaiveDate::from_ymd_opt(2021, 2, 4).ok_or(eyre!("Invalid date."))?,
                NaiveTime::from_hms_opt(0, 0, 0).ok_or(eyre!("Invalid date."))?,
            )))
            .build();

        assert!(time_range.is_in_range(activity_date_time));

        Ok(())
    }

    #[test]
    fn test_pace_date_time_is_in_range_options_fails() -> Result<()> {
        assert!(TimeRangeOptions::builder()
            .start(PaceDateTime::from(DateTime<Utc>::new(
                NaiveDate::from_ymd_opt(2021, 2, 4).ok_or(eyre!("Invalid date."))?,
                NaiveTime::from_hms_opt(0, 0, 0).ok_or(eyre!("Invalid date."))?,
            )))
            .end(PaceDateTime::from(DateTime<Utc>::new(
                NaiveDate::from_ymd_opt(2021, 2, 2).ok_or(eyre!("Invalid date."))?,
                NaiveTime::from_hms_opt(0, 0, 0).ok_or(eyre!("Invalid date."))?,
            )))
            .build()
            .validate()
            .is_err());

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_date_range_to_time_range_options_passes() -> Result<()> {
        let time_frame = PaceTimeFrame::DateRange(
            TimeRangeOptions::builder()
                .start(PaceDateTime::from(
                    Local::new(
                        NaiveDate::from_ymd_opt(2021, 2, 2).ok_or(eyre!("Invalid date."))?,
                        NaiveTime::from_hms_opt(0, 0, 0).ok_or(eyre!("Invalid date."))?,
                    )
                    .with_timezone(&Utc),
                ))
                .end(PaceDateTime::from(
                    Local::new(
                        NaiveDate::from_ymd_opt(2021, 2, 4).ok_or(eyre!("Invalid date."))?,
                        NaiveTime::from_hms_opt(0, 0, 0).ok_or(eyre!("Invalid date."))?,
                    )
                    .with_timezone(&Utc),
                ))
                .build(),
        );

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::builder()
                .start(PaceDateTime::from(DateTime<Utc>::new(
                    NaiveDate::from_ymd_opt(2021, 2, 2).ok_or(eyre!("Invalid date."))?,
                    NaiveTime::from_hms_opt(0, 0, 0).ok_or(eyre!("Invalid date."))?,
                )))
                .end(PaceDateTime::from(DateTime<Utc>::new(
                    NaiveDate::from_ymd_opt(2021, 2, 4).ok_or(eyre!("Invalid date."))?,
                    NaiveTime::from_hms_opt(0, 0, 0).ok_or(eyre!("Invalid date."))?,
                )))
                .build()
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_specific_date_to_time_range_options_passes() -> Result<()> {
        let time_frame = PaceTimeFrame::SpecificDate(PaceDate::new(
            NaiveDate::from_ymd_opt(2021, 2, 2).ok_or(eyre!("Invalid date."))?,
        ));

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::builder()
                .start(PaceDateTime::from(DateTime<Utc>::new(
                    NaiveDate::from_ymd_opt(2021, 2, 2).ok_or(eyre!("Invalid date."))?,
                    NaiveTime::from_hms_opt(0, 0, 0).ok_or(eyre!("Invalid date."))?,
                )))
                .end(PaceDateTime::from(DateTime<Utc>::new(
                    NaiveDate::from_ymd_opt(2021, 2, 2).ok_or(eyre!("Invalid date."))?,
                    NaiveTime::from_hms_opt(23, 59, 59).ok_or(eyre!("Invalid date."))?,
                )))
                .build()
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_current_month_to_time_range_options_passes() -> Result<()> {
        let time_frame = PaceTimeFrame::CurrentMonth;

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::current_month()?
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_current_week_to_time_range_options_passes() -> Result<()> {
        let time_frame = PaceTimeFrame::CurrentWeek;

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::current_week()?
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_current_year_to_time_range_options_passes() -> Result<()> {
        let time_frame = PaceTimeFrame::CurrentYear;

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::current_year()?
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_last_month_to_time_range_options_passes() -> Result<()> {
        let time_frame = PaceTimeFrame::LastMonth;

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::last_month()?
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_last_week_to_time_range_options_passes() -> Result<()> {
        let time_frame = PaceTimeFrame::LastWeek;

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::last_week()?
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_last_year_to_time_range_options_passes() -> Result<()> {
        let time_frame = PaceTimeFrame::LastYear;

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::last_year()?
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_today_to_time_range_options_passes() -> Result<()> {
        let time_frame = PaceTimeFrame::Today;

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::today()?
        );

        Ok(())
    }

    #[test]
    fn test_convert_pace_time_frame_yesterday_to_time_range_options_passes() -> Result<()> {
        let time_frame = PaceTimeFrame::Yesterday;

        assert_eq!(
            TimeRangeOptions::try_from(time_frame)?,
            TimeRangeOptions::yesterday()?
        );

        Ok(())
    }
}
