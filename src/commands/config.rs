//! `pace config`

use abscissa_core::{Command, Runnable};
use pace_error::PaceResult;

use crate::{
    application::{PACE_APP, exit_on_error},
    dirs::Located,
};

/// Look at pace's configuration.
#[derive(clap::Subcommand, Command, Debug, Runnable)]
pub enum ConfigCmd {
    /// Show where pace keeps its files.
    Show(ShowCmd),
}

/// `pace config show`
#[derive(clap::Args, Command, Debug)]
pub struct ShowCmd {
    /// Say where each value comes from.
    #[arg(long)]
    origin: bool,
}

impl Runnable for ShowCmd {
    fn run(&self) {
        exit_on_error(self.execute());
    }
}

impl ShowCmd {
    fn execute(&self) -> PaceResult<()> {
        let dirs = PACE_APP.dirs()?;
        let lines = [
            (
                "config file",
                &dirs.config_file,
                dirs.config_file.path.is_file(),
            ),
            ("data", &dirs.data, dirs.data.path.is_dir()),
            ("machine", &dirs.machine, dirs.machine.path.is_dir()),
        ];
        for (label, located, exists) in lines {
            println!("{}", self.describe(label, located, exists));
        }
        Ok(())
    }

    fn describe(&self, label: &str, located: &Located, exists: bool) -> String {
        let place = format!("# {:<12} {}", format!("{label}:"), located.path.display());
        if !self.origin {
            return place;
        }
        let presence = if exists { "" } else { ", not there yet" };
        format!("{place} ({}{presence})", located.origin)
    }
}
