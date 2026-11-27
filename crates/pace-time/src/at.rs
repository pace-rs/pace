//! The `--at` grammar: `09:30`, `yesterday 17:00`, `2026-11-26 09:00`,
//! `-15m`.

use std::fmt;

use jiff::{
    SignedDuration, ZonedDateTime,
    civil::{Date, DateTime, Time},
    fmt::friendly::SpanParser,
    tz::AmbiguousOffset,
};
use pace_error::{ErrorKind, PaceError, PaceResult};

use crate::out_of_range;

/// A moment given with `--at`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum At {
    /// The moment.
    Exact(ZonedDateTime),
    /// A local time that happened twice because the clocks were set back.
    /// Ask which one was meant; without a terminal, take the earlier one.
    Ambiguous {
        /// The first time the clock showed the local time.
        earlier: ZonedDateTime,
        /// The second time the clock showed the local time.
        later: ZonedDateTime,
    },
}

impl At {
    /// The moment, or the earlier one if the local time happened twice.
    #[must_use]
    pub fn earlier(self) -> ZonedDateTime {
        match self {
            Self::Exact(moment)
            | Self::Ambiguous {
                earlier: moment, ..
            } => moment,
        }
    }
}

/// Read a `--at` time, seen from `now` and in the time zone of `now`.
///
/// - `HH:MM` is the most recent such time: `23:30` typed at 00:15 means
///   yesterday 23:30. `HH:MM:SS` adds the seconds.
/// - `yesterday HH:MM` and `YYYY-MM-DD HH:MM` name the day.
/// - A relative time goes back from now in whole seconds: `-15m`,
///   `-1h30m`, `15m ago`.
///
/// A local time skipped by a clock change moves forward by the length of the
/// gap. A local time that happened twice is [`At::Ambiguous`] if both lie in
/// the past.
///
/// # Errors
///
/// If the input follows none of the forms above, or names a time in the
/// future: pace records what happened.
pub fn parse_at(input: &str, now: &ZonedDateTime) -> PaceResult<At> {
    let text = input.trim().to_lowercase();

    if let Some(time) = text
        .strip_prefix("yesterday")
        .filter(|rest| rest.starts_with(char::is_whitespace))
        .and_then(|rest| clock_time(rest.trim_start()))
    {
        let yesterday = now.date().yesterday().map_err(out_of_range)?;
        return in_the_past(yesterday.to_datetime(time), now);
    }

    if let Some(time) = clock_time(&text) {
        let today = in_the_past(now.date().to_datetime(time), now);
        if today.is_ok() {
            return today;
        }
        let yesterday = now.date().yesterday().map_err(out_of_range)?;
        return in_the_past(yesterday.to_datetime(time), now);
    }

    if let Some(datetime) = date_and_time(&text) {
        return in_the_past(datetime, now);
    }

    if let Some(duration) = duration_back_from_now(&text) {
        return back_from_now(input, duration, now);
    }

    Err(no_time(input))
}

/// `H:MM` or `HH:MM`, with seconds if they follow: `HH:MM:SS`.
fn clock_time(text: &str) -> Option<Time> {
    let mut parts = text.split(':');
    let (hour, minute, second) = (parts.next()?, parts.next()?, parts.next());
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    let two_digits = |part: &str| digits(part) && part.len() == 2;
    if parts.next().is_some()
        || !(digits(hour) && hour.len() <= 2 && two_digits(minute))
        || !second.is_none_or(two_digits)
    {
        return None;
    }
    let second = second.map_or(Some(0), |second| second.parse().ok())?;
    Time::new(hour.parse().ok()?, minute.parse().ok()?, second, 0).ok()
}

/// A time back from now in jiff's friendly format: `-15m`, `-1h30m`,
/// `15m ago`. A text with `:` is a clock time, never a duration, and the ISO
/// 8601 format (`PT15M`) is no part of pace's grammar.
fn duration_back_from_now(text: &str) -> Option<SignedDuration> {
    const FRIENDLY: SpanParser = SpanParser::new();
    if text.contains(':') {
        return None;
    }
    FRIENDLY.parse_duration(text).ok()
}

/// `YYYY-MM-DD HH:MM` or `YYYY-MM-DDTHH:MM`, with seconds if they follow.
fn date_and_time(text: &str) -> Option<DateTime> {
    let (date, time) = text.split_once([' ', 't'])?;
    Some(date.parse::<Date>().ok()?.to_datetime(clock_time(time)?))
}

/// The local `datetime` in the zone of `now`, if it lies in the past.
fn in_the_past(datetime: DateTime, now: &ZonedDateTime) -> PaceResult<At> {
    let (first, second) = moments(datetime, now).map_err(|error| {
        // jiff stops a day before the end of its civil dates, and such a
        // time lies in the future.
        if datetime > now.datetime() {
            in_the_future(datetime, now)
        } else {
            out_of_range(error)
        }
    })?;

    let passed = |moment: &ZonedDateTime| moment <= now;
    match second {
        Some(later) if passed(&later) => Ok(At::Ambiguous {
            earlier: first,
            later,
        }),
        _ if passed(&first) => Ok(At::Exact(first)),
        _ => Err(in_the_future(&first, now)),
    }
}

/// The moments at which the clock in the zone of `now` showed `datetime`:
/// the first, and the second if the clocks were set back over it.
fn moments(
    datetime: DateTime,
    now: &ZonedDateTime,
) -> Result<(ZonedDateTime, Option<ZonedDateTime>), jiff::Error> {
    let ambiguous = now.time_zone().to_ambiguous_zoned_datetime(datetime);
    match ambiguous.offset() {
        AmbiguousOffset::Fold { .. } => {
            Ok((ambiguous.clone().earlier()?, Some(ambiguous.later()?)))
        }
        AmbiguousOffset::Unambiguous { .. } | AmbiguousOffset::Gap { .. } => {
            Ok((ambiguous.compatible()?, None))
        }
    }
}

fn back_from_now(input: &str, duration: SignedDuration, now: &ZonedDateTime) -> PaceResult<At> {
    if duration.is_positive() {
        let back = input.trim().trim_start_matches('+').trim_start();
        return Err(
            PaceError::new(ErrorKind::InvalidInput, "`{input}` lies in the future")
                .attach_context("input", input)
                .attach_hint(format!("did you mean `-{back}`?")),
        );
    }
    let moment = now
        .checked_add(duration)
        .and_then(|moment| moment.with().subsec_nanosecond(0).build())
        .map_err(out_of_range)?;
    Ok(At::Exact(moment))
}

fn in_the_future(moment: impl fmt::Display, now: &ZonedDateTime) -> Box<PaceError> {
    PaceError::new(
        ErrorKind::InvalidInput,
        "the time {moment} lies in the future",
    )
    .attach_context("moment", moment.to_string())
    .attach_hint(format!(
        "pace records what happened; give a time before now, {}",
        now.strftime("%Y-%m-%d %H:%M")
    ))
}

fn no_time(input: &str) -> Box<PaceError> {
    PaceError::new(
        ErrorKind::InvalidInput,
        "`{input}` is not a time pace understands",
    )
    .attach_context("input", input)
    .attach_hint(
        "use `HH:MM`, `yesterday HH:MM`, `YYYY-MM-DD HH:MM` or a time back from now \
         like `-15m`",
    )
}
