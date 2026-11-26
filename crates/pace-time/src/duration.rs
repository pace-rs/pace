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
///
/// # Errors
///
/// If the input is no duration, or a number without a unit.
pub fn parse_duration(input: &str) -> PaceResult<SignedDuration> {
    input.parse::<SignedDuration>().map_err(|error| {
        let trimmed = input.trim();
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
    })
}
