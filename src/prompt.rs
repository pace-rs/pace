//! Asking the user, only when a terminal is attached: the [`Ask`] trait and
//! the [`Terminal`] that the edge finds once.

use std::{
    fmt,
    io::{IsTerminal, Write as _},
};

use dialoguer::{Confirm, Input, Select, theme::Theme};
use pace_error::{ErrorKind, PaceError, PaceResult};

/// How pace asks the user. The edge finds the [`Terminal`] once and passes
/// it in, so a test passes scripted answers instead.
pub trait Ask {
    /// Whether pace may ask.
    fn can_ask(&self) -> bool;

    /// Let the user pick one of `items`, with the item at `default`
    /// picked at the start; the index of the pick.
    ///
    /// # Errors
    ///
    /// If the terminal fails or the user cancels.
    fn choose(&self, prompt: &str, items: &[String], default: usize) -> PaceResult<usize>;

    /// Ask a yes-or-no question; Enter takes `default`.
    ///
    /// # Errors
    ///
    /// If the terminal fails or the user cancels.
    fn confirm(&self, question: &str, default: bool) -> PaceResult<bool>;

    /// Ask for a line of text; Enter takes `default`. Without a default the
    /// user must type something.
    ///
    /// # Errors
    ///
    /// If the terminal fails or the user cancels.
    fn input(&self, question: &str, default: Option<&str>) -> PaceResult<String>;

    /// Tell the user `message` between two questions, such as why an
    /// answer didn't fit.
    fn tell(&self, message: &str);
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

/// dialoguer's plain theme, but a choice's question keeps its question
/// mark, without the colon dialoguer adds: "Which open entry?", not
/// "Which open entry?:".
struct Questions;

impl Theme for Questions {
    fn format_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        write!(f, "{prompt}")
    }

    fn format_select_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selection: &str,
    ) -> fmt::Result {
        write!(f, "{prompt} {selection}")
    }

    fn format_input_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        default: Option<&str>,
    ) -> fmt::Result {
        match default {
            Some("") => write!(f, "{prompt} [none] "),
            Some(default) => write!(f, "{prompt} [{default}] "),
            None => write!(f, "{prompt} "),
        }
    }

    fn format_input_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selection: &str,
    ) -> fmt::Result {
        write!(f, "{prompt} {selection}")
    }
}

impl Ask for Terminal {
    fn can_ask(&self) -> bool {
        self.attached
    }

    fn choose(&self, prompt: &str, items: &[String], default: usize) -> PaceResult<usize> {
        if !self.attached {
            return Err(Self::detached(prompt));
        }
        Select::with_theme(&Questions)
            .with_prompt(prompt)
            .items(items)
            .default(default)
            .interact()
            .map_err(|error| {
                PaceError::with_source(ErrorKind::InvalidInput, "nothing was chosen", error)
                    .attach_hint("run the command again and pick one")
            })
    }

    fn confirm(&self, question: &str, default: bool) -> PaceResult<bool> {
        if !self.attached {
            return Err(Self::detached(question));
        }
        Confirm::with_theme(&Questions)
            .with_prompt(question)
            .default(default)
            .interact()
            .map_err(no_answer)
    }

    fn input(&self, question: &str, default: Option<&str>) -> PaceResult<String> {
        if !self.attached {
            return Err(Self::detached(question));
        }
        let mut input = Input::<String>::with_theme(&Questions)
            .with_prompt(question)
            .allow_empty(default.is_some());
        if let Some(default) = default {
            input = input.default(default.to_owned()).show_default(true);
        }
        input.interact_text().map_err(no_answer)
    }

    fn tell(&self, message: &str) {
        // A message that can't reach the terminal leaves the question to
        // show what is wrong.
        _ = writeln!(std::io::stderr().lock(), "{message}");
    }
}

