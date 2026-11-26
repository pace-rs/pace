use jiff::{
    SignedDuration, ZonedDateTime,
    civil::{Date, DateTime, Time},
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
///   yesterday 23:30.
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

    if let Ok(duration) = text.parse::<SignedDuration>() {
        return back_from_now(input, duration, now);
    }

    Err(no_time(input))
}

/// `H:MM` or `HH:MM`.
fn clock_time(text: &str) -> Option<Time> {
    let (hour, minute) = text.split_once(':')?;
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    if !(digits(hour) && digits(minute) && hour.len() <= 2 && minute.len() == 2) {
        return None;
    }
    Time::new(hour.parse().ok()?, minute.parse().ok()?, 0, 0).ok()
}

/// `YYYY-MM-DD HH:MM` or `YYYY-MM-DDTHH:MM`.
fn date_and_time(text: &str) -> Option<DateTime> {
    let (date, time) = text.split_once([' ', 't'])?;
    Some(date.parse::<Date>().ok()?.to_datetime(clock_time(time)?))
}

/// The local `datetime` in the zone of `now`, if it lies in the past.
fn in_the_past(datetime: DateTime, now: &ZonedDateTime) -> PaceResult<At> {
    let ambiguous = now.time_zone().to_ambiguous_zoned_datetime(datetime);
    let candidates = match ambiguous.offset() {
        AmbiguousOffset::Fold { .. } => vec![
            ambiguous.clone().earlier().map_err(out_of_range)?,
            ambiguous.later().map_err(out_of_range)?,
        ],
        AmbiguousOffset::Unambiguous { .. } | AmbiguousOffset::Gap { .. } => {
            vec![ambiguous.compatible().map_err(out_of_range)?]
        }
    };

    let mut past = candidates.iter().filter(|moment| *moment <= now).cloned();
    match (past.next(), past.next()) {
        (Some(earlier), Some(later)) => Ok(At::Ambiguous { earlier, later }),
        (Some(moment), None) => Ok(At::Exact(moment)),
        (None, _) => Err(in_the_future(&candidates[0], now)),
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

fn in_the_future(moment: &ZonedDateTime, now: &ZonedDateTime) -> Box<PaceError> {
    PaceError::new(ErrorKind::InvalidInput, "{moment} lies in the future")
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
