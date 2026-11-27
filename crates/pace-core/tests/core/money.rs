//! Money: exact decimals in a currency's minor unit, amounts per hour, and
//! totals that keep currencies apart.

use jiff::SignedDuration;
use pace_core::{Currency, Money, Totals};
use rstest::rstest;

use crate::TestResult;

fn eur() -> Result<Currency, Box<dyn std::error::Error>> {
    Ok("EUR".parse()?)
}

#[rstest]
#[case("95.50", 9550)]
#[case("95", 9500)]
#[case("95.5", 9550)]
#[case("0.07", 7)]
fn decimal_reads_exactly_into_cents(#[case] text: &str, #[case] cents: i64) -> TestResult {
    assert_eq!(Money::parse_decimal(text, eur()?)?.minor(), cents);
    Ok(())
}

#[rstest]
#[case::too_many_decimals("95.555")]
#[case::no_digits_after_point("95.")]
#[case::no_digits_before_point(".5")]
#[case::comma("95,50")]
#[case::sign("-5")]
#[case::empty("")]
#[case::too_big("92233720368547759")]
fn other_decimals_are_refused_and_named(#[case] text: &str) -> TestResult {
    let error = Money::parse_decimal(text, eur()?).err();

    assert_eq!(
        error.map(|error| error.to_string()),
        Some(format!("`{text}` is not an amount in EUR"))
    );
    Ok(())
}

#[test]
fn too_many_decimals_says_how_many_currency_has() -> TestResult {
    let error = Money::parse_decimal("95.555", eur()?).err();

    assert_eq!(
        error
            .and_then(|error| error.hint().map(ToOwned::to_owned))
            .as_deref(),
        Some("write an amount in EUR with at most 2 decimals, like `95.50`")
    );
    Ok(())
}

#[test]
fn yen_has_no_minor_digits() -> TestResult {
    let jpy: Currency = "JPY".parse()?;

    let money = Money::parse_decimal("1200", jpy)?;

    assert_eq!((jpy.minor_digits(), money.minor()), (0, 1200));
    assert_eq!(money.to_string(), "1200");
    assert!(Money::parse_decimal("1200.5", jpy).is_err());
    Ok(())
}

#[rstest]
#[case("95.50", "95.50")]
#[case("95", "95.00")]
#[case("0.07", "0.07")]
#[case("1200.5", "1200.50")]
fn money_displays_as_decimal(#[case] text: &str, #[case] shown: &str) -> TestResult {
    assert_eq!(Money::parse_decimal(text, eur()?)?.to_string(), shown);
    Ok(())
}

#[test]
fn unknown_currency_names_closest_code() {
    let error = "EUO".parse::<Currency>().err();

    assert_eq!(
        error.as_ref().map(ToString::to_string).as_deref(),
        Some("`EUO` is not an ISO 4217 currency code")
    );
    assert_eq!(
        error.as_ref().and_then(|error| error.hint()),
        Some("did you mean `EUR`?")
    );
}

#[test]
fn lowercase_currency_names_its_uppercase_code() {
    let hint = "eur"
        .parse::<Currency>()
        .err()
        .and_then(|error| error.hint().map(ToOwned::to_owned));

    assert_eq!(hint.as_deref(), Some("did you mean `EUR`?"));
}

#[test]
fn currency_without_minor_unit_is_refused() {
    let error = "XAU".parse::<Currency>().err();

    assert_eq!(
        error.map(|error| error.to_string()).as_deref(),
        Some("pace can't bill in `XAU`, a currency without a minor unit")
    );
}

#[test]
fn currency_displays_its_code() -> TestResult {
    assert_eq!(eur()?.to_string(), "EUR");
    assert_eq!(eur()?.code(), "EUR");
    Ok(())
}

#[test]
fn ten_and_twenty_cents_add_to_exactly_thirty() -> TestResult {
    let mut totals = Totals::default();

    totals.add(Money::parse_decimal("0.10", eur()?)?)?;
    totals.add(Money::parse_decimal("0.20", eur()?)?)?;

    assert_eq!(totals.get(eur()?).to_string(), "0.30");
    Ok(())
}

#[rstest]
#[case::twenty_minutes(SignedDuration::from_mins(20), "31.83")]
#[case::one_second(SignedDuration::from_secs(1), "0.03")]
#[case::one_hour(SignedDuration::from_hours(1), "95.50")]
#[case::nothing(SignedDuration::ZERO, "0.00")]
fn amount_for_hourly_rate_rounds_half_up_to_cent(
    #[case] worked: SignedDuration,
    #[case] amount: &str,
) -> TestResult {
    let hourly = Money::parse_decimal("95.50", eur()?)?;

    assert_eq!(hourly.amount_for(worked)?.to_string(), amount);
    Ok(())
}

#[test]
fn half_cent_rounds_up() -> TestResult {
    let hourly = Money::parse_decimal("0.01", eur()?)?;

    assert_eq!(hourly.amount_for(SignedDuration::from_mins(30))?.minor(), 1);
    assert_eq!(
        hourly.amount_for(SignedDuration::from_secs(1799))?.minor(),
        0
    );
    Ok(())
}

#[test]
fn amount_that_outgrows_counting_is_refused() -> TestResult {
    let hourly = Money::parse_decimal("9000000000000000000", "JPY".parse()?)?;

    assert!(hourly.amount_for(SignedDuration::from_hours(2)).is_err());
    Ok(())
}

#[test]
fn totals_keep_currencies_apart() -> TestResult {
    let usd: Currency = "USD".parse()?;
    let mut totals = Totals::default();

    totals.add(Money::parse_decimal("10", eur()?)?)?;
    totals.add(Money::parse_decimal("5", usd)?)?;
    totals.add(Money::parse_decimal("1.50", eur()?)?)?;

    let shown: Vec<String> = totals
        .iter()
        .map(|money| format!("{} {}", money, money.currency()))
        .collect();
    assert_eq!(shown, ["11.50 EUR", "5.00 USD"]);
    Ok(())
}

#[test]
fn total_of_currency_without_amounts_is_zero() -> TestResult {
    let totals = Totals::default();

    assert_eq!(totals.get(eur()?).to_string(), "0.00");
    Ok(())
}
