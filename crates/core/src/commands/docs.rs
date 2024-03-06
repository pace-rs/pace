#[cfg(feature = "clap")]
use clap::Parser;

use crate::{
    constants::PACE_DOCS_URL,
    constants::{PACE_CONFIG_DOCS_URL, PACE_DEV_DOCS_URL},
    PaceResult, UserMessage,
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
    /// Handles the `docs` subcommand
    ///
    /// # Errors
    ///
    /// Returns an error if the browser could not be opened
    ///
    /// # Returns
    ///
    /// Returns a `UserMessage` with the information about the opened documentation
    /// that can be displayed to the user
    #[tracing::instrument(skip(self))]
    pub fn handle_docs(&self) -> PaceResult<UserMessage> {
        // If no flag is set, open the regular documentation
        let user_string = if !self.dev && !self.config {
            open::that(PACE_DOCS_URL)?;

            format!("Opening the user documentation at {}", PACE_DOCS_URL)
        } else if self.config {
            // Open the config documentation
            open::that(PACE_CONFIG_DOCS_URL)?;

            format!(
                "Opening the configuration documentation at {}",
                PACE_CONFIG_DOCS_URL
            )
        } else if self.dev {
            // Open the development documentation
            open::that(PACE_DEV_DOCS_URL)?;

            format!(
                "Opening the development documentation at {}",
                PACE_DEV_DOCS_URL
            )
        } else {
            "No documentation to open".to_string()
        };

        Ok(UserMessage::new(user_string))
    }
}
