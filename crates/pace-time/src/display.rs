//! Durations and moments as pace shows them: `1h 5m`, `25 minutes ago`.

use jiff::{SignedDuration, ZonedDateTime};

/// A duration in hours and minutes, as pace shows it: `45s`, `5m`, `1h`,
/// `1h 5m`, `26h 3m`. Seconds show only below a minute; longer durations
/// drop them, and stay in hours past a day.
#[must_use]
pub fn format_duration(duration: SignedDuration) -> String {
    let sign = if duration.is_negative() { "-" } else { "" };
    let seconds = duration.as_secs().unsigned_abs();
    if seconds < 60 {
        return format!("{sign}{seconds}s");
    }
    match (seconds / 3_600, seconds % 3_600 / 60) {
        (0, minutes) => format!("{sign}{minutes}m"),
        (hours, 0) => format!("{sign}{hours}h"),
        (hours, minutes) => format!("{sign}{hours}h {minutes}m"),
    }
}

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;

/// A moment relative to `now`: `just now`, `25 minutes ago`, `5 days ago`.
///
/// Anything older than five days shows its date in the zone of `now`
/// (`on 2026-11-20`). A moment ahead of now, recorded by a device whose
/// clock runs ahead, reads `in 10 minutes`.
#[must_use]
pub fn relative_time(then: &ZonedDateTime, now: &ZonedDateTime) -> String {
    let delta = then.duration_until(now);
    let seconds = delta.as_secs().unsigned_abs();
    let amount = |count: u64, unit: &str| {
        if count == 1 {
            format!("1 {unit}")
        } else {
            format!("{count} {unit}s")
        }
    };
    let span = match seconds {
        0..MINUTE => return "just now".to_owned(),
        MINUTE..HOUR => amount(seconds / MINUTE, "minute"),
        HOUR..DAY => amount(seconds / HOUR, "hour"),
        _ if seconds <= 5 * DAY => amount(seconds / DAY, "day"),
        _ => {
            let date = then.with_time_zone(now.time_zone().clone()).date();
            return format!("on {date}");
        }
    };
    if delta.is_negative() {
        format!("in {span}")
    } else {
        format!("{span} ago")
    }
}
