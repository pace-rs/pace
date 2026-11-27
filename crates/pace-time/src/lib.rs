//! Time handling for pace, on [jiff].
//!
//! - [`TimeFrame`]: the named report periods (`today`, `this-week`, …) and
//!   `--from`/`--to`, resolved to a half-open [`TimeRange`] in the report
//!   time zone.
//! - [`parse_at`]: the `--at` grammar of the tracking commands.
//! - [`parse_duration`]: durations as people type them (`1h30m`).
//! - [`format_duration`], [`relative_time`], [`format_moment`] and
//!   [`format_moment_against`]: durations and moments as pace shows them (`1h 5m`,
//!   `25 minutes ago`, `09:05`).
//!
//! Nothing here reads the clock: every function that needs "now" takes it as
//! an argument, so callers decide which clock to use.
//!
//! ```
//! use jiff::{SignedDuration, ZonedDateTime};
//! use pace_time::{format_duration, format_moment, parse_duration};
//!
//! let duration = parse_duration("1h30m")?;
//! assert_eq!(duration, SignedDuration::from_mins(90));
//! assert_eq!(format_duration(duration), "1h 30m");
//!
//! let now: ZonedDateTime = "2026-11-26T17:00[Europe/Berlin]".parse()?;
//! let began: ZonedDateTime = "2026-11-26T09:05[Europe/Berlin]".parse()?;
//! assert_eq!(format_moment(&began, &now), "09:05");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod at;
mod display;
mod duration;
mod frame;
mod range;

pub use at::{At, NOT_UNDERSTOOD, parse_at};
pub use display::{format_duration, format_moment, format_moment_against, relative_time};
pub use duration::parse_duration;
pub use frame::TimeFrame;
pub use range::TimeRange;

use pace_error::{ErrorKind, PaceError};

/// A jiff error from date arithmetic near the ends of the supported range.
fn out_of_range(error: jiff::Error) -> Box<PaceError> {
    PaceError::with_source(
        ErrorKind::Time,
        "the date lies outside the range pace can calculate with",
        error,
    )
    .attach_hint("give a date closer to today")
}
