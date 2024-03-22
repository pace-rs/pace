use chrono::NaiveTime;
use serde_derive::{Deserialize, Serialize};

use crate::date_time::PaceDateTime;

/// Wrapper for a time of an activity
#[derive(Debug, Serialize, Deserialize, Hash, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct PaceTime(pub NaiveTime);

impl From<PaceDateTime> for PaceTime {
    fn from(date_time: PaceDateTime) -> Self {
        Self(date_time.inner().time())
    }
}

impl From<&PaceDateTime> for PaceTime {
    fn from(date_time: &PaceDateTime) -> Self {
        Self(date_time.inner().time())
    }
}

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

#[cfg(test)]
mod tests {

    use chrono::NaiveDate;
    use eyre::{OptionExt, Result};

    use super::*;

    #[test]
    fn test_from_pace_date_time() -> Result<()> {
        let date_time = PaceDateTime::try_from((
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or_eyre("Invalid date.")?,
            NaiveTime::from_hms_opt(12, 0, 0).ok_or_eyre("Invalid time.")?,
        ))?;

        let time = PaceTime::from(date_time);

        assert_eq!(
            time,
            PaceTime(NaiveTime::from_hms_opt(12, 0, 0).ok_or_eyre("Invalid time.")?)
        );

        Ok(())
    }

    #[test]
    fn test_from_pace_date_time_ref() -> Result<()> {
        let date_time = PaceDateTime::try_from((
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or_eyre("Invalid date.")?,
            NaiveTime::from_hms_opt(12, 0, 0).ok_or_eyre("Invalid time.")?,
        ))?;

        let time = PaceTime::from(&date_time);

        assert_eq!(
            time,
            PaceTime(NaiveTime::from_hms_opt(12, 0, 0).ok_or_eyre("Invalid time.")?)
        );

        Ok(())
    }

    #[test]
    fn test_from_naive_time() -> Result<()> {
        let time = PaceTime::from(NaiveTime::from_hms_opt(12, 0, 0).ok_or_eyre("Invalid time.")?);

        assert_eq!(
            time,
            PaceTime(NaiveTime::from_hms_opt(12, 0, 0).ok_or_eyre("Invalid time.")?)
        );

        Ok(())
    }

    #[test]
    fn test_deref() -> Result<()> {
        let time = PaceTime(NaiveTime::from_hms_opt(12, 0, 0).ok_or_eyre("Invalid time.")?);

        assert_eq!(
            *time,
            NaiveTime::from_hms_opt(12, 0, 0).ok_or_eyre("Invalid time.")?
        );

        Ok(())
    }

    #[test]
    fn test_eq() -> Result<()> {
        let time = PaceTime(NaiveTime::from_hms_opt(12, 0, 0).ok_or_eyre("Invalid time.")?);
        let other_time = PaceTime(NaiveTime::from_hms_opt(12, 0, 0).ok_or_eyre("Invalid time.")?);

        assert_eq!(time, other_time);

        Ok(())
    }

    #[test]
    fn test_ord() -> Result<()> {
        let time = PaceTime(NaiveTime::from_hms_opt(12, 0, 0).ok_or_eyre("Invalid time.")?);
        let other_time = PaceTime(NaiveTime::from_hms_opt(12, 1, 0).ok_or_eyre("Invalid time.")?);

        assert!(time < other_time);

        Ok(())
    }
}
