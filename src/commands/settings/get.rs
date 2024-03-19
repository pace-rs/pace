use abscissa_core::{Application, Command, Runnable};
use clap::{Parser, Subcommand};

use crate::prelude::PACE_APP;

/// `get` subcommand
#[derive(Subcommand, Command, Debug, Runnable)]
pub enum GetChoiceSubCmd {
    /// Get the time zone
    #[clap(visible_alias = "tz")]
    Timezone(GetTimezoneSubCmd),
}

/// `get` subcommand for settings
#[derive(Command, Debug, Parser, Runnable)]
pub struct GetChoiceCmd {
    #[clap(subcommand)]
    commands: GetChoiceSubCmd,
}

/// Get the time zone
#[derive(Command, Debug, Parser)]
pub struct GetTimezoneSubCmd {}

impl Runnable for GetTimezoneSubCmd {
    fn run(&self) {
        if let Some(time_zone) = PACE_APP.config().general().default_time_zone().as_ref() {
            println!("{}", time_zone);
        } else {
            println!("No time zone set.");
        };
    }
}
