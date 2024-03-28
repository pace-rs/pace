use std::str::FromStr;

use chrono::{Local, NaiveDate};

use serde_derive::{Deserialize, Serialize};

use crate::date_time::PaceDateTime;
use pace_error::TimeErrorKind;

/// {0}
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

impl From<PaceDateTime> for PaceDate {
    fn from(date_time: PaceDateTime) -> Self {
        Self(date_time.inner().date_naive())
    }
}

impl From<&PaceDateTime> for PaceDate {
    fn from(date_time: &PaceDateTime) -> Self {
        Self(date_time.inner().date_naive())
    }
}

impl PaceDate {
    #[must_use]
    pub const fn new(date: NaiveDate) -> Self {
        Self(date)
    }

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
    type Err = TimeErrorKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| TimeErrorKind::ParsingDateFailed(format!("Invalid date: {s}")))?;

        Ok(Self(date))
    }
}

impl TryFrom<(i32, u32, u32)> for PaceDate {
    type Error = TimeErrorKind;

    fn try_from((year, month, day): (i32, u32, u32)) -> Result<Self, Self::Error> {
        NaiveDate::from_ymd_opt(year, month, day).map_or_else(
            || Err(TimeErrorKind::InvalidDate(format!("{year}/{month}/{day}"))),
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

#[cfg(test)]
mod tests {

    use chrono::NaiveTime;
    use eyre::{OptionExt, Result};

    use super::*;

    #[test]
    fn test_pace_date_from_str_passes() -> Result<()> {
        let date = PaceDate::from_str("2021-01-01")?;
        assert_eq!(
            date.0,
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or_eyre("Invalid date")?
        );

        Ok(())
    }

    #[test]
    fn test_pace_date_from_str_fails() {
        let date = PaceDate::from_str("2021-01-32");
        assert!(date.is_err());
    }

    #[test]
    fn test_pace_date_try_from_ymd_passes() -> Result<()> {
        let date = PaceDate::try_from((2021, 1, 1))?;
        assert_eq!(
            date.0,
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or_eyre("Invalid date")?
        );

        Ok(())
    }

    #[test]
    fn test_pace_date_try_from_fails() {
        let date = PaceDate::try_from((2021, 1, 32));
        assert!(date.is_err());
    }

    #[test]
    fn test_pace_date_is_future_is_false_passes() -> Result<()> {
        let date = PaceDate::new(NaiveDate::from_ymd_opt(2021, 1, 1).ok_or_eyre("Invalid date")?);
        assert!(!date.is_future());

        Ok(())
    }

    #[test]
    fn test_pace_date_is_future_is_true_passes() -> Result<()> {
        let date = PaceDate::new(NaiveDate::from_ymd_opt(2079, 1, 2).ok_or_eyre("Invalid date")?);
        assert!(date.is_future());

        Ok(())
    }

    #[test]
    fn test_pace_date_with_start_passes() {
        let date = PaceDate::with_start();
        assert_eq!(date.0, NaiveDate::default());
    }

    #[test]
    fn test_pace_date_default_passes() {
        let date = PaceDate::default();
        assert_eq!(date.0, Local::now().naive_local().date());
    }

    #[test]
    fn test_pace_date_from_pace_date_time_passes() -> Result<()> {
        let date_time = PaceDateTime::try_from((
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or_eyre("Invalid date")?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or_eyre("Invalid time")?,
        ))?;

        let date = PaceDate::from(date_time);
        assert_eq!(
            date.0,
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or_eyre("Invalid date")?
        );

        Ok(())
    }

    #[test]
    fn test_pace_date_from_pace_date_time_ref_passes() -> Result<()> {
        let date_time = PaceDateTime::try_from((
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or_eyre("Invalid date")?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or_eyre("Invalid time")?,
        ))?;

        let date = PaceDate::from(&date_time);
        assert_eq!(
            date.0,
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or_eyre("Invalid date")?
        );

        Ok(())
    }

    #[test]
    fn test_pace_date_deref_passes() -> Result<()> {
        let date = PaceDate::new(NaiveDate::from_ymd_opt(2021, 1, 1).ok_or_eyre("Invalid date")?);
        assert_eq!(
            *date,
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or_eyre("Invalid date")?
        );

        Ok(())
    }

    #[test]
    fn test_pace_date_display_passes() -> Result<()> {
        let date = PaceDate::new(NaiveDate::from_ymd_opt(2021, 1, 1).ok_or_eyre("Invalid date")?);
        assert_eq!(format!("{date}"), "2021-01-01");

        Ok(())
    }

    #[test]
    fn test_pace_date_eq_passes() -> Result<()> {
        let date1 = PaceDate::new(NaiveDate::from_ymd_opt(2021, 1, 1).ok_or_eyre("Invalid date")?);
        let date2 = PaceDate::new(NaiveDate::from_ymd_opt(2021, 1, 1).ok_or_eyre("Invalid date")?);
        assert_eq!(date1, date2);

        Ok(())
    }

    #[test]
    fn test_pace_date_ne_passes() -> Result<()> {
        let date1 = PaceDate::new(NaiveDate::from_ymd_opt(2021, 1, 1).ok_or_eyre("Invalid date")?);
        let date2 = PaceDate::new(NaiveDate::from_ymd_opt(2021, 1, 2).ok_or_eyre("Invalid date")?);
        assert_ne!(date1, date2);

        Ok(())
    }
}
