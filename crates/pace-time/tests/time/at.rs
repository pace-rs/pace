//! The `--at` grammar of the tracking commands.

use jiff::ZonedDateTime;
use pace_time::{At, parse_at};
use proptest::prelude::*;
use rstest::rstest;

use crate::TestResult;

const NOW: &str = "2026-11-26T14:30:15.5[Europe/Berlin]";

fn at(rfc9557: &str) -> Result<ZonedDateTime, jiff::Error> {
    rfc9557.parse()
}

fn exact(input: &str, now: &str) -> Result<String, Box<dyn std::error::Error>> {
    match parse_at(input, &at(now)?)? {
        At::Exact(moment) => Ok(moment.to_string()),
        ambiguous @ At::Ambiguous { .. } => Err(format!("ambiguous: {ambiguous:?}").into()),
    }
}

fn rejected(input: &str, now: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    let error = parse_at(input, &at(now)?)
        .err()
        .ok_or_else(|| format!("`{input}` must be rejected"))?;
    Ok((
        error.to_string(),
        error.hint().unwrap_or_default().to_owned(),
    ))
}

#[rstest]
#[case::earlier_today("09:15", NOW, "2026-11-26T09:15:00+01:00[Europe/Berlin]")]
#[case::this_minute("14:30", NOW, "2026-11-26T14:30:00+01:00[Europe/Berlin]")]
#[case::one_digit_hour("9:05", NOW, "2026-11-26T09:05:00+01:00[Europe/Berlin]")]
#[case::later_than_now_means_yesterday(
    "23:30",
    "2026-11-27T00:15[Europe/Berlin]",
    "2026-11-26T23:30:00+01:00[Europe/Berlin]"
)]
fn a_clock_time_means_the_most_recent_such_time(
    #[case] input: &str,
    #[case] now: &str,
    #[case] expected: &str,
) -> TestResult {
    assert_eq!(exact(input, now)?, expected);
    Ok(())
}

#[rstest]
#[case("yesterday 17:00", "2026-11-25T17:00:00+01:00[Europe/Berlin]")]
#[case("Yesterday 17:00", "2026-11-25T17:00:00+01:00[Europe/Berlin]")]
#[case("yesterday 23:59", "2026-11-25T23:59:00+01:00[Europe/Berlin]")]
fn yesterday_names_the_day_before(#[case] input: &str, #[case] expected: &str) -> TestResult {
    assert_eq!(exact(input, NOW)?, expected);
    Ok(())
}

#[rstest]
#[case("2026-11-25 17:00", "2026-11-25T17:00:00+01:00[Europe/Berlin]")]
#[case("2026-11-25T17:00", "2026-11-25T17:00:00+01:00[Europe/Berlin]")]
#[case("2021-01-01 12:00", "2021-01-01T12:00:00+01:00[Europe/Berlin]")]
#[case("2026-07-01 08:00", "2026-07-01T08:00:00+02:00[Europe/Berlin]")]
fn a_date_and_time_is_taken_as_is_in_the_zone_of_now(
    #[case] input: &str,
    #[case] expected: &str,
) -> TestResult {
    assert_eq!(exact(input, NOW)?, expected);
    Ok(())
}

#[rstest]
#[case("-15m", "2026-11-26T14:15:15+01:00[Europe/Berlin]")]
#[case("-1h30m", "2026-11-26T13:00:15+01:00[Europe/Berlin]")]
#[case("15m ago", "2026-11-26T14:15:15+01:00[Europe/Berlin]")]
fn a_relative_time_counts_back_from_now_in_whole_seconds(
    #[case] input: &str,
    #[case] expected: &str,
) -> TestResult {
    assert_eq!(exact(input, NOW)?, expected);
    Ok(())
}

#[test]
fn a_time_in_the_future_is_rejected() -> TestResult {
    let (message, hint) = rejected("2079-01-02 10:00", NOW)?;

    assert_eq!(
        message,
        "2079-01-02T10:00:00+01:00[Europe/Berlin] lies in the future"
    );
    assert_eq!(
        hint,
        "pace records what happened; give a time before now, 2026-11-26 14:30"
    );
    Ok(())
}

#[rstest]
#[case("15m")]
#[case("+15m")]
fn a_relative_time_forward_suggests_going_back(#[case] input: &str) -> TestResult {
    let (_, hint) = rejected(input, NOW)?;

    assert_eq!(hint, "did you mean `-15m`?");
    Ok(())
}

#[rstest]
#[case::no_such_day("2021-01-32 12:00")]
#[case::no_such_hour("25:00")]
#[case::no_such_minute("12:60")]
#[case::yesterday_without_a_time("yesterday")]
#[case::words("soon")]
#[case::empty("")]
fn text_that_is_no_time_explains_the_grammar(#[case] input: &str) -> TestResult {
    let (message, hint) = rejected(input, NOW)?;

    assert_eq!(message, format!("`{input}` is not a time pace understands"));
    assert_eq!(
        hint,
        "use `HH:MM`, `yesterday HH:MM`, `YYYY-MM-DD HH:MM` or a time back from now \
         like `-15m`"
    );
    Ok(())
}

// 2026-03-29 02:30 does not exist in Berlin: clocks jump from 02:00 to 03:00.
#[test]
fn a_time_skipped_by_a_clock_change_moves_forward() -> TestResult {
    assert_eq!(
        exact("02:30", "2026-03-29T12:00[Europe/Berlin]")?,
        "2026-03-29T03:30:00+02:00[Europe/Berlin]"
    );
    Ok(())
}

// 2026-10-25 02:30 happens twice in Berlin: first at +02:00, then at +01:00.
#[test]
fn a_time_that_happened_twice_offers_both() -> TestResult {
    let both = parse_at("02:30", &at("2026-10-25T12:00[Europe/Berlin]")?)?;

    assert_eq!(
        both,
        At::Ambiguous {
            earlier: at("2026-10-25T02:30+02:00[Europe/Berlin]")?,
            later: at("2026-10-25T02:30+01:00[Europe/Berlin]")?,
        }
    );
    assert_eq!(
        both.earlier().to_string(),
        "2026-10-25T02:30:00+02:00[Europe/Berlin]"
    );
    Ok(())
}

#[test]
fn a_time_that_happened_twice_is_exact_while_the_second_lies_ahead() -> TestResult {
    assert_eq!(
        exact("02:30", "2026-10-25T02:45+02:00[Europe/Berlin]")?,
        "2026-10-25T02:30:00+02:00[Europe/Berlin]"
    );
    Ok(())
}

proptest! {
    #[test]
    fn parsing_never_panics(input in "\\PC*") {
        _ = parse_at(&input, &at(NOW).map_err(|e| TestCaseError::fail(e.to_string()))?);
    }

    #[test]
    fn a_clock_time_lies_within_the_last_day(hour in 0u8..24, minute in 0u8..60) {
        let now = at(NOW).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let moment = parse_at(&format!("{hour:02}:{minute:02}"), &now)
            .map_err(|e| TestCaseError::fail(e.to_string()))?
            .earlier();

        prop_assert!(moment <= now);
        prop_assert!(moment.duration_until(&now).as_hours() < 24);
    }
}
