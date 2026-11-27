//! The adapters for the `Clock` and `Ids` ports.
//!
//! The CLI uses the system clock in the configured time zone and random IDs.
//! With `PACE_TEST_NOW` set to an RFC 9557 time, for journey tests, the clock
//! stands still at that moment (seen in the configured zone). IDs stay
//! random, so machines and runs never share one; snapshots show them as
//! numbered placeholders.

use std::{ffi::OsString, sync::Arc};

use jiff::{Timestamp, ZonedDateTime, tz::TimeZone};
use pace_core::{Clock, FixedClock, Ids};
use pace_error::{ErrorKind, PaceError, PaceResult};
use uuid::Uuid;

use crate::config::TimeZoneSetting;

/// The variable that fixes the clock for tests.
pub const TEST_NOW: &str = "PACE_TEST_NOW";

/// The system clock, in whole seconds, in a time zone.
#[derive(Debug, Clone)]
pub struct SystemClock {
    zone: TimeZone,
}

impl SystemClock {
    /// The system clock seen in `zone`.
    #[must_use]
    pub const fn new(zone: TimeZone) -> Self {
        Self { zone }
    }
}

impl Clock for SystemClock {
    fn now(&self) -> ZonedDateTime {
        let now = Timestamp::now();
        Timestamp::from_second(now.as_second())
            .unwrap_or(now)
            .to_zoned_datetime(self.zone.clone())
    }
}

/// Random IDs, each a UUIDv7.
#[derive(Debug, Clone, Copy, Default)]
pub struct RandomIds;

impl Ids for RandomIds {
    fn next_id(&self) -> Uuid {
        Uuid::now_v7()
    }
}

/// The time zone of a setting.
///
/// # Errors
///
/// If the system's zone is unknown, such as with `TZ=Nowhere/Invalid`, or
/// a named zone is unknown, which the config check prevents.
pub fn zone(setting: &TimeZoneSetting) -> PaceResult<TimeZone> {
    match setting {
        TimeZoneSetting::System => TimeZone::try_system().map_err(|error| {
            PaceError::with_source(
                ErrorKind::Time,
                "pace cannot tell the time zone of the system",
                error,
            )
            .attach_hint("set `time-zone` to a name like `Europe/Berlin`")
        }),
        TimeZoneSetting::Named(name) => jiff::tz::db().get(name).map_err(|error| {
            PaceError::with_source(
                ErrorKind::Time,
                "pace doesn't know the time zone {zone}",
                error,
            )
            .attach_context("zone", name.clone())
            .attach_hint("set `time-zone` to a name like `Europe/Berlin`, or `system`")
        }),
    }
}

/// The clock for this run: the system clock, or with `test_now` one that
/// stands still.
///
/// # Errors
///
/// If `PACE_TEST_NOW` is set but no RFC 9557 time, or the zone is unknown.
pub fn clock(setting: &TimeZoneSetting, test_now: Option<OsString>) -> PaceResult<Arc<dyn Clock>> {
    let zone = zone(setting)?;
    let Some(test_now) = test_now.filter(|value| !value.is_empty()) else {
        return Ok(Arc::new(SystemClock::new(zone)));
    };
    let fixed: ZonedDateTime = test_now.to_string_lossy().parse().map_err(|error| {
        PaceError::with_source(
            ErrorKind::Configuration,
            "{variable} is no time like `2026-11-26T09:00[Europe/Berlin]`",
            error,
        )
        .attach_context("variable", TEST_NOW)
        .attach_hint(format!(
            "{TEST_NOW} is for tests; unset it to use the system clock"
        ))
    })?;
    Ok(Arc::new(FixedClock::new(fixed.with_time_zone(zone))))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ported from the old pace: "now" has whole seconds.
    #[test]
    fn the_system_clock_shows_whole_seconds_in_its_zone() -> Result<(), Box<dyn std::error::Error>>
    {
        let zone = TimeZone::get("Asia/Tokyo")?;
        let now = SystemClock::new(zone.clone()).now();

        assert_eq!(now.subsec_nanosecond(), 0);
        assert_eq!(now.time_zone(), &zone);
        Ok(())
    }

    #[test]
    fn test_time_is_seen_in_the_configured_zone() -> Result<(), Box<dyn std::error::Error>> {
        let clock = clock(
            &TimeZoneSetting::Named("America/New_York".to_owned()),
            Some("2026-11-26T09:00[Europe/Berlin]".into()),
        )?;

        assert_eq!(
            clock.now().to_string(),
            "2026-11-26T03:00:00-05:00[America/New_York]"
        );
        Ok(())
    }
}
