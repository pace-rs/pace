use chrono::{FixedOffset, Local};

use crate::error::PaceTimeErrorKind;

/// Get the local time zone offset to UTC to guess the time zones
///
/// # Returns
///
/// The local time zone offset
#[must_use]
pub fn get_local_time_zone_offset() -> i32 {
    Local::now().offset().local_minus_utc()
}

/// The time zone kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaceTimeZoneKind {
    TimeZone(chrono_tz::Tz),
    TimeZoneOffset(FixedOffset),
    #[default]
    NotSet,
}

impl PaceTimeZoneKind {
    /// Returns `true` if the time zone kind is [`TimeZoneOffset`].
    ///
    /// [`TimeZoneOffset`]: TimeZoneKind::TimeZoneOffset
    #[must_use]
    pub const fn is_time_zone_offset(&self) -> bool {
        matches!(self, Self::TimeZoneOffset(..))
    }

    /// Returns `true` if the time zone kind is [`TimeZone`].
    ///
    /// [`TimeZone`]: TimeZoneKind::TimeZone
    #[must_use]
    pub const fn is_time_zone(&self) -> bool {
        matches!(self, Self::TimeZone(..))
    }

    #[must_use]
    pub const fn as_time_zone(&self) -> Option<&chrono_tz::Tz> {
        if let Self::TimeZone(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_time_zone_offset(&self) -> Option<&FixedOffset> {
        if let Self::TimeZoneOffset(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Try to convert the time zone kind into a time zone offset
    ///
    /// # Errors
    ///
    /// Returns the time zone kind if it is not a time zone offset
    pub const fn try_into_time_zone_offset(self) -> Result<FixedOffset, Self> {
        if let Self::TimeZoneOffset(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }

    /// Try to convert the time zone kind into a time zone
    ///
    /// # Errors
    ///
    /// Returns the time zone kind if it is not a time zone
    pub const fn try_into_time_zone(self) -> Result<chrono_tz::Tz, Self> {
        if let Self::TimeZone(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }

    /// Returns `true` if the time zone kind is [`NotSet`].
    ///
    /// [`NotSet`]: TimeZoneKind::NotSet
    #[must_use]
    pub const fn is_not_set(&self) -> bool {
        matches!(self, Self::NotSet)
    }
}

impl TryFrom<(Option<&chrono_tz::Tz>, Option<&FixedOffset>)> for PaceTimeZoneKind {
    type Error = PaceTimeErrorKind;

    fn try_from(
        (tz, tz_offset): (Option<&chrono_tz::Tz>, Option<&FixedOffset>),
    ) -> Result<Self, Self::Error> {
        match (tz, tz_offset) {
            (Some(tz), None) => Ok(Self::TimeZone(tz.to_owned())),
            (None, Some(tz_offset)) => Ok(Self::TimeZoneOffset(tz_offset.to_owned())),
            (None, None) => Ok(Self::NotSet),
            (Some(_), Some(_)) => Err(PaceTimeErrorKind::AmbiguousTimeZones),
        }
    }
}

impl From<Option<&chrono_tz::Tz>> for PaceTimeZoneKind {
    fn from(tz: Option<&chrono_tz::Tz>) -> Self {
        tz.map_or(Self::NotSet, |tz| Self::TimeZone(tz.to_owned()))
    }
}
