//! The time between two moments, and durations as people type them.

use jiff::{SignedDuration, ZonedDateTime};
use pace_error::{ErrorKind, PaceError, PaceResult};

/// The time that passed from `start` to `end`, whatever zones they were
/// recorded in.
///
/// # Errors
///
/// If `end` lies before `start`.
pub fn elapsed(start: &ZonedDateTime, end: &ZonedDateTime) -> PaceResult<SignedDuration> {
    if end < start {
        return Err(PaceError::new(
            ErrorKind::InvalidInput,
            "the end {end} lies before the start {start}",
        )
        .attach_hint("give an end after the start")
        .attach_context("start", start.to_string())
        .attach_context("end", end.to_string()));
    }
    Ok(start.duration_until(end))
}

/// A duration as people type it: `15m`, `1h30m`, `1h 30m`, `1.5h`, `45s`.
/// Space around it does not count.
///
/// # Errors
///
/// If the input is no duration, a number without a unit, or a duration
/// that is not positive, such as `0m` or `30m ago`.
pub fn parse_duration(input: &str) -> PaceResult<SignedDuration> {
    let trimmed = input.trim();
    let duration = trimmed.parse::<SignedDuration>().map_err(|error| {
        let hint = if !trimmed.is_empty() && trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
            format!("add a unit, for example `{trimmed}m` or `{trimmed}s`")
        } else {
            "write it like `15m`, `1h30m` or `1.5h`".to_owned()
        };
        PaceError::with_source(
            ErrorKind::InvalidInput,
            "`{input}` is not a duration",
            error,
        )
        .attach_context("input", input)
        .attach_hint(hint)
    })?;

    if !duration.is_positive() {
        let hint = match duration.checked_neg() {
            Some(back) if back.is_positive() => format!("did you mean `{back:#}`?"),
            _ => "give a duration longer than zero, like `15m`".to_owned(),
        };
        return Err(PaceError::new(
            ErrorKind::InvalidInput,
            "the duration `{input}` is not positive",
        )
        .attach_context("input", input)
        .attach_hint(hint));
    }
    Ok(duration)
}
