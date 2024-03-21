pub mod date;
pub mod date_time;
pub mod duration;
pub mod error;
pub mod flags;
pub mod time;
pub mod time_frame;
pub mod time_range;

use std::{
    fmt::{Display, Formatter},
    str::FromStr,
    time::Duration,
};

use chrono::{
    DateTime, Datelike, FixedOffset, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime,
    SubsecRound, TimeZone, Timelike, Utc,
};

use clap::Parser;
use derive_more::{Add, AddAssign, Sub, SubAssign};
use displaydoc::Display;
use getset::{Getters, MutGetters, Setters};
use humantime::format_duration;
use serde_derive::{Deserialize, Serialize};
use tracing::debug;
use typed_builder::TypedBuilder;

use crate::error::{PaceTimeErrorKind, PaceTimeResult};

pub trait Validate {
    type Output;
    type Error;

    /// Validate a struct
    ///
    /// # Errors
    ///
    /// Returns an error if the validation was not successful
    ///
    /// # Returns
    ///
    /// Returns the struct if the validation was successful
    fn validate(self) -> Result<Self::Output, Self::Error>;
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

#[cfg(test)]
mod tests {

    use chrono::NaiveDate;

    use crate::{
        date_time::PaceDateTime,
        duration::{calculate_duration, duration_to_str, PaceDuration},
        flags::{DateFlags, TimeFlags},
        time_frame::PaceTimeFrame,
        time_range::TimeRangeOptions,
    };

    use eyre::Result;

    use super::*;

    #[test]
    fn test_duration_to_str_passes() {
        let initial_time = Local::now();
        let result = duration_to_str(initial_time);
        assert_eq!(result, "just now");
    }

    #[test]
    fn test_pace_date_time_is_future_fails() -> Result<()> {
        let future = Local::now() + chrono::TimeDelta::try_days(1).ok_or("Invalid time delta.")?;
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
    //             NaiveTime::from_hms_opt(12, 0, 0).ok_or("Invalid date.")?,
    //         )
    //     );

    //     Ok(())
    // }

    #[test]
    fn test_calculate_duration_passes() -> Result<()> {
        let begin = PaceDateTime::with_date_time_fixed_offset(DateTime::new(
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
    fn test_calculate_duration_fails() -> Result<()> {
        let begin = PaceDateTime::with_date_time_fixed_offset(DateTime::new(
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
        assert_eq!(result, PaceDuration::new(1));
    }

    #[test]
    fn test_pace_duration_from_chrono_duration_passes() -> Result<()> {
        let duration = chrono::TimeDelta::try_seconds(1).ok_or("Invalid time delta.")?;
        let result = PaceDuration::try_from(duration)?;
        assert_eq!(result, PaceDuration::new(1));

        Ok(())
    }

    #[test]
    fn test_begin_date_time_new_passes() -> Result<()> {
        let time = DateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or("Invalid date.")?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
        );
        let result = PaceDateTime::with_date_time_fixed_offset(time);
        assert_eq!(result, PaceDateTime(time));

        Ok(())
    }

    #[test]
    fn test_begin_date_time_naive_date_time_passes() -> Result<()> {
        let time = DateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or("Invalid date.")?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or("Invalid date.")?,
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

        assert_eq!(result, PaceDuration::new(0));
    }

    #[test]
    fn test_pace_duration_zero_passes() {
        let result = PaceDuration::zero();

        assert_eq!(result, PaceDuration::new(0));
    }

    #[test]
    fn test_pace_duration_add_assign_passes() {
        let mut duration = PaceDuration::new(1);
        duration += PaceDuration::new(1);

        assert_eq!(duration, PaceDuration::new(2));
    }

    #[test]
    fn test_pace_duration_sub_passes() {
        let duration = PaceDuration::new(2) - PaceDuration::new(1);

        assert_eq!(duration, PaceDuration::new(1));
    }

    #[test]
    fn test_pace_duration_sub_assign_passes() {
        let mut duration = PaceDuration::new(2);
        duration -= PaceDuration::new(1);

        assert_eq!(duration, PaceDuration::new(1));
    }

    #[test]
    fn test_pace_duration_sub_assign_with_u64_passes() {
        let mut duration = PaceDuration::new(2);
        duration -= 1;

        assert_eq!(duration, PaceDuration::new(1));
    }

    #[test]
    fn test_pace_duration_sub_assign_below_zero_passes() {
        let mut duration = PaceDuration::new(2);
        duration -= PaceDuration::new(3);

        assert_eq!(duration, PaceDuration::new(0));
    }

    #[test]
    fn test_pace_duration_add_passes() {
        let duration = PaceDuration::new(1) + PaceDuration::new(1);

        assert_eq!(duration, PaceDuration::new(2));
    }

    #[test]
    fn test_pace_duration_from_str_passes() -> Result<()> {
        let duration = "1".parse::<PaceDuration>()?;

        assert_eq!(duration, PaceDuration::new(1));

        Ok(())
    }

    #[test]
    fn test_pace_duration_from_str_fails() {
        let duration = "a".parse::<PaceDuration>();

        assert!(duration.is_err());
    }

    #[test]
    fn test_pace_date_time_is_in_range_options_passes() -> Result<()> {
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
    fn test_pace_date_time_is_in_range_options_fails() -> Result<()> {
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
    fn test_convert_pace_time_frame_date_range_to_time_range_options_passes() -> Result<()> {
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
    fn test_convert_pace_time_frame_specific_date_to_time_range_options_passes() -> Result<()> {
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

    #[test]
    fn test_get_time_frame_from_flags_today_passes() -> Result<()> {
        let time_flags = TimeFlags::builder().today().build();
        let date_flags = DateFlags::default();

        let result = PaceTimeFrame::try_from((&time_flags, &date_flags))?;

        assert_eq!(result, PaceTimeFrame::Today);

        Ok(())
    }
}
