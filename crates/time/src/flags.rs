use chrono::NaiveDate;

use clap::Parser;
use getset::{Getters, MutGetters, Setters};
use typed_builder::TypedBuilder;

#[derive(Debug, Getters, TypedBuilder, Setters, MutGetters, Clone, Eq, PartialEq, Default)]
#[getset(get = "pub")]
#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(feature = "clap", clap(group = clap::ArgGroup::new("time-flag").multiple(false)))]
// We allow this here, because it's convenient to have all the flags in one place for the cli
// and because it's easier to deal with clap in this way.
#[allow(clippy::struct_excessive_bools)]
pub struct TimeFlags {
    /// Show the reflection for the current day
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    #[builder(setter(strip_bool))]
    today: bool,

    /// Show the reflection for the previous day
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    #[builder(setter(strip_bool))]
    yesterday: bool,

    /// Show the reflection for the current week
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    #[builder(setter(strip_bool))]
    current_week: bool,

    /// Show the reflection for the previous week
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    #[builder(setter(strip_bool))]
    last_week: bool,

    /// Show the reflection for the current month
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    #[builder(setter(strip_bool))]
    current_month: bool,

    /// Show the reflection for the previous month
    #[cfg_attr(feature = "clap", clap(long, group = "time-flag"))]
    #[builder(setter(strip_bool))]
    last_month: bool,
}

#[derive(Debug, Getters, Default, TypedBuilder, Setters, MutGetters, Clone, Eq, PartialEq)]
#[getset(get = "pub")]
#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(
        feature = "clap", clap(group = clap::ArgGroup::new("date-flag").multiple(true)))]
pub struct DateFlags {
    /// Show the reflection for a specific date, mutually exclusive with `from` and `to`. Format: YYYY-MM-DD
    #[cfg_attr(
        feature = "clap",
        clap(
            long,
            group = "date-flag",
            value_name = "Specific Date",
            exclusive = true
        )
    )]
    #[builder(setter(strip_option))]
    pub(crate) date: Option<NaiveDate>,

    /// Start date for the reflection period. Format: YYYY-MM-DD
    #[cfg_attr(
        feature = "clap",
        clap(long, group = "date-flag", value_name = "Start Date")
    )]
    #[builder(setter(strip_option))]
    pub(crate) from: Option<NaiveDate>,

    /// End date for the reflection period. Format: YYYY-MM-DD
    #[cfg_attr(
        feature = "clap",
        clap(long, group = "date-flag", value_name = "End Date")
    )]
    #[builder(setter(strip_option))]
    pub(crate) to: Option<NaiveDate>,
}