/// The error for a question the user left without an answer.
fn no_answer(error: dialoguer::Error) -> Box<PaceError> {
    PaceError::with_source(ErrorKind::InvalidInput, "there was no answer", error)
        .attach_hint("run the command again and answer the question")
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
        inputs: RefCell<VecDeque<String>>,
        questions: RefCell<Vec<String>>,
        offered: RefCell<Vec<Vec<String>>>,
        told: RefCell<Vec<String>>,
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

        /// A terminal where the user types `inputs`, one per question; an
        /// empty input takes the question's default, as Enter does.
        #[must_use]
        pub fn typing<'text>(inputs: impl IntoIterator<Item = &'text str>) -> Self {
            Self {
                attached: true,
                ..Self::default()
            }
            .and_typing(inputs)
        }

        /// These answers, and the user types `inputs` too.
        #[must_use]
        pub fn and_typing<'text>(self, inputs: impl IntoIterator<Item = &'text str>) -> Self {
            self.inputs
                .borrow_mut()
                .extend(inputs.into_iter().map(str::to_owned));
            self
        }

        /// These answers, and the user picks `choices` too.
        #[must_use]
        pub fn and_choosing(self, choices: impl IntoIterator<Item = usize>) -> Self {
            self.choices.borrow_mut().extend(choices);
            self
        }

        /// These answers, and the user answers `answers` too.
        #[must_use]
        pub fn and_confirming(self, answers: impl IntoIterator<Item = bool>) -> Self {
            self.confirms.borrow_mut().extend(answers);
            self
        }

        /// What pace told the user between questions, in order.
        #[must_use]
        pub fn told(&self) -> Vec<String> {
            self.told.borrow().clone()
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

        fn choose(&self, prompt: &str, items: &[String], _default: usize) -> PaceResult<usize> {
            self.questions.borrow_mut().push(prompt.to_owned());
            self.offered.borrow_mut().push(items.to_vec());
            self.choices
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| Self::unscripted(prompt))
        }

        fn confirm(&self, question: &str, _default: bool) -> PaceResult<bool> {
            self.questions.borrow_mut().push(question.to_owned());
            self.confirms
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| Self::unscripted(question))
        }

        fn input(&self, question: &str, default: Option<&str>) -> PaceResult<String> {
            self.questions.borrow_mut().push(question.to_owned());
            let typed = self
                .inputs
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| Self::unscripted(question))?;
            Ok(match default {
                Some(default) if typed.is_empty() => default.to_owned(),
                _ => typed,
            })
        }

        fn tell(&self, message: &str) {
            self.told.borrow_mut().push(message.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered(write: impl Fn(&mut String) -> fmt::Result) -> String {
        let mut text = String::new();
        write(&mut text).unwrap();
        text
    }

    #[test]
    fn picker_question_ends_in_its_question_mark() {
        let question = rendered(|text| Questions.format_select_prompt(text, "Which open entry?"));
        let answered = rendered(|text| {
            Questions.format_select_prompt_selection(text, "Which open entry?", "\"Draft\"")
        });

        assert_eq!(question, "Which open entry?");
        assert_eq!(answered, "Which open entry? \"Draft\"");
    }

    #[test]
    fn input_question_shows_its_default_in_brackets() {
        let named = rendered(|text| Questions.format_input_prompt(text, "Name?", Some("work")));
        let none = rendered(|text| Questions.format_input_prompt(text, "Purpose?", Some("")));
        let required = rendered(|text| Questions.format_input_prompt(text, "Currency?", None));
        let answered =
            rendered(|text| Questions.format_input_prompt_selection(text, "Name?", "Work"));

        assert_eq!(named, "Name? [work] ");
        assert_eq!(none, "Purpose? [none] ");
        assert_eq!(required, "Currency? ");
        assert_eq!(answered, "Name? Work");
    }

    #[test]
    fn detached_terminal_refuses_text_input() {
        let terminal = Terminal { attached: false };

        let error = terminal.input("Name?", Some("work")).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::Internal);
    }

    #[test]
    fn scripted_empty_input_takes_default() {
        let asker = Scripted::typing(["", "Paid work", ""]);

        assert_eq!(asker.input("Name?", Some("work")).unwrap(), "work");
        assert_eq!(asker.input("Name?", Some("work")).unwrap(), "Paid work");
        assert_eq!(asker.input("Currency?", None).unwrap(), "");
        assert_eq!(asker.questions(), ["Name?", "Name?", "Currency?"]);
    }

    #[test]
    fn detached_terminal_refuses_to_ask() {
        let terminal = Terminal { attached: false };

        let error = terminal.confirm("Forget it?", false).unwrap_err();

        assert!(!terminal.can_ask());
        assert_eq!(error.kind(), ErrorKind::Internal);
        assert_eq!(
            error.to_string(),
            "pace asked \"Forget it?\" without a terminal"
        );
    }
}
