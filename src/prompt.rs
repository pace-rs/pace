//! Asking the user, only when a terminal is attached.

use std::io::IsTerminal;

use dialoguer::{Confirm, Select, theme::SimpleTheme};
use pace_error::{ErrorKind, PaceError, PaceResult};

/// Whether pace may ask: stdin and stderr are terminals.
#[must_use]
pub fn interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stderr().is_terminal()
}

/// Let the user pick one of `items`; the index of the pick.
///
/// # Errors
///
/// If the terminal fails or the user cancels.
pub fn choose(prompt: &str, items: &[String]) -> PaceResult<usize> {
    Select::with_theme(&SimpleTheme)
        .with_prompt(prompt)
        .items(items)
        .default(0)
        .interact()
        .map_err(|error| {
            PaceError::with_source(ErrorKind::InvalidInput, "nothing was chosen", error)
                .attach_hint("run the command again and pick one")
        })
}

/// Ask a yes-or-no question; no is the default.
///
/// # Errors
///
/// If the terminal fails or the user cancels.
pub fn confirm(question: &str) -> PaceResult<bool> {
    Confirm::with_theme(&SimpleTheme)
        .with_prompt(question)
        .default(false)
        .interact()
        .map_err(|error| {
            PaceError::with_source(ErrorKind::InvalidInput, "there was no answer", error)
                .attach_hint("run the command again and answer the question")
        })
}
