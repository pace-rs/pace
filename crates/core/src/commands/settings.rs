use std::collections::HashSet;

use chrono::{NaiveDateTime, NaiveTime};
use chrono_tz::Tz;
#[cfg(feature = "clap")]
use clap::Parser;
use getset::Getters;
use tracing::debug;
use typed_builder::TypedBuilder;

use crate::{
    config::PaceConfig,
    prelude::{PaceResult, UserMessage},
};

/// `settings` subcommand options
#[derive(Debug)]
#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(
        feature = "clap", clap(group = clap::ArgGroup::new("tz").multiple(false).required(false)))]
pub struct SettingsCommandOptions {
    /// Time zone to use for the activity, e.g., "Europe/Amsterdam"
    #[cfg_attr(
        feature = "clap",
        clap(long, value_name = "Time Zone", group = "tz", visible_alias = "tz")
    )]
    time_zone: Option<Tz>,

    /// Time zone offset to use for the activity, e.g., "+0200" or "-0500". Format: ±HHMM
    #[cfg_attr(
        feature = "clap",
        clap(
            long,
            value_name = "Time Zone Offset",
            group = "tz",
            visible_alias = "tzo"
        )
    )]
    time_zone_offset: Option<i32>,
}

impl SettingsCommandOptions {
    /// Handle the `settings` subcommand
    ///
    /// # Arguments
    ///
    /// * `config` - The pace configuration
    ///
    ///
    /// # Returns
    ///
    /// A `UserMessage` to be printed to the user indicating the result of the operation and
    /// some additional information
    #[tracing::instrument(skip(self))]
    pub fn handle_settings(&self, config: &PaceConfig) -> PaceResult<UserMessage> {
        Ok(UserMessage::new("Not implemented yet!"))
    }
}
