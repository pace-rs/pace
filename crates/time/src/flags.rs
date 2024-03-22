use chrono::NaiveDate;

#[cfg(feature = "clap")]
use clap::{Parser, ValueEnum};
use getset::{Getters, MutGetters, Setters};
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
#[cfg_attr(feature = "clap", derive(ValueEnum))]
pub enum TimeFlags {
    /// Show the reflection for the current day
    #[default]
    Today,

    /// Show the reflection for the previous day
    Yesterday,

    /// Show the reflection for the current week
    CurrentWeek,

    /// Show the reflection for the previous week
    LastWeek,

    /// Show the reflection for the current month
    CurrentMonth,

    /// Show the reflection for the previous month
    LastMonth,
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
