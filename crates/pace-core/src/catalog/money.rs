//! Money: an amount as a whole number of its currency's minor unit, the
//! amount for a rate and a duration, and totals per currency.
//!
//! The currencies and their minor units are ISO 4217's, from the crate
//! `iso_currency`. An amount never passes through a float: pace reads it
//! exactly from its decimal text and computes with whole numbers.

use std::{collections::BTreeMap, fmt, str::FromStr};

use iso_currency::IntoEnumIterator;
use jiff::SignedDuration;
use pace_error::{ErrorKind, PaceError, PaceResult, closest_name};
use serde::{Serialize, Serializer, ser::SerializeStruct};

const SECONDS_PER_HOUR: i128 = 3600;

/// A currency: a checked ISO 4217 code with a minor unit, such as `EUR`
/// (cents) or `JPY` (none).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Currency {
    iso: iso_currency::Currency,
    minor_digits: u8,
}

impl Currency {
    /// The ISO 4217 code, such as `EUR`.
    #[must_use]
    pub fn code(&self) -> &'static str {
        self.iso.code()
    }

    /// The number of decimals of its minor unit: 2 for EUR, 0 for JPY.
    #[must_use]
    pub const fn minor_digits(&self) -> u8 {
        self.minor_digits
    }

    /// How many minor units make one major unit: 100 for EUR, 1 for JPY.
    fn minor_per_major(self) -> i64 {
        10_i64.pow(u32::from(self.minor_digits))
    }
}

impl FromStr for Currency {
    type Err = Box<PaceError>;

    fn from_str(code: &str) -> Result<Self, Self::Err> {
        let Some(iso) = iso_currency::Currency::from_code(code) else {
            return Err(unknown_currency(code));
        };
        let minor_digits = iso.exponent().and_then(|digits| u8::try_from(digits).ok());
        let Some(minor_digits) = minor_digits else {
            return Err(PaceError::new(
                ErrorKind::InvalidInput,
                "pace can't bill in `{code}`, a currency without a minor unit",
            )
            .attach_context("code", code)
            .attach_hint("give the client a currency such as `EUR`")
            .attach_error_code("pace::core::currency-without-minor-unit"));
        };
        Ok(Self { iso, minor_digits })
    }
}

/// The error for a code that is no ISO 4217 currency; it names the closest.
fn unknown_currency(code: &str) -> Box<PaceError> {
    let upper = code.to_ascii_uppercase();
    let closest = closest_name(
        &upper,
        iso_currency::Currency::iter()
            .filter(|iso| iso.exponent().is_some())
            .map(|iso| iso.code()),
    );
    let hint = closest.map_or_else(
        || "a currency is a code of ISO 4217, such as `EUR` or `USD`".to_owned(),
        |closest| format!("did you mean `{closest}`?"),
    );
    PaceError::new(
        ErrorKind::InvalidInput,
        "`{code}` is not an ISO 4217 currency code",
    )
    .attach_context("code", code)
    .attach_hint(hint)
    .attach_error_code("pace::core::unknown-currency")
}

impl Ord for Currency {
    /// By code, so totals list `EUR` before `USD`.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.code().cmp(other.code())
    }
}

impl PartialOrd for Currency {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl fmt::Debug for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Currency({})", self.code())
    }
}

/// An amount of money: a whole number of its currency's minor unit, so
/// 95.50 EUR is 9550 cents.
///
/// Two amounts add up only in [`Totals`], which keeps each currency apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Money {
    minor: i64,
    currency: Currency,
}

impl Money {
    /// `minor` units of `currency`: `Money::from_minor(9550, eur)` is
    /// 95.50 EUR.
    #[must_use]
    pub const fn from_minor(minor: i64, currency: Currency) -> Self {
        Self { minor, currency }
    }

    /// Nothing, in `currency`.
    #[must_use]
    pub const fn zero(currency: Currency) -> Self {
        Self::from_minor(0, currency)
    }

    /// Read the decimal `text` exactly, such as `95.50`, `95.5` or `95`, as
    /// an amount in `currency`. No float takes part.
    ///
    /// # Errors
    ///
    /// If `text` is no decimal of digits with an optional point, if it has
    /// more decimals than the currency's minor unit, or if it is too big to
    /// count in minor units.
    pub fn parse_decimal(text: &str, currency: Currency) -> PaceResult<Self> {
        let refuse = || not_an_amount(text, currency);
        let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
        let digits_only = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
        if whole.is_empty()
            || !digits_only(whole)
            || !digits_only(fraction)
            || (text.contains('.') && fraction.is_empty())
            || fraction.len() > usize::from(currency.minor_digits)
        {
            return Err(refuse());
        }
        let padded = format!(
            "{whole}{fraction:0<width$}",
            width = usize::from(currency.minor_digits)
        );
        let minor = padded
            .parse::<i64>()
            .map_err(|error| refuse().attach_source(error))?;
        Ok(Self::from_minor(minor, currency))
    }

