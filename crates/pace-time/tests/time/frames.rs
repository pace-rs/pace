//! Time frames resolve to half-open ranges `[start, end)` in the zone of
//! "now", which is the report time zone.

use jiff::{ZonedDateTime, civil::Weekday};
use pace_time::{TimeFrame, TimeRange};
use rstest::rstest;

use crate::TestResult;

fn at(rfc9557: &str) -> Result<ZonedDateTime, jiff::Error> {
    rfc9557.parse()
}

fn bounds(range: &TimeRange) -> (Option<String>, Option<String>) {
    (
        range.start().map(ToString::to_string),
        range.end().map(ToString::to_string),
    )
}

fn resolve(
    frame: &TimeFrame,
    now: &str,
    week_start: Weekday,
) -> Result<(Option<String>, Option<String>), Box<dyn std::error::Error>> {
    Ok(bounds(&frame.resolve(&at(now)?, week_start)?))
}

#[rstest]
#[case("today", TimeFrame::Today)]
#[case("yesterday", TimeFrame::Yesterday)]
#[case("this-week", TimeFrame::ThisWeek)]
#[case("last-week", TimeFrame::LastWeek)]
#[case("this-month", TimeFrame::ThisMonth)]
#[case("last-month", TimeFrame::LastMonth)]
#[case("this-year", TimeFrame::ThisYear)]
#[case("last-year", TimeFrame::LastYear)]
#[case("all", TimeFrame::All)]
fn frames_are_named_as_on_the_command_line(
    #[case] name: &str,
    #[case] frame: TimeFrame,
) -> TestResult {
    assert_eq!(name.parse::<TimeFrame>()?, frame);
    assert_eq!(frame.to_string(), name);
    Ok(())
}

#[rstest]
#[case(Some("2021-02-02"), Some("2021-02-04"), "2021-02-02 to 2021-02-04")]
#[case(Some("2026-11-01"), None, "from 2026-11-01")]
#[case(None, Some("2026-11-01"), "until 2026-11-01")]
#[case(None, None, "all")]
fn days_between_read_as_sentence(
    #[case] from: Option<&str>,
    #[case] to: Option<&str>,
    #[case] text: &str,
) -> TestResult {
    let frame = TimeFrame::Between {
        from: from.map(str::parse).transpose()?,
        to: to.map(str::parse).transpose()?,
    };

    assert_eq!(frame.to_string(), text);
    Ok(())
}

#[test]
fn misspelled_frame_suggests_the_closest_name() {
    let error = "this-wek"
        .parse::<TimeFrame>()
        .err()
        .map(|error| error.hint().unwrap_or_default().to_owned());

    assert_eq!(error.as_deref(), Some("did you mean `this-week`?"));
}

