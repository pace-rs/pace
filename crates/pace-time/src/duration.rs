//! Durations as people type them.

use jiff::SignedDuration;
use pace_error::{ErrorKind, PaceError, PaceResult};

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