    /// The number of minor units: 9550 for 95.50 EUR.
    #[must_use]
    pub const fn minor(&self) -> i64 {
        self.minor
    }

    /// Its currency.
    #[must_use]
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    /// The amount for `worked` at this rate per hour, in whole seconds of
    /// `worked`, rounded half up to the minor unit: 95.50 per hour for
    /// 20 minutes is 31.83. It computes in `i128`, so no product overflows.
    ///
    /// # Errors
    ///
    /// If the amount is too big to count in minor units.
    pub fn amount_for(&self, worked: SignedDuration) -> PaceResult<Self> {
        let product = i128::from(self.minor) * i128::from(worked.as_secs());
        let rounded = (product + SECONDS_PER_HOUR / 2).div_euclid(SECONDS_PER_HOUR);
        let minor = i64::try_from(rounded).map_err(|error| {
            PaceError::with_source(
                ErrorKind::InvalidInput,
                "the amount for {worked} at {rate} {currency} per hour is too big to count",
                error,
            )
            .attach_context("worked", format!("{worked:#}"))
            .attach_context("rate", self.to_string())
            .attach_context("currency", self.currency.code())
            .attach_error_code("pace::core::amount-too-big")
        })?;
        Ok(Self::from_minor(minor, self.currency))
    }
}

impl Serialize for Money {
    /// The amount as its exact decimal text and the currency's code:
    /// `{"amount": "95.50", "currency": "EUR"}`.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut money = serializer.serialize_struct("Money", 2)?;
        money.serialize_field("amount", &self.to_string())?;
        money.serialize_field("currency", self.currency.code())?;
        money.end()
    }
}

/// The error for `text` that is no amount in `currency`.
fn not_an_amount(text: &str, currency: Currency) -> Box<PaceError> {
    let hint = match currency.minor_digits {
        0 => format!("write an amount in {currency} without decimals, like `1200`"),
        digits => format!(
            "write an amount in {currency} with at most {digits} decimals, like `95.{:0<width$}`",
            "5",
            width = usize::from(digits)
        ),
    };
    PaceError::new(
        ErrorKind::InvalidInput,
        "`{text}` is not an amount in {currency}",
    )
    .attach_context("text", text)
    .attach_context("currency", currency.code())
    .attach_hint(hint)
    .attach_error_code("pace::core::not-an-amount")
}

impl fmt::Display for Money {
    /// The amount as a decimal with all digits of its minor unit: `95.50`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let per_major = self.currency.minor_per_major().unsigned_abs();
        let sign = if self.minor < 0 { "-" } else { "" };
        let whole = self.minor.unsigned_abs() / per_major;
        match self.currency.minor_digits {
            0 => write!(f, "{sign}{whole}"),
            digits => write!(
                f,
                "{sign}{whole}.{:0>width$}",
                self.minor.unsigned_abs() % per_major,
                width = usize::from(digits)
            ),
        }
    }
}

/// Sums of money, one per currency: amounts of two currencies never add up.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Totals(BTreeMap<Currency, i64>);

impl Totals {
    /// Add `money` to the total of its currency.
    ///
    /// # Errors
    ///
    /// If the total grows too big to count in minor units.
    pub fn add(&mut self, money: Money) -> PaceResult<()> {
        let total = self.0.entry(money.currency).or_default();
        *total = total.checked_add(money.minor).ok_or_else(|| {
            PaceError::new(
                ErrorKind::InvalidInput,
                "the total in {currency} grows too big to count",
            )
            .attach_context("currency", money.currency.code())
            .attach_error_code("pace::core::amount-too-big")
        })?;
        Ok(())
    }

    /// The total in `currency`; zero if nothing in it was added.
    #[must_use]
    pub fn get(&self, currency: Currency) -> Money {
        Money::from_minor(self.0.get(&currency).copied().unwrap_or(0), currency)
    }

    /// The totals, ordered by currency code.
    pub fn iter(&self) -> impl Iterator<Item = Money> + '_ {
        self.0
            .iter()
            .map(|(currency, minor)| Money::from_minor(*minor, *currency))
    }
}
