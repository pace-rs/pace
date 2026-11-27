//! Durations and moments as pace shows them.

use jiff::{SignedDuration, ZonedDateTime};
use pace_time::{clock, clock_against, format_duration, relative_time};
use rstest::rstest;

use crate::TestResult;

#[rstest]
#[case::nothing(0, "0s")]
#[case::seconds(45, "45s")]
#[case::almost_minute(59, "59s")]
#[case::minute(60, "1m")]
#[case::minutes_drop_the_seconds(65, "1m")]
#[case::an_hour(3_600, "1h")]
#[case::hours_and_minutes(3_660, "1h 1m")]
#[case::more_than_day_stays_in_hours(26 * 3_600 + 180, "26h 3m")]
#[case::negative(-90, "-1m")]
fn durations_read_in_hours_and_minutes(#[case] seconds: i64, #[case] text: &str) {
    assert_eq!(format_duration(SignedDuration::from_secs(seconds)), text);
}

const NOW: &str = "2026-11-26T14:30:00[Europe/Berlin]";

fn at(rfc9557: &str) -> Result<ZonedDateTime, jiff::Error> {
    rfc9557.parse()
}

#[rstest]
#[case::same_moment("2026-11-26T14:30:00[Europe/Berlin]", "just now")]
#[case::seconds_ago("2026-11-26T14:29:15[Europe/Berlin]", "just now")]
#[case::minute_ago("2026-11-26T14:29:00[Europe/Berlin]", "1 minute ago")]
#[case::minutes_ago("2026-11-26T14:05:00[Europe/Berlin]", "25 minutes ago")]
#[case::an_hour_ago("2026-11-26T13:29:00[Europe/Berlin]", "1 hour ago")]
#[case::hours_ago("2026-11-26T09:00:00[Europe/Berlin]", "5 hours ago")]
#[case::day_ago("2026-11-25T12:00:00[Europe/Berlin]", "1 day ago")]
#[case::days_ago("2026-11-21T14:30:00[Europe/Berlin]", "5 days ago")]
#[case::older_shows_the_date("2026-11-20T14:29:00[Europe/Berlin]", "on 2026-11-20")]
#[case::ahead_from_another_device("2026-11-26T14:40:00[Europe/Berlin]", "in 10 minutes")]
fn moments_read_relative_to_now(#[case] then: &str, #[case] text: &str) -> TestResult {
    assert_eq!(relative_time(&at(then)?, &at(NOW)?), text);
    Ok(())
}

#[test]
fn an_old_moment_shows_its_date_in_the_zone_of_now() -> TestResult {
    let then = at("2026-11-01T23:30:00[America/New_York]")?;

    assert_eq!(relative_time(&then, &at(NOW)?), "on 2026-11-02");
    Ok(())
}

#[rstest]
#[case::same_day("2026-11-26T09:05:00[Europe/Berlin]", "09:05")]
#[case::other_day("2026-11-25T23:59:00[Europe/Berlin]", "2026-11-25 23:59")]
#[case::other_zone_in_zone_of_now("2026-11-26T08:00:00[UTC]", "09:00")]
#[case::date_in_zone_of_now("2026-11-25T23:30:00[UTC]", "00:30")]
fn moments_show_clock_time_on_day_of_now(#[case] moment: &str, #[case] text: &str) -> TestResult {
    assert_eq!(clock(&at(moment)?, &at(NOW)?), text);
    Ok(())
}

#[rstest]
#[case::other_minute(
    "2026-11-26T09:05:10[Europe/Berlin]",
    "2026-11-26T09:06:00[Europe/Berlin]",
    "09:05"
)]
#[case::same_minute(
    "2026-11-26T09:05:10[Europe/Berlin]",
    "2026-11-26T09:05:40[Europe/Berlin]",
    "09:05:10"
)]
#[case::same_moment(
    "2026-11-26T09:05:10[Europe/Berlin]",
    "2026-11-26T09:05:10[Europe/Berlin]",
    "09:05"
)]
#[case::same_minute_other_day(
    "2026-11-25T09:05:10[Europe/Berlin]",
    "2026-11-25T09:05:40[Europe/Berlin]",
    "2026-11-25 09:05:10"
)]
fn moment_shows_seconds_against_other_of_same_minute(
    #[case] moment: &str,
    #[case] other: &str,
    #[case] text: &str,
) -> TestResult {
    assert_eq!(clock_against(&at(moment)?, &at(other)?, &at(NOW)?), text);
    Ok(())
}
