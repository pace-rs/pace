use abscissa_core::{Command, Runnable};
use clap::{Parser, Subcommand};

use pace_cli::prompt_time_zone;
use pace_core::prelude::PaceConfig;

use crate::prelude::PACE_APP;

/// `set` subcommand
#[derive(Subcommand, Command, Debug, Runnable)]
pub enum SetChoiceSubCmd {
    /// Set the time zone
    #[clap(visible_alias = "tz")]
    Timezone(SetTimezoneSubCmd),
}

/// `setup` subcommand
#[derive(Command, Debug, Parser, Runnable)]
pub struct SetChoiceCmd {
    #[clap(subcommand)]
    commands: SetChoiceSubCmd,
}

/// Set the time zone
#[derive(Command, Debug, Parser)]
pub struct SetTimezoneSubCmd {}

impl Runnable for SetTimezoneSubCmd {
    fn run(&self) {
        let time_zone = prompt_time_zone().expect("Time zone not set.");

        let mut config: PaceConfig = toml::from_str(
            std::fs::read_to_string(PACE_APP.config_path())
                .unwrap()
                .as_str(),
        )
        .unwrap();

        config.set_time_zone(time_zone);

        std::fs::write(PACE_APP.config_path(), toml::to_string(&config).unwrap()).unwrap();
    }
}
