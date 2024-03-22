use chrono::{FixedOffset, Local, NaiveDate, TimeZone};
use displaydoc::Display;
use serde_derive::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    date::PaceDate,
    date_time::{pace_date_time_from_date_and_tz_with_zero_hms, PaceDateTime},
    error::{PaceTimeErrorKind, PaceTimeResult},
    flags::{DateFlags, TimeFlags},
    time_range::TimeRangeOptions,
    time_zone::TimeZoneKind,
};

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

impl TryFrom<(&TimeFlags, &DateFlags, TimeZoneKind, TimeZoneKind)> for PaceTimeFrame {
    type Error = PaceTimeErrorKind;

    fn try_from(
        (time_flags, date_flags, tz, tz_config): (
            &TimeFlags,
            &DateFlags,
            TimeZoneKind,
            TimeZoneKind,
        ),
    ) -> Result<Self, Self::Error> {
        time_frame_from_date_and_time_flags_with_time_zone_kind(
            time_flags, date_flags, tz, tz_config,
        )
    }
}

/// Get the time zone aware time frame from the time and date flags
///
/// # Arguments
///
/// * `time_zone_offset` - The time zone offset
/// * `tz_user` - The time zone of the user
/// * `tz_config` - The time zone from the configuration
/// * `time_flags` - The time flags
/// * `date_flags` - The date flags
///
/// # Errors
///
/// Returns an error if the time frame could not be created
///
/// # Returns
///
/// Returns the time frame
pub(crate) fn time_frame_from_date_and_time_flags_with_time_zone_kind(
    time_flags: &TimeFlags,
    date_flags: &DateFlags,
    tz: TimeZoneKind,
    tz_config: TimeZoneKind,
) -> Result<PaceTimeFrame, PaceTimeErrorKind> {
    let time_zone = match (tz.as_tz(), tz_config.as_tz()) {
        (None, Some(tz)) | (Some(tz), None) => tz,
        (Some(tz), Some(_)) => tz,
        (None, None) => Local as impl TimeZone,
    };

    let time_frame = match (time_flags, date_flags) {
        (TimeFlags::Today, _) => PaceTimeFrame::Today,
        (TimeFlags::Yesterday, _) => PaceTimeFrame::Yesterday,
        (TimeFlags::CurrentWeek, _) => PaceTimeFrame::CurrentWeek,
        (TimeFlags::LastWeek, _) => PaceTimeFrame::LastWeek,
        (TimeFlags::CurrentMonth, _) => PaceTimeFrame::CurrentMonth,
        (TimeFlags::LastMonth, _) => PaceTimeFrame::LastMonth,
        (
            _,
            DateFlags {
                date: Some(specific_date),
                from: None,
                to: None,
            },
        ) => {
            // We have a specific date, but no date range
            PaceTimeFrame::SpecificDate(PaceDate::from(specific_date.to_owned()))
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
            PaceTimeFrame::DateRange(
                TimeRangeOptions::builder()
                    .start(PaceDateTime::try_from((from, time_zone))?)
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
            PaceTimeFrame::DateRange(
                TimeRangeOptions::builder()
                    .end(PaceDateTime::try_from((to, time_zone))?)
                    .build(),
            )
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
            PaceTimeFrame::DateRange(
                TimeRangeOptions::builder()
                    .start(PaceDateTime::try_from((from, time_zone))?)
                    .end(PaceDateTime::try_from((to, time_zone))?)
                    .build(),
            )
        }
        _ => PaceTimeFrame::default(),
    };

    debug!("Time frame: {:?}", time_frame);

    Ok(time_frame)
}

#[cfg(test)]
mod tests {

    use crate::flags::{DateFlags, TimeFlags};

    use super::*;

    use eyre::Result;

    #[test]
    fn test_get_time_frame_from_flags_today_passes() -> Result<()> {
        let time_flags = TimeFlags::Today;
        let date_flags = DateFlags::default();

        let result = PaceTimeFrame::try_from((&time_flags, &date_flags))?;

        assert_eq!(result, PaceTimeFrame::Today);

        Ok(())
    }
}
