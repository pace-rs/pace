use displaydoc::Display;
use serde_derive::{Deserialize, Serialize};

use crate::{date::PaceDate, time_range::TimeRangeOptions};

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

#[cfg(test)]
mod tests {

    use crate::flags::{DateFlags, TimeFlags};

    use super::*;

    use eyre::Result;

    #[test]
    fn test_get_time_frame_from_flags_today_passes() -> Result<()> {
        let time_flags = TimeFlags::builder().today().build();
        let date_flags = DateFlags::default();

        let result = PaceTimeFrame::try_from((&time_flags, &date_flags))?;

        assert_eq!(result, PaceTimeFrame::Today);

        Ok(())
    }
}
