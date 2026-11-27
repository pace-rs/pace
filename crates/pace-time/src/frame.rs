//! Periods for reports: named frames such as `today` and `last-week`, or the
//! days from `--from` to `--to`.

use std::{fmt, str::FromStr};

use jiff::{
    ToSpan, ZonedDateTime,
    civil::{Date, Weekday},
    tz::TimeZone,
};
use pace_error::{ErrorKind, PaceError, PaceResult};

use crate::{TimeRange, out_of_range};

/// A period for reports: a named frame, or the days from `--from` to `--to`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeFrame {
    /// `today`
    Today,
    /// `yesterday`
    Yesterday,
    /// `this-week`, starting on the configured first day of the week
    ThisWeek,
    /// `last-week`
    LastWeek,
    /// `this-month`
    ThisMonth,
    /// `last-month`
    LastMonth,
    /// `this-year`
    ThisYear,
    /// `last-year`
    LastYear,
    /// `all`: everything recorded
    All,
    /// `--from` and `--to`, both days included. Only `from` runs to now;
    /// only `to` starts at the first entry.
    Between {
        /// The first day.
        from: Option<Date>,
        /// The last day.
        to: Option<Date>,
    },
}

const NAMED: [(&str, TimeFrame); 9] = [
    ("today", TimeFrame::Today),
    ("yesterday", TimeFrame::Yesterday),
    ("this-week", TimeFrame::ThisWeek),
    ("last-week", TimeFrame::LastWeek),
    ("this-month", TimeFrame::ThisMonth),
    ("last-month", TimeFrame::LastMonth),
    ("this-year", TimeFrame::ThisYear),
    ("last-year", TimeFrame::LastYear),
    ("all", TimeFrame::All),
];

impl TimeFrame {
    /// The range of this frame, seen from `now`, in the time zone of `now`.
    ///
    /// # Errors
    ///
    /// If `from` lies after `to`, or after today without `to`, or the dates
    /// leave the range jiff can calculate with.
    pub fn resolve(&self, now: &ZonedDateTime, week_start: Weekday) -> PaceResult<TimeRange> {
        let zone = now.time_zone();
        let today = now.date();
        let days = |date: Date, days: i64| date.checked_add(days.days()).map_err(out_of_range);
        let months =
            |date: Date, months: i64| date.checked_add(months.months()).map_err(out_of_range);
        let years = |date: Date, years: i64| date.checked_add(years.years()).map_err(out_of_range);
        let week = days(today, -i64::from(today.weekday().since(week_start)))?;
        let month = today.first_of_month();
        let year = today.first_of_year();

        let (start, end) = match *self {
            Self::Today => (today, days(today, 1)?),
            Self::Yesterday => (days(today, -1)?, today),
            Self::ThisWeek => (week, days(week, 7)?),
            Self::LastWeek => (days(week, -7)?, week),
            Self::ThisMonth => (month, months(month, 1)?),
            Self::LastMonth => (months(month, -1)?, month),
            Self::ThisYear => (year, years(year, 1)?),
            Self::LastYear => (years(year, -1)?, year),
            Self::All => return TimeRange::new(None, None),
            Self::Between { from, to } => return between(from, to, now),
        };

        TimeRange::new(
            Some(start_of_day(start, zone)?),
            Some(start_of_day(end, zone)?),
        )
    }
}

fn between(from: Option<Date>, to: Option<Date>, now: &ZonedDateTime) -> PaceResult<TimeRange> {
    if let (Some(from), Some(to)) = (from, to)
        && from > to
    {
        return Err(PaceError::new(
            ErrorKind::InvalidInput,
            "the time frame starts on {from}, after it ends on {to}",
        )
        .attach_hint("swap `--from` and `--to`")
        .attach_context("from", from.to_string())
        .attach_context("to", to.to_string()));
    }

    let today = now.date();
    if let (Some(from), None) = (from, to)
        && from > today
    {
        return Err(PaceError::new(
            ErrorKind::InvalidInput,
            "the time frame starts on {from}, after today",
        )
        .attach_hint(format!("give `--from` today, {today}, or an earlier day"))
        .attach_context("from", from.to_string()));
    }

    let zone = now.time_zone();
    let start = from.map(|from| start_of_day(from, zone)).transpose()?;
    let end = match to {
        Some(to) => Some(start_of_day(
            to.checked_add(1.day()).map_err(out_of_range)?,
            zone,
        )?),
        None => from.map(|_| now.clone()),
    };
    TimeRange::new(start, end)
}

/// The first moment of `date` in `zone`, also when midnight falls into a
/// clock change.
fn start_of_day(date: Date, zone: &TimeZone) -> PaceResult<ZonedDateTime> {
    date.to_zoned_datetime(zone.clone()).map_err(out_of_range)
}

impl fmt::Display for TimeFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Between {
                from: Some(from),
                to: Some(to),
            } => write!(f, "{from} to {to}"),
            Self::Between {
                from: Some(from),
                to: None,
            } => write!(f, "from {from}"),
            Self::Between {
                from: None,
                to: Some(to),
            } => write!(f, "until {to}"),
            named => {
                let name = NAMED
                    .iter()
                    .find(|(_, frame)| *frame == named)
                    .map_or("all", |(name, _)| name);
                f.write_str(name)
            }
        }
    }
}

impl FromStr for TimeFrame {
    type Err = Box<PaceError>;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if let Some((_, frame)) = NAMED.iter().find(|(name, _)| *name == input) {
            return Ok(*frame);
        }

        let names = NAMED.map(|(name, _)| name);
        let closest = names
            .iter()
            .map(|name| (strsim::jaro_winkler(input, name), name))
            .filter(|(score, _)| *score > 0.8)
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, name)| *name);

        let error = PaceError::new(
            ErrorKind::InvalidInput,
            "`{input}` is not a time frame; pace knows {names}",
        )
        .attach_context("input", input)
        .attach_context("names", names.join(", "));

        Err(match closest {
            Some(name) => error.attach_hint(format!("did you mean `{name}`?")),
            None => error.attach_hint("pick one of the names above, or use `--from` and `--to`"),
        })
    }
}
