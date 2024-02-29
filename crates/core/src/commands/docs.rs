#[cfg(feature = "clap")]
use clap::Parser;

use crate::{constants::PACE_DEV_DOCS_URL, constants::PACE_DOCS_URL, PaceResult};

/// `docs` subcommand options
#[derive(Debug, Clone)]
#[cfg_attr(feature = "clap", derive(Parser))]
pub struct DocsCommandOptions {
    /// Open the development documentation
    #[cfg_attr(feature = "clap", clap(short, long))]
    dev: bool,
}

impl DocsCommandOptions {
    pub fn handle_docs(&self) -> PaceResult<()> {
        match self.dev {
            true => open::that(PACE_DEV_DOCS_URL)?,
            false => open::that(PACE_DOCS_URL)?,
        }

        Ok(())
    }
}
