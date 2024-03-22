use displaydoc::Display;
use serde_derive::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    date::PaceDate,
    date_time::PaceDateTime,
    error::PaceTimeErrorKind,
    flags::{DateFlags, TimeFlags},
    time_range::TimeRangeOptions,
    time_zone::PaceTimeZoneKind,
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

impl
    TryFrom<(
        Option<&TimeFlags>,
        Option<&DateFlags>,
        PaceTimeZoneKind,
        PaceTimeZoneKind,
    )> for PaceTimeFrame
{
    type Error = PaceTimeErrorKind;

    fn try_from(
        (time_flags, date_flags, tz, tz_config): (
            Option<&TimeFlags>,
            Option<&DateFlags>,
            PaceTimeZoneKind,
            PaceTimeZoneKind,
        ),
    ) -> Result<Self, Self::Error> {
        time_frame_from_date_and_time_flags_with_time_zone_kind(
            time_flags, date_flags, tz, tz_config,
        )
    }
}

impl TryFrom<(Option<&TimeFlags>, Option<&DateFlags>, PaceTimeZoneKind)> for PaceTimeFrame {
    type Error = PaceTimeErrorKind;

    fn try_from(
        (time_flags, date_flags, tz): (Option<&TimeFlags>, Option<&DateFlags>, PaceTimeZoneKind),
    ) -> Result<Self, Self::Error> {
        time_frame_from_date_and_time_flags_with_time_zone_kind(
            time_flags,
            date_flags,
            tz,
            PaceTimeZoneKind::NotSet,
        )
    }
}

impl TryFrom<(&DateFlags, PaceTimeZoneKind)> for PaceTimeFrame {
    type Error = PaceTimeErrorKind;

    fn try_from((date_flags, tz): (&DateFlags, PaceTimeZoneKind)) -> Result<Self, Self::Error> {
        time_frame_from_date_and_time_flags_with_time_zone_kind(
            None,
            Some(date_flags),
            tz,
            PaceTimeZoneKind::NotSet,
        )
    }
}

impl TryFrom<&DateFlags> for PaceTimeFrame {
    type Error = PaceTimeErrorKind;

    fn try_from(date_flags: &DateFlags) -> Result<Self, Self::Error> {
        time_frame_from_date_and_time_flags_with_time_zone_kind(
            None,
            Some(date_flags),
            PaceTimeZoneKind::NotSet,
            PaceTimeZoneKind::NotSet,
        )
    }
}

impl TryFrom<(Option<&DateFlags>, PaceTimeZoneKind)> for PaceTimeFrame {
    type Error = PaceTimeErrorKind;

    fn try_from(
        (date_flags, tz): (Option<&DateFlags>, PaceTimeZoneKind),
    ) -> Result<Self, Self::Error> {
        time_frame_from_date_and_time_flags_with_time_zone_kind(
            None,
            date_flags,
            tz,
            PaceTimeZoneKind::NotSet,
        )
    }
}

impl TryFrom<(Option<&TimeFlags>, PaceTimeZoneKind)> for PaceTimeFrame {
    type Error = PaceTimeErrorKind;

    fn try_from(
        (time_flags, tz): (Option<&TimeFlags>, PaceTimeZoneKind),
    ) -> Result<Self, Self::Error> {
        time_frame_from_date_and_time_flags_with_time_zone_kind(
            time_flags,
            None,
            tz,
            PaceTimeZoneKind::NotSet,
        )
    }
}

impl TryFrom<(&TimeFlags, PaceTimeZoneKind)> for PaceTimeFrame {
    type Error = PaceTimeErrorKind;

    fn try_from((time_flags, tz): (&TimeFlags, PaceTimeZoneKind)) -> Result<Self, Self::Error> {
        time_frame_from_date_and_time_flags_with_time_zone_kind(
            Some(time_flags),
            None,
            tz,
            PaceTimeZoneKind::NotSet,
        )
    }
}

impl TryFrom<&TimeFlags> for PaceTimeFrame {
    type Error = PaceTimeErrorKind;

    fn try_from(time_flags: &TimeFlags) -> Result<Self, Self::Error> {
        time_frame_from_date_and_time_flags_with_time_zone_kind(
            Some(time_flags),
            None,
            PaceTimeZoneKind::NotSet,
            PaceTimeZoneKind::NotSet,
        )
    }
}

