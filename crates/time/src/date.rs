use std::str::FromStr;

use chrono::{Local, NaiveDate};

use serde_derive::{Deserialize, Serialize};

use crate::{date_time::PaceDateTime, error::PaceTimeErrorKind};

/// `PaceDate`: {0}
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
    type Err = PaceTimeErrorKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| PaceTimeErrorKind::ParsingDateFailed(format!("Invalid date: {s}")))?;

        Ok(Self(date))
    }
}

impl TryFrom<(i32, u32, u32)> for PaceDate {
    type Error = PaceTimeErrorKind;

    fn try_from((year, month, day): (i32, u32, u32)) -> Result<Self, Self::Error> {
        NaiveDate::from_ymd_opt(year, month, day).map_or_else(
            || {
                Err(PaceTimeErrorKind::InvalidDate(format!(
                    "{year}/{month}/{day}"
                )))
            },
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
