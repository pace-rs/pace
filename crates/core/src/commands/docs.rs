#[cfg(feature = "clap")]
use clap::Parser;

use crate::{
    constants::PACE_DOCS_URL,
    constants::{PACE_CONFIG_DOCS_URL, PACE_DEV_DOCS_URL},
    PaceResult,
};

/// `docs` subcommand options
#[derive(Debug, Clone)]
#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(
        feature = "clap", clap(group = clap::ArgGroup::new("documentation").multiple(false)))]
pub struct DocsCommandOptions {
    /// Open the development documentation
    #[cfg_attr(feature = "clap", clap(short, long, group = "documentation"))]
    dev: bool,

    /// Open the config documentation
    #[cfg_attr(feature = "clap", clap(short, long, group = "documentation"))]
    config: bool,
}

impl DocsCommandOptions {
    pub fn handle_docs(&self) -> PaceResult<()> {
        // If no flag is set, open the regular documentation
        if !self.dev && !self.config {
            open::that(PACE_DOCS_URL)?;
            return Ok(());
        } else if self.config {
            // Open the config documentation
            open::that(PACE_CONFIG_DOCS_URL)?;
            return Ok(());
        } else if self.dev {
            // Open the development documentation
            open::that(PACE_DEV_DOCS_URL)?;
            return Ok(());
        }

        Ok(())
    }
}
