#[cfg(feature = "clap")]
use clap::Parser;

use crate::PaceResult;

/// Opens the documentation.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "clap", derive(Parser))]
pub struct DocsOptions {
    /// Open the development documentation
    #[clap(short, long)]
    dev: bool,
}

impl DocsOptions {
    pub fn handle_docs(&self) -> PaceResult<()> {
        match self.dev {
            true => open::that("https://pace.cli.rs/dev-docs")?,
            false => open::that("https://pace.cli.rs/docs")?,
        }

        Ok(())
    }
}
