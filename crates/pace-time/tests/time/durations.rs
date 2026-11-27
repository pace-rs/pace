//! Elapsed time between two moments, and durations as people type them.

use jiff::{SignedDuration, ZonedDateTime};
use pace_time::{elapsed, parse_duration};
use rstest::rstest;

use crate::TestResult;

fn at(rfc9557: &str) -> Result<ZonedDateTime, jiff::Error> {
    rfc9557.parse()
}

// The same three cases as the old pace: begin and end recorded with
// different offsets.
#[rstest]
#[case(
    "2024-03-23T11:34:31+01:00[+01:00]",
    "2024-03-23T08:34:31-02:00[-02:00]",
    0
)]
#[case(
    "2024-03-23T10:00:00+03:00[+03:00]",
    "2024-03-23T10:00:00+01:00[+01:00]",
    7_200
)]
#[case(
    "2024-03-23T10:00:00+01:00[+01:00]",
    "2024-03-23T10:00:00-03:00[-03:00]",
    14_400
)]
fn elapsed_time_does_not_depend_on_the_offsets(
    #[case] start: &str,
    #[case] end: &str,
    #[case] seconds: i64,
) -> TestResult {
    assert_eq!(
        elapsed(&at(start)?, &at(end)?)?,
        SignedDuration::from_secs(seconds)
    );
    Ok(())
}

#[test]
fn elapsed_time_counts_seconds() -> TestResult {
    assert_eq!(
        elapsed(
            &at("2021-01-01T00:00:00[Europe/Berlin]")?,
            &at("2021-01-01T00:00:01[Europe/Berlin]")?
        )?,
        SignedDuration::from_secs(1)
    );
    Ok(())
}

#[test]
fn elapsed_time_counts_the_real_hours_across_clock_change() -> TestResult {
    assert_eq!(
        elapsed(
            &at("2026-03-29T01:00[Europe/Berlin]")?,
            &at("2026-03-29T04:00[Europe/Berlin]")?
        )?,
        SignedDuration::from_hours(2)
    );
    Ok(())
}

#[test]
fn an_end_before_the_start_is_rejected() -> TestResult {
    let error = elapsed(
        &at("2021-01-01T00:00:01[Europe/Berlin]")?,
        &at("2021-01-01T00:00:00[Europe/Berlin]")?,
    )
    .err()
    .ok_or("an end before the start must fail")?;

    assert_eq!(
        error.to_string(),
        "the end 2021-01-01T00:00:00+01:00[Europe/Berlin] lies before the start \
         2021-01-01T00:00:01+01:00[Europe/Berlin]"
    );
    Ok(())
}

#[rstest]
#[case("15m", 15 * 60)]
#[case("1h30m", 90 * 60)]
#[case("1h 30m", 90 * 60)]
#[case("1.5h", 90 * 60)]
#[case("45s", 45)]
fn durations_are_typed_with_units(#[case] input: &str, #[case] seconds: i64) -> TestResult {
    assert_eq!(parse_duration(input)?, SignedDuration::from_secs(seconds));
    Ok(())
}

#[test]
fn bare_number_asks_for_unit() {
    let hint = parse_duration("90")
        .err()
        .and_then(|error| error.hint().map(ToOwned::to_owned));

    assert_eq!(
        hint.as_deref(),
        Some("add a unit, for example `90m` or `90s`")
    );
}

#[test]
fn text_that_is_no_duration_is_rejected() {
    let error = parse_duration("a while").err();

    assert_eq!(
        error.map(|error| error.to_string()).as_deref(),
        Some("`a while` is not a duration")
    );
}

#[rstest]
#[case::leading(" 30m")]
#[case::trailing("30m ")]
fn space_around_duration_is_ignored(#[case] input: &str) -> TestResult {
    assert_eq!(parse_duration(input)?, SignedDuration::from_mins(30));
    Ok(())
}

#[rstest]
#[case::ago("30m ago", "did you mean `30m`?")]
#[case::minus("-1h 30m", "did you mean `1h 30m`?")]
#[case::zero("0m", "give a duration longer than zero, like `15m`")]
#[case::longest_negative(
    "-9223372036854775808s",
    "give a duration longer than zero, like `15m`"
)]
fn duration_that_is_not_positive_is_rejected(
    #[case] input: &str,
    #[case] expected_hint: &str,
) -> TestResult {
    let error = parse_duration(input)
        .err()
        .ok_or_else(|| format!("`{input}` must be rejected"))?;

    assert_eq!(
        error.to_string(),
        format!("the duration `{input}` is not positive")
    );
    assert_eq!(error.hint(), Some(expected_hint));
    Ok(())
}
