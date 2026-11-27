//! The period of a bill in words: `October 2026` for a calendar month,
//! a day, or the first and the last day.

use jiff::{SignedDuration, civil::Date, tz::TimeZone};
use pace_time::TimeRange;

/// The period of a bill, by the days it covers in the report's zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillPeriod {
    /// A calendar month, by its first day.
    Month(Date),
    /// One day.
    Day(Date),
    /// The first and the last day, both included.
    Days(Date, Date),
    /// Up to the last day, without a start.
    Until(Date),
    /// From the first day, without an end.
    From(Date),
    /// Everything recorded.
    All,
}

impl BillPeriod {
    /// The period of `range`, in `zone`.
    #[must_use]
    pub fn of(range: &TimeRange, zone: &TimeZone) -> Self {
        let first = range
            .start()
            .map(|start| start.with_time_zone(zone.clone()));
        let last = range.end().map(|end| {
            let end = end.with_time_zone(zone.clone());
            end.checked_sub(SignedDuration::from_nanos(1))
                .map_or_else(|_| end.date(), |last| last.date())
        });
        match (first, last) {
            (Some(first), Some(last)) => {
                let (first_day, last_day) = (first.date(), last.max(first.date()));
                let whole_month = first_day.day() == 1
                    && first.start_of_day().ok().as_ref() == Some(&first)
                    && last_day == first_day.last_of_month();
                if whole_month {
                    Self::Month(first_day)
                } else if first_day == last_day {
                    Self::Day(first_day)
                } else {
                    Self::Days(first_day, last_day)
                }
            }
            (None, Some(last)) => Self::Until(last),
            (Some(first), None) => Self::From(first.date()),
            (None, None) => Self::All,
        }
    }

    /// The period as a name: `October 2026`, `2026-11-26`,
    /// `2026-11-02 to 2026-11-26`, `until 2026-11-26`, `from 2026-11-02`,
    /// `all time`.
    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::Month(first) => first.strftime("%B %Y").to_string(),
            Self::Day(day) => day.to_string(),
            Self::Days(first, last) => format!("{first} to {last}"),
            Self::Until(last) => format!("until {last}"),
            Self::From(first) => format!("from {first}"),
            Self::All => "all time".to_owned(),
        }
    }

    /// The period as the end of a sentence: `in October 2026`,
    /// `on 2026-11-26`, `from 2026-11-02 to 2026-11-26`, `yet`.
    #[must_use]
    pub fn phrase(self) -> String {
        match self {
            Self::Month(_) => format!("in {}", self.label()),
            Self::Day(day) => format!("on {day}"),
            Self::Days(first, last) => format!("from {first} to {last}"),
            Self::Until(_) | Self::From(_) => self.label(),
            Self::All => "yet".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::{ZonedDateTime, civil::date};

    use super::*;

    fn berlin() -> TimeZone {
        TimeZone::get("Europe/Berlin").unwrap()
    }

    fn range(start: Option<&str>, end: Option<&str>) -> TimeRange {
        let time = |text: &str| {
            format!("{text}[Europe/Berlin]")
                .parse::<ZonedDateTime>()
                .unwrap()
        };
        TimeRange::new(start.map(time), end.map(time)).unwrap()
    }

    #[test]
    fn calendar_month_is_named_by_month_and_year() {
        let period = BillPeriod::of(
            &range(Some("2026-10-01T00:00"), Some("2026-11-01T00:00")),
            &berlin(),
        );

        assert_eq!(period, BillPeriod::Month(date(2026, 10, 1)));
        assert_eq!(period.label(), "October 2026");
        assert_eq!(period.phrase(), "in October 2026");
    }

    #[test]
    fn days_of_month_short_of_its_end_are_named_by_first_and_last_day() {
        let period = BillPeriod::of(
            &range(Some("2026-11-01T00:00"), Some("2026-11-26T15:00")),
            &berlin(),
        );

        assert_eq!(period.label(), "2026-11-01 to 2026-11-26");
        assert_eq!(period.phrase(), "from 2026-11-01 to 2026-11-26");
    }

    #[test]
    fn one_day_is_named_by_its_date() {
        let period = BillPeriod::of(
            &range(Some("2026-11-26T00:00"), Some("2026-11-27T00:00")),
            &berlin(),
        );

        assert_eq!(period.label(), "2026-11-26");
        assert_eq!(period.phrase(), "on 2026-11-26");
    }

    #[test]
    fn open_periods_are_named_by_their_bound() {
        let zone = berlin();

        assert_eq!(
            BillPeriod::of(&range(None, Some("2026-11-27T00:00")), &zone).phrase(),
            "until 2026-11-26"
        );
        assert_eq!(
            BillPeriod::of(&range(Some("2026-11-02T00:00"), None), &zone).label(),
            "from 2026-11-02"
        );
        assert_eq!(
            BillPeriod::of(&TimeRange::everything(), &zone).label(),
            "all time"
        );
        assert_eq!(BillPeriod::All.phrase(), "yet");
    }
}