#[rstest]
// Thursday
#[case::today(
    TimeFrame::Today,
    "2026-11-26T14:30[Europe/Berlin]",
    "2026-11-26T00:00:00+01:00[Europe/Berlin]",
    "2026-11-27T00:00:00+01:00[Europe/Berlin]"
)]
#[case::yesterday(
    TimeFrame::Yesterday,
    "2026-11-26T14:30[Europe/Berlin]",
    "2026-11-25T00:00:00+01:00[Europe/Berlin]",
    "2026-11-26T00:00:00+01:00[Europe/Berlin]"
)]
#[case::this_week(
    TimeFrame::ThisWeek,
    "2026-11-26T14:30[Europe/Berlin]",
    "2026-11-23T00:00:00+01:00[Europe/Berlin]",
    "2026-11-30T00:00:00+01:00[Europe/Berlin]"
)]
// The old pace returned the previous week on Mondays.
#[case::this_week_on_monday(
    TimeFrame::ThisWeek,
    "2026-11-30T08:00[Europe/Berlin]",
    "2026-11-30T00:00:00+01:00[Europe/Berlin]",
    "2026-12-07T00:00:00+01:00[Europe/Berlin]"
)]
#[case::this_week_late_on_sunday(
    TimeFrame::ThisWeek,
    "2026-11-29T23:59[Europe/Berlin]",
    "2026-11-23T00:00:00+01:00[Europe/Berlin]",
    "2026-11-30T00:00:00+01:00[Europe/Berlin]"
)]
#[case::last_week(
    TimeFrame::LastWeek,
    "2026-11-26T14:30[Europe/Berlin]",
    "2026-11-16T00:00:00+01:00[Europe/Berlin]",
    "2026-11-23T00:00:00+01:00[Europe/Berlin]"
)]
#[case::last_week_on_monday(
    TimeFrame::LastWeek,
    "2026-11-30T08:00[Europe/Berlin]",
    "2026-11-23T00:00:00+01:00[Europe/Berlin]",
    "2026-11-30T00:00:00+01:00[Europe/Berlin]"
)]
// The old pace had a hack here that returned Dec 25 to Dec 31.
#[case::last_week_across_new_year(
    TimeFrame::LastWeek,
    "2027-01-02T10:00[Europe/Berlin]",
    "2026-12-21T00:00:00+01:00[Europe/Berlin]",
    "2026-12-28T00:00:00+01:00[Europe/Berlin]"
)]
#[case::this_month(
    TimeFrame::ThisMonth,
    "2026-11-26T14:30[Europe/Berlin]",
    "2026-11-01T00:00:00+01:00[Europe/Berlin]",
    "2026-12-01T00:00:00+01:00[Europe/Berlin]"
)]
#[case::last_month(
    TimeFrame::LastMonth,
    "2026-11-26T14:30[Europe/Berlin]",
    "2026-10-01T00:00:00+02:00[Europe/Berlin]",
    "2026-11-01T00:00:00+01:00[Europe/Berlin]"
)]
// The old pace failed in January: it asked for month 0.
#[case::last_month_in_january(
    TimeFrame::LastMonth,
    "2027-01-15T09:00[Europe/Berlin]",
    "2026-12-01T00:00:00+01:00[Europe/Berlin]",
    "2027-01-01T00:00:00+01:00[Europe/Berlin]"
)]
#[case::this_year(
    TimeFrame::ThisYear,
    "2026-11-26T14:30[Europe/Berlin]",
    "2026-01-01T00:00:00+01:00[Europe/Berlin]",
    "2027-01-01T00:00:00+01:00[Europe/Berlin]"
)]
#[case::last_year(
    TimeFrame::LastYear,
    "2026-11-26T14:30[Europe/Berlin]",
    "2025-01-01T00:00:00+01:00[Europe/Berlin]",
    "2026-01-01T00:00:00+01:00[Europe/Berlin]"
)]
fn frames_cover_whole_periods(
    #[case] frame: TimeFrame,
    #[case] now: &str,
    #[case] start: &str,
    #[case] end: &str,
) -> TestResult {
    assert_eq!(
        resolve(&frame, now, Weekday::Monday)?,
        (Some(start.to_owned()), Some(end.to_owned()))
    );
    Ok(())
}

#[test]
fn the_week_starts_on_the_configured_day() -> TestResult {
    assert_eq!(
        resolve(
            &TimeFrame::ThisWeek,
            "2026-11-26T14:30[Europe/Berlin]",
            Weekday::Sunday
        )?,
        (
            Some("2026-11-22T00:00:00+01:00[Europe/Berlin]".to_owned()),
            Some("2026-11-29T00:00:00+01:00[Europe/Berlin]".to_owned())
        )
    );
    Ok(())
}

#[test]
fn frames_use_the_zone_of_now() -> TestResult {
    assert_eq!(
        resolve(
            &TimeFrame::Today,
            "2026-11-26T20:00[America/New_York]",
            Weekday::Monday
        )?,
        (
            Some("2026-11-26T00:00:00-05:00[America/New_York]".to_owned()),
            Some("2026-11-27T00:00:00-05:00[America/New_York]".to_owned())
        )
    );
    Ok(())
}

#[rstest]
#[case::spring_forward("2026-03-29T12:00[Europe/Berlin]", 23)]
#[case::fall_back("2026-10-25T12:00[Europe/Berlin]", 25)]
fn day_with_clock_change_has_its_real_length(#[case] now: &str, #[case] hours: i64) -> TestResult {
    let range = TimeFrame::Today.resolve(&at(now)?, Weekday::Monday)?;
    let (Some(start), Some(end)) = (range.start(), range.end()) else {
        return Err("today must be bounded".into());
    };

    assert_eq!(start.duration_until(end).as_hours(), hours);
    Ok(())
}

// Chile springs forward at midnight: 2026-09-06 begins at 01:00.
#[test]
fn day_without_midnight_starts_at_its_first_moment() -> TestResult {
    assert_eq!(
        resolve(
            &TimeFrame::Today,
            "2026-09-06T12:00[America/Santiago]",
            Weekday::Monday
        )?,
        (
            Some("2026-09-06T01:00:00-03:00[America/Santiago]".to_owned()),
            Some("2026-09-07T00:00:00-03:00[America/Santiago]".to_owned())
        )
    );
    Ok(())
}