impl TryFrom<PaceTimeZoneKind> for PaceTimeFrame {
    type Error = PaceTimeErrorKind;

    fn try_from(tz: PaceTimeZoneKind) -> Result<Self, Self::Error> {
        time_frame_from_date_and_time_flags_with_time_zone_kind(
            Some(&TimeFlags::default()),
            None,
            tz,
            PaceTimeZoneKind::NotSet,
        )
    }
}

/// Get the time zone aware time frame from the time and date flags
///
/// # Arguments
///
/// * `time_flags` - The time flags
/// * `date_flags` - The date flags
/// * `tz_user` - The time zone kind from the user
/// * `tz_config` - The time zone kind from the configuration
///
/// # Errors
///
/// Returns an error if the time frame could not be created
///
/// # Returns
///
/// Returns the time frame
pub(crate) fn time_frame_from_date_and_time_flags_with_time_zone_kind(
    time_flags: Option<&TimeFlags>,
    date_flags: Option<&DateFlags>,
    tz: PaceTimeZoneKind,
    tz_config: PaceTimeZoneKind,
) -> Result<PaceTimeFrame, PaceTimeErrorKind> {
    let time_zone = match (tz, tz_config) {
        (tzk, _) | (PaceTimeZoneKind::NotSet, tzk) if !tzk.is_not_set() => tzk,
        _ => PaceTimeZoneKind::default(),
    };

    debug!("Using Time zone: {time_zone:?}");

    let time_frame = match (time_flags, date_flags) {
        (Some(TimeFlags::Today), _) => PaceTimeFrame::Today,
        (Some(TimeFlags::Yesterday), _) => PaceTimeFrame::Yesterday,
        (Some(TimeFlags::CurrentWeek), _) => PaceTimeFrame::CurrentWeek,
        (Some(TimeFlags::LastWeek), _) => PaceTimeFrame::LastWeek,
        (Some(TimeFlags::CurrentMonth), _) => PaceTimeFrame::CurrentMonth,
        (Some(TimeFlags::LastMonth), _) => PaceTimeFrame::LastMonth,
        (
            None,
            Some(DateFlags {
                date: Some(specific_date),
                from: None,
                to: None,
            }),
        ) => {
            // We have a specific date, but no date range
            PaceTimeFrame::SpecificDate(PaceDate::from(specific_date.to_owned()))
        }
        (
            None,
            Some(DateFlags {
                date: None,
                from: Some(from),
                to: None,
            }),
        ) => {
            // We have a from date, but no end date
            PaceTimeFrame::DateRange(
                TimeRangeOptions::builder()
                    .start(PaceDateTime::try_from((from.to_owned(), time_zone))?.start_of_day()?)
                    .build(),
            )
        }
        (
            None,
            Some(DateFlags {
                date: None,
                from: None,
                to: Some(to),
            }),
        ) => {
            // We have an end date, but no start date
            PaceTimeFrame::DateRange(
                TimeRangeOptions::builder()
                    .end(PaceDateTime::try_from((to.to_owned(), time_zone))?.end_of_day()?)
                    .build(),
            )
        }
        (
            None,
            Some(DateFlags {
                date: None,
                from: Some(from),
                to: Some(to),
            }),
        ) => {
            // We have a date range
            PaceTimeFrame::DateRange(
                TimeRangeOptions::builder()
                    .start(PaceDateTime::try_from((from.to_owned(), time_zone))?.start_of_day()?)
                    .end(PaceDateTime::try_from((to.to_owned(), time_zone))?.end_of_day()?)
                    .build(),
            )
        }
        _ => PaceTimeFrame::default(),
    };

    debug!("Converted Time frame: {:?}", time_frame);

    Ok(time_frame)
}

#[cfg(test)]
mod tests {

    use crate::flags::TimeFlags;

    use super::*;

    use eyre::Result;

    #[test]
    fn test_get_time_frame_from_flags_today_passes() -> Result<()> {
        let time_flags = TimeFlags::Today;

        let result = PaceTimeFrame::try_from(&time_flags)?;

        assert_eq!(result, PaceTimeFrame::Today);

        Ok(())
    }
}
