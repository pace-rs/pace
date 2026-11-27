//! Asking the user, only when a terminal is attached: the [`Ask`] trait and
//! the [`Terminal`] that the edge finds once.

use std::io::IsTerminal;

use dialoguer::{Confirm, Select, theme::SimpleTheme};
use pace_error::{ErrorKind, PaceError, PaceResult};

/// How pace asks the user. The edge finds the [`Terminal`] once and passes
/// it in, so a test passes scripted answers instead.
pub trait Ask {
    /// Whether pace may ask.
    fn can_ask(&self) -> bool;

    /// Let the user pick one of `items`; the index of the pick.
    ///
    /// # Errors
    ///
    /// If the terminal fails or the user cancels.
    fn choose(&self, prompt: &str, items: &[String]) -> PaceResult<usize>;

    /// Ask a yes-or-no question; no is the default.
    ///
    /// # Errors
    ///
    /// If the terminal fails or the user cancels.
    fn confirm(&self, question: &str) -> PaceResult<bool>;
}

/// The terminal, as pace found it at the start: whether stdin and stderr
/// are terminals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Terminal {
    attached: bool,
}

impl Terminal {
    /// Find out whether stdin and stderr are terminals.
    #[must_use]
    pub fn detect() -> Self {
        Self {
            attached: std::io::stdin().is_terminal() && std::io::stderr().is_terminal(),
        }
    }

    /// The error for a question without a terminal, which the callers'
    /// `can_ask` checks prevent.
    fn detached(question: &str) -> Box<PaceError> {
        PaceError::new(
            ErrorKind::Internal,
            "pace asked \"{question}\" without a terminal",
        )
        .attach_context("question", question)
        .ask_report()
    }
}

impl Ask for Terminal {
    fn can_ask(&self) -> bool {
        self.attached
    }

    fn choose(&self, prompt: &str, items: &[String]) -> PaceResult<usize> {
        if !self.attached {
            return Err(Self::detached(prompt));
        }
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

    fn confirm(&self, question: &str) -> PaceResult<bool> {
        if !self.attached {
            return Err(Self::detached(question));
        }
        Confirm::with_theme(&SimpleTheme)
            .with_prompt(question)
            .default(false)
            .interact()
            .map_err(|error| {
                PaceError::with_source(ErrorKind::InvalidInput, "there was no answer", error)
                    .attach_hint("run the command again and answer the question")
            })
    }
}

#[cfg(test)]
pub use scripted::Scripted;

#[cfg(test)]
mod scripted {
    //! Scripted answers, for tests.

    use std::{cell::RefCell, collections::VecDeque};

    use pace_error::{ErrorKind, PaceError, PaceResult};

    use crate::prompt::Ask;

    /// Answers a test gives in place of the user, and the questions pace
    /// asked.
    #[derive(Debug, Default)]
    pub struct Scripted {
        attached: bool,
        choices: RefCell<VecDeque<usize>>,
        confirms: RefCell<VecDeque<bool>>,
        questions: RefCell<Vec<String>>,
        offered: RefCell<Vec<Vec<String>>>,
    }

    impl Scripted {
        /// No terminal: pace may not ask.
        #[must_use]
        pub fn without_terminal() -> Self {
            Self::default()
        }

        /// A terminal where the user picks `choices`, one per question.
        #[must_use]
        pub fn choosing(choices: impl IntoIterator<Item = usize>) -> Self {
            Self {
                attached: true,
                choices: RefCell::new(choices.into_iter().collect()),
                ..Self::default()
            }
        }

        /// A terminal where the user answers `answers`, one per question.
        #[must_use]
        pub fn confirming(answers: impl IntoIterator<Item = bool>) -> Self {
            Self {
                attached: true,
                confirms: RefCell::new(answers.into_iter().collect()),
                ..Self::default()
            }
        }

        /// The questions pace asked, in order.
        #[must_use]
        pub fn questions(&self) -> Vec<String> {
            self.questions.borrow().clone()
        }

        /// The items of each choice pace offered, in order.
        #[must_use]
        pub fn offered(&self) -> Vec<Vec<String>> {
            self.offered.borrow().clone()
        }

        fn unscripted(question: &str) -> Box<PaceError> {
            PaceError::new(ErrorKind::Internal, "the test has no answer to {question}")
                .attach_context("question", question)
        }
    }

    impl Ask for Scripted {
        fn can_ask(&self) -> bool {
            self.attached
        }

        fn choose(&self, prompt: &str, items: &[String]) -> PaceResult<usize> {
            self.questions.borrow_mut().push(prompt.to_owned());
            self.offered.borrow_mut().push(items.to_vec());
            self.choices
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| Self::unscripted(prompt))
        }

        fn confirm(&self, question: &str) -> PaceResult<bool> {
            self.questions.borrow_mut().push(question.to_owned());
            self.confirms
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| Self::unscripted(question))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detached_terminal_refuses_to_ask() {
        let terminal = Terminal { attached: false };

        let error = terminal.confirm("Forget it?").unwrap_err();

        assert!(!terminal.can_ask());
        assert_eq!(error.kind(), ErrorKind::Internal);
        assert_eq!(
            error.to_string(),
            "pace asked \"Forget it?\" without a terminal"
        );
    }
}
