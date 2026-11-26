//! Time handling for pace, on [jiff].
//!
//! - [`TimeFrame`]: the named report periods (`today`, `this-week`, …) and
//!   `--from`/`--to`, resolved to a half-open [`TimeRange`] in the report
//!   time zone.
//! - [`elapsed`] and [`parse_duration`]: elapsed time between two moments,
//!   and durations as people type them (`1h30m`).
//!
//! Nothing here reads the clock: every function that needs "now" takes it as
//! an argument, so callers decide which clock to use.

mod duration;
mod frame;
mod range;

pub use duration::{elapsed, parse_duration};
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
    .attach_hint("use a date between the years -9999 and 9999")
}
