use std::{
    fmt::{Display, Formatter},
    str::FromStr,
    time::Duration,
};

use chrono::{DateTime, Local};

use derive_more::{Add, AddAssign, Sub, SubAssign};
use humantime::format_duration;
use serde_derive::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    date_time::PaceDateTime,
    error::{PaceTimeErrorKind, PaceTimeResult},
};

/// Converts timespec to nice readable relative time string
///
/// # Arguments
///
/// * `initial_time` - The initial time to calculate the relative time from
///
/// # Returns
///
/// A string representing the relative time from the initial time
// TODO: Check if it makes sense to switch this out with `chrono-humanize` crate
#[tracing::instrument]
pub fn duration_to_str(initial_time: DateTime<Local>) -> String {
    let now = Local::now();
    let delta = now.signed_duration_since(initial_time);

    let delta = (
        delta.num_days(),
        delta.num_hours(),
        delta.num_minutes(),
        delta.num_seconds(),
    );

    debug!("Time Delta: {:?}", delta);

    match delta {
        (days, ..) if days > 5 => format!("{}", initial_time.format("%b %d, %Y")),
        (days @ 2..=5, ..) => format!("{days} days ago"),
        (1, ..) => "one day ago".to_string(),

        (_, hours, ..) if hours > 1 => format!("{hours} hours ago"),
        (_, 1, ..) => "an hour ago".to_string(),

        (_, _, minutes, _) if minutes > 1 => format!("{minutes} minutes ago"),
        (_, _, 1, _) => "one minute ago".to_string(),

        (_, _, _, seconds) if seconds > 0 => format!("{seconds} seconds ago"),
        _ => "just now".to_string(),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum PaceDurationRange {
    Short,
    #[default]
    Medium,
    Long,
}

/// The duration of an activity
#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
)]
pub struct PaceDuration(u64);

impl Display for PaceDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", format_duration(Duration::from_secs(self.0)))
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
impl PaceDuration {
    #[must_use]
    pub const fn new(duration: u64) -> Self {
        Self(duration)
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub const fn as_secs(&self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn as_duration(&self) -> Duration {
        Duration::from_secs(self.0)
    }

    #[must_use]
    // We allow this because it's unlikely, that we will reach this case
    #[allow(clippy::cast_precision_loss)]
    pub fn as_minutes(&self) -> f64 {
        self.0 as f64 / 60.0
    }

    #[must_use]
    // We allow this because it's unlikely, that we will reach this case
    #[allow(clippy::cast_precision_loss)]
    pub fn as_hours(&self) -> f64 {
        self.0 as f64 / 3600.0
    }

    #[must_use]
    // We allow this because it's unlikely, that we will reach this case
    #[allow(clippy::cast_precision_loss)]
    pub fn as_days(&self) -> f64 {
        self.0 as f64 / 86400.0
    }

    #[must_use]
    // We allow this because it's unlikely, that we will reach this case
    #[allow(clippy::cast_precision_loss)]
    pub fn as_weeks(&self) -> f64 {
        self.0 as f64 / 604_800.0
    }

    #[must_use]
    pub const fn from_seconds(seconds: u64) -> Self {
        Self(seconds)
    }
}

impl FromStr for PaceDuration {
    type Err = PaceTimeErrorKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u64>().map_or_else(
            |_| Err(PaceTimeErrorKind::ParsingDurationFailed(s.to_string())),
            |duration| Ok(Self(duration)),
        )
    }
}

impl From<Duration> for PaceDuration {
    fn from(duration: Duration) -> Self {
        Self(duration.as_secs())
    }
}

impl TryFrom<chrono::Duration> for PaceDuration {
    type Error = PaceTimeErrorKind;

    fn try_from(duration: chrono::Duration) -> Result<Self, Self::Error> {
        Ok(Self(duration.num_seconds().try_into().map_err(|_| {
            PaceTimeErrorKind::ParsingDurationFailed(duration.to_string())
        })?))
    }
}

impl std::ops::AddAssign<u64> for PaceDuration {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

impl std::ops::SubAssign<u64> for PaceDuration {
    fn sub_assign(&mut self, rhs: u64) {
        self.0 -= rhs;
    }
}

/// Calculate the duration of the activity
///
/// # Arguments
///
/// * `end` - The end date and time of the activity
///
/// # Errors
///
/// Returns an error if the duration can't be calculated or is negative
///
/// # Returns
///
/// Returns the duration of the activity
#[tracing::instrument]
pub fn calculate_duration(begin: &PaceDateTime, end: PaceDateTime) -> PaceTimeResult<PaceDuration> {
    let duration = end.inner().signed_duration_since(begin.inner()).to_std()?;

    debug!("Duration: {:?}", duration);

    Ok(duration.into())
}

#[cfg(test)]
mod tests {

    use super::*;

    use chrono::{NaiveDate, NaiveTime};
    use eyre::{eyre, Result};

    #[test]
    fn test_duration_to_str_passes() {
        let initial_time = Local::now();
        let result = duration_to_str(initial_time);
        assert_eq!(result, "just now");
    }

    #[test]
    fn test_calculate_duration_passes() -> Result<()> {
        let begin = PaceDateTime::try_from((
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or(eyre!("Invalid date."))?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or(eyre!("Invalid date."))?,
        ))?;

        let end = PaceDateTime::try_from((
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or(eyre!("Invalid date."))?,
            NaiveTime::from_hms_opt(0, 0, 1).ok_or(eyre!("Invalid date."))?,
        ))?;

        let duration = calculate_duration(&begin, end.into())?;
        assert_eq!(duration, Duration::from_secs(1).into());

        Ok(())
    }

    #[test]
    fn test_calculate_duration_fails() -> Result<()> {
        let begin = PaceDateTime::try_from((
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or(eyre!("Invalid date."))?,
            NaiveTime::from_hms_opt(0, 0, 1).ok_or(eyre!("Invalid date."))?,
        ))?;

        let end = PaceDateTime::try_from((
            NaiveDate::from_ymd_opt(2021, 1, 1).ok_or(eyre!("Invalid date."))?,
            NaiveTime::from_hms_opt(0, 0, 0).ok_or(eyre!("Invalid date."))?,
        ))?;

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
        let duration = chrono::TimeDelta::try_seconds(1).ok_or(eyre!("Invalid time delta."))?;
        let result = PaceDuration::try_from(duration)?;
        assert_eq!(result, PaceDuration::new(1));

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
}
