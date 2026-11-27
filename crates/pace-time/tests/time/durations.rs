//! Durations as people type them.

use jiff::SignedDuration;
use pace_time::parse_duration;
use rstest::rstest;

use crate::TestResult;

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