#[test]
fn all_is_unbounded() -> TestResult {
    assert_eq!(
        resolve(
            &TimeFrame::All,
            "2026-11-26T14:30[Europe/Berlin]",
            Weekday::Monday
        )?,
        (None, None)
    );
    Ok(())
}

#[test]
fn from_and_to_include_both_days() -> TestResult {
    let frame = TimeFrame::Between {
        from: Some("2021-02-02".parse()?),
        to: Some("2021-02-04".parse()?),
    };

    assert_eq!(
        resolve(&frame, "2026-11-26T14:30[Europe/Berlin]", Weekday::Monday)?,
        (
            Some("2021-02-02T00:00:00+01:00[Europe/Berlin]".to_owned()),
            Some("2021-02-05T00:00:00+01:00[Europe/Berlin]".to_owned())
        )
    );
    Ok(())
}

// The old pace ended a single day at 23:59:59 and lost its last second.
#[test]
fn single_day_ends_at_the_next_midnight() -> TestResult {
    let day = "2021-02-02".parse()?;
    let frame = TimeFrame::Between {
        from: Some(day),
        to: Some(day),
    };

    assert_eq!(
        resolve(&frame, "2026-11-26T14:30[Europe/Berlin]", Weekday::Monday)?,
        (
            Some("2021-02-02T00:00:00+01:00[Europe/Berlin]".to_owned()),
            Some("2021-02-03T00:00:00+01:00[Europe/Berlin]".to_owned())
        )
    );
    Ok(())
}

#[test]
fn only_from_runs_to_now() -> TestResult {
    let frame = TimeFrame::Between {
        from: Some("2026-11-01".parse()?),
        to: None,
    };

    assert_eq!(
        resolve(&frame, "2026-11-26T14:30[Europe/Berlin]", Weekday::Monday)?,
        (
            Some("2026-11-01T00:00:00+01:00[Europe/Berlin]".to_owned()),
            Some("2026-11-26T14:30:00+01:00[Europe/Berlin]".to_owned())
        )
    );
    Ok(())
}

#[test]
fn only_to_starts_at_the_first_entry() -> TestResult {
    let frame = TimeFrame::Between {
        from: None,
        to: Some("2026-11-01".parse()?),
    };

    assert_eq!(
        resolve(&frame, "2026-11-26T14:30[Europe/Berlin]", Weekday::Monday)?,
        (
            None,
            Some("2026-11-02T00:00:00+01:00[Europe/Berlin]".to_owned())
        )
    );
    Ok(())
}

#[test]
fn from_after_to_is_rejected() -> TestResult {
    let frame = TimeFrame::Between {
        from: Some("2021-02-04".parse()?),
        to: Some("2021-02-02".parse()?),
    };

    let error = frame
        .resolve(&at("2026-11-26T14:30[Europe/Berlin]")?, Weekday::Monday)
        .err()
        .ok_or("from after to must fail")?;

    assert_eq!(
        error.to_string(),
        "the time frame starts on 2021-02-04, after it ends on 2021-02-02"
    );
    Ok(())
}

#[test]
fn range_contains_its_start_but_not_its_end() -> TestResult {
    let range = TimeRange::new(
        Some(at("2021-02-02T00:00[Europe/Berlin]")?),
        Some(at("2021-02-04T00:00[Europe/Berlin]")?),
    )?;

    assert!(range.contains(&at("2021-02-02T00:00[Europe/Berlin]")?));
    assert!(range.contains(&at("2021-02-03T00:00[Europe/Berlin]")?));
    assert!(!range.contains(&at("2021-02-04T00:00[Europe/Berlin]")?));
    assert!(!range.contains(&at("2021-02-01T23:59:59[Europe/Berlin]")?));
    Ok(())
}

#[test]
fn an_open_range_contains_everything_on_its_open_side() -> TestResult {
    let range = TimeRange::new(None, Some(at("2021-02-04T00:00[Europe/Berlin]")?))?;

    assert!(range.contains(&at("1999-01-01T00:00[Europe/Berlin]")?));
    assert!(!range.contains(&at("2021-02-04T00:00[Europe/Berlin]")?));
    Ok(())
}

#[test]
fn range_that_ends_before_it_starts_is_rejected() -> TestResult {
    let result = TimeRange::new(
        Some(at("2021-02-04T00:00[Europe/Berlin]")?),
        Some(at("2021-02-02T00:00[Europe/Berlin]")?),
    );

    assert!(result.is_err());
    Ok(())
}
