//! The question machine: a [`Flow`] gives questions and takes answers
//! without I/O, and [`ask_flow`] drives it through the asker.
//!
//! A command that adds something builds its flow with the values its flags
//! gave, so the flow asks only for the rest. A flag's value passes the
//! same check as a typed answer: [`ask_flow`] hands it to
//! [`Flow::take_answer`], and a value that doesn't fit is a usage error
//! that names the flag. Each question has a default
//! that Enter takes. Without a terminal, [`ask_flow`] takes each default and
//! notes it, such as "name `work` from the key"; a question without a
//! default fails with a usage error that names its flag. The flow ends with
//! the [`FieldEdit`]s that write what the answers decided.

pub mod add_role;

use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_store::field_edit::FieldEdit;
use serde::{Deserialize, Serialize};

use crate::prompt::Ask;

/// What a question asks for, and its default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionKind {
    /// A line of text. Enter takes `default`; an empty default stands for
    /// none. Without a default, the user must type something.
    Text {
        /// What Enter takes.
        default: Option<String>,
    },
    /// One of `items`; Enter takes the one at `default`.
    Choice {
        /// What the user picks from.
        items: Vec<String>,
        /// The index of the item Enter takes.
        default: usize,
    },
    /// Yes or no; Enter takes `default`.
    YesNo {
        /// What Enter takes.
        default: bool,
    },
}

/// An answer to a question, as the user gave it. It serializes, so a flow
/// that is saved half-way, as `pace setup` will be, keeps its answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Answer {
    /// The text typed, or the default.
    Text(String),
    /// The index of the item picked.
    Choice(usize),
    /// Yes or no.
    YesNo(bool),
}

/// A question that a flow asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    /// What the flow calls it: `name`.
    pub id: &'static str,
    /// What the user reads: "What is the role's name?"
    pub text: String,
    /// What it asks for, and its default.
    pub kind: QuestionKind,
    /// The flag that answers it instead: `--name`.
    pub flag: &'static str,
    /// The answer the flag gave, if it was given: [`ask_flow`] takes it
    /// instead of asking.
    pub flag_answer: Option<Answer>,
    /// What pace says when it takes the default without a terminal:
    /// "name `work` from the key".
    pub default_note: String,
}

impl Question {
    /// The answer Enter gives; `None` if the question has no default.
    #[must_use]
    pub fn default_answer(&self) -> Option<Answer> {
        match &self.kind {
            QuestionKind::Text { default } => default.clone().map(Answer::Text),
            QuestionKind::Choice { default, .. } => Some(Answer::Choice(*default)),
            QuestionKind::YesNo { default } => Some(Answer::YesNo(*default)),
        }
    }
}

/// What a flow decided once every question has its answer: the edits that
/// write it, and what it made, for the command's sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ended<T> {
    /// The field edits that write the answers.
    pub edits: Vec<FieldEdit>,
    /// What the flow made, such as the new role.
    pub made: T,
}

/// A flow of questions without I/O: it gives the next question, takes its
/// answer, and ends with the field edits.
pub trait Flow {
    /// What the flow makes, such as a role.
    type Made;

    /// The next question; `None` once every question has its answer.
    fn next_question(&self) -> Option<Question>;

    /// Take the answer to the question [`Flow::next_question`] gave.
    ///
    /// # Errors
    ///
    /// If the answer doesn't fit: the message says why, and the question
    /// is asked again.
    fn take_answer(&mut self, answer: Answer) -> Result<(), String>;

    /// The edits and what the flow made; call it once
    /// [`Flow::next_question`] gives `None`.
    fn finish(self) -> Ended<Self::Made>;
}

/// What [`ask_flow`] ends with: the flow's edits and what it made, and a
/// note for each default it took without a terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked<T> {
    /// The field edits that write the answers.
    pub edits: Vec<FieldEdit>,
    /// What the flow made.
    pub made: T,
    /// The defaults taken without a terminal: "name `work` from the key".
    pub defaults: Vec<String>,
}

/// Drive `flow` through `asker` until every question has its answer.
///
/// A question its flag answered asks nothing: the flag's value goes to
/// [`Flow::take_answer`] as if typed. On a terminal, each other question is
/// asked; an answer that doesn't fit is told and asked again. Without one,
/// each question takes its default, and [`Asked::defaults`] names it.
///
/// # Errors
///
/// If the flow refuses a flag's value: a usage error that names the flag.
/// Without a terminal, a question with no default is a usage error that
/// names its flag. On a terminal, if the user gives no answer.
pub fn ask_flow<F: Flow>(mut flow: F, asker: &dyn Ask) -> PaceResult<Asked<F::Made>> {
    let mut defaults = Vec::new();
    while let Some(question) = flow.next_question() {
        if let Some(answer) = question.flag_answer.clone() {
            flow.take_answer(answer)
                .map_err(|message| flag_refused(&question, &message))?;
        } else if asker.can_ask() {
            ask_until_it_fits(&mut flow, &question, asker)?;
        } else {
            let answer = question
                .default_answer()
                .ok_or_else(|| no_terminal_for(&question))?;
            flow.take_answer(answer)
                .map_err(|message| default_refused(&question, &message))?;
            defaults.push(question.default_note);
        }
    }
    let Ended { edits, made } = flow.finish();
    Ok(Asked {
        edits,
        made,
        defaults,
    })
}

/// Ask `question` until `flow` takes the answer.
fn ask_until_it_fits(flow: &mut impl Flow, question: &Question, asker: &dyn Ask) -> PaceResult<()> {
    loop {
        let answer = match &question.kind {
            QuestionKind::Text { default } => asker
                .input(&question.text, default.as_deref())
                .map(Answer::Text)?,
            QuestionKind::Choice { items, default } => asker
                .choose(&question.text, items, *default)
                .map(Answer::Choice)?,
            QuestionKind::YesNo { default } => {
                asker.confirm(&question.text, *default).map(Answer::YesNo)?
            }
        };
        match flow.take_answer(answer) {
            Ok(()) => return Ok(()),
            Err(message) => asker.tell(&message),
        }
    }
}

/// The error for a question without a default and without a terminal.
fn no_terminal_for(question: &Question) -> Box<PaceError> {
    PaceError::new(
        ErrorKind::InvalidInput,
        "pace has no terminal to ask \"{question}\", which has no default",
    )
    .attach_context("question", question.text.clone())
    .attach_hint(format!("answer it with `{}`", question.flag))
    .attach_exit_code(ExitCode::Usage)
}

/// The error for a flag's value that the flow refused, such as a blank
/// `--name`.
fn flag_refused(question: &Question, message: &str) -> Box<PaceError> {
    PaceError::new(
        ErrorKind::InvalidInput,
        "`{flag} {value}` doesn't fit: {message}",
    )
    .attach_context("flag", question.flag)
    .attach_context("value", flag_value(question))
    .attach_context("message", message)
    .attach_hint(format!(
        "give `{}` another value, or leave the flag out",
        question.flag
    ))
    .attach_exit_code(ExitCode::Usage)
}

/// The flag's value as the command line had it: text in quotes, so a
/// blank one shows; a choice by its item.
fn flag_value(question: &Question) -> String {
    match (&question.flag_answer, &question.kind) {
        (Some(Answer::Text(text)), _) => format!("\"{text}\""),
        (Some(Answer::Choice(index)), QuestionKind::Choice { items, .. }) => items
            .get(*index)
            .map_or_else(|| index.to_string(), Clone::clone),
        (Some(Answer::Choice(index)), _) => index.to_string(),
        (Some(Answer::YesNo(yes)), _) => if *yes { "yes" } else { "no" }.to_owned(),
        (None, _) => String::new(),
    }
}

/// The error for a default that its own flow refused: a bug.
fn default_refused(question: &Question, message: &str) -> Box<PaceError> {
    PaceError::new(
        ErrorKind::Internal,
        "the default of \"{question}\" doesn't fit: {message}",
    )
    .attach_context("question", question.text.clone())
    .attach_context("message", message)
    .ask_report()
}

#[cfg(test)]
mod tests {
    use pace_store::field_edit::FieldValue;

    use super::*;
    use crate::prompt::Scripted;

    /// A flow of two questions: a code without a default, which must be
    /// three letters, and a colour with the default `blue`.
    #[derive(Debug, Default)]
    struct Paint {
        code_flag: Option<String>,
        code: Option<String>,
        colour: Option<String>,
        colour_default: Option<String>,
    }

    impl Flow for Paint {
        type Made = (String, String);

        fn next_question(&self) -> Option<Question> {
            if self.code.is_none() {
                return Some(Question {
                    id: "code",
                    text: "Which code?".to_owned(),
                    kind: QuestionKind::Text { default: None },
                    flag: "--code",
                    flag_answer: self.code_flag.clone().map(Answer::Text),
                    default_note: String::new(),
                });
            }
            self.colour.is_none().then(|| Question {
                id: "colour",
                text: "Which colour?".to_owned(),
                kind: QuestionKind::Text {
                    default: Some(
                        self.colour_default
                            .clone()
                            .unwrap_or_else(|| "blue".to_owned()),
                    ),
                },
                flag: "--colour",
                flag_answer: None,
                default_note: "colour `blue`".to_owned(),
            })
        }

        fn take_answer(&mut self, answer: Answer) -> Result<(), String> {
            let Answer::Text(text) = answer else {
                return Err("type it".to_owned());
            };
            if self.code.is_none() {
                if text.len() != 3 {
                    return Err(format!("`{text}` is no code of three letters"));
                }
                self.code = Some(text);
            } else if text.is_empty() {
                return Err("a colour has a name".to_owned());
            } else {
                self.colour = Some(text);
            }
            Ok(())
        }

        fn finish(self) -> Ended<Self::Made> {
            let code = self.code.unwrap_or_default();
            let colour = self.colour.unwrap_or_default();
            Ended {
                edits: vec![FieldEdit::set(
                    "paint.toml",
                    &[&code],
                    FieldValue::String(colour.clone()),
                )],
                made: (code, colour),
            }
        }
    }

    #[test]
    fn scripted_answers_end_in_field_edits() {
        let asker = Scripted::typing(["abc", "red"]);

        let result = ask_flow(Paint::default(), &asker).unwrap();

        assert_eq!(result.made, ("abc".to_owned(), "red".to_owned()));
        assert_eq!(
            result.edits,
            [FieldEdit::set(
                "paint.toml",
                &["abc"],
                FieldValue::String("red".to_owned())
            )]
        );
        assert_eq!(result.defaults, Vec::<String>::new());
        assert_eq!(asker.questions(), ["Which code?", "Which colour?"]);
    }

    #[test]
    fn enter_takes_default_on_terminal_without_note() {
        let asker = Scripted::typing(["abc", ""]);

        let result = ask_flow(Paint::default(), &asker).unwrap();

        assert_eq!(result.made.1, "blue");
        assert_eq!(result.defaults, Vec::<String>::new());
    }

    #[test]
    fn answer_that_does_not_fit_is_told_and_asked_again() {
        let asker = Scripted::typing(["abcd", "abc", ""]);

        let result = ask_flow(Paint::default(), &asker).unwrap();

        assert_eq!(result.made.0, "abc");
        assert_eq!(asker.told(), ["`abcd` is no code of three letters"]);
        assert_eq!(
            asker.questions(),
            ["Which code?", "Which code?", "Which colour?"]
        );
    }

    #[test]
    fn flag_skips_its_question() {
        let flow = Paint {
            code_flag: Some("abc".to_owned()),
            ..Paint::default()
        };
        let asker = Scripted::typing(["red"]);

        let result = ask_flow(flow, &asker).unwrap();

        assert_eq!(result.made, ("abc".to_owned(), "red".to_owned()));
        assert_eq!(asker.questions(), ["Which colour?"]);
    }

    #[test]
    fn flag_that_does_not_fit_is_usage_error_naming_flag() {
        let flow = Paint {
            code_flag: Some("abcd".to_owned()),
            ..Paint::default()
        };
        let asker = Scripted::typing(["abc", "red"]);

        let error = ask_flow(flow, &asker).unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(
            error.to_string(),
            "`--code \"abcd\"` doesn't fit: `abcd` is no code of three letters"
        );
        assert_eq!(
            error.hint(),
            Some("give `--code` another value, or leave the flag out")
        );
        assert_eq!(asker.questions(), Vec::<String>::new());
    }

    #[test]
    fn no_terminal_takes_defaults_and_names_them() {
        let flow = Paint {
            code: Some("abc".to_owned()),
            ..Paint::default()
        };
        let asker = Scripted::without_terminal();

        let result = ask_flow(flow, &asker).unwrap();

        assert_eq!(result.made.1, "blue");
        assert_eq!(result.defaults, ["colour `blue`"]);
        assert_eq!(asker.questions(), Vec::<String>::new());
    }

    #[test]
    fn no_terminal_and_no_default_is_usage_error_naming_flag() {
        let error = ask_flow(Paint::default(), &Scripted::without_terminal()).unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(
            error.to_string(),
            "pace has no terminal to ask \"Which code?\", which has no default"
        );
        assert_eq!(error.hint(), Some("answer it with `--code`"));
    }

    #[test]
    fn default_its_flow_refuses_is_internal_error() {
        let flow = Paint {
            code: Some("abc".to_owned()),
            colour_default: Some(String::new()),
            ..Paint::default()
        };

        let error = ask_flow(flow, &Scripted::without_terminal()).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::Internal);
    }

    #[test]
    fn choice_and_yes_no_have_their_defaults() {
        let choice = Question {
            id: "kind",
            text: "Which kind?".to_owned(),
            kind: QuestionKind::Choice {
                items: vec!["area".to_owned(), "meta".to_owned()],
                default: 1,
            },
            flag: "--kind",
            flag_answer: None,
            default_note: "kind `meta`".to_owned(),
        };
        let yes_no = Question {
            kind: QuestionKind::YesNo { default: true },
            ..choice.clone()
        };

        assert_eq!(choice.default_answer(), Some(Answer::Choice(1)));
        assert_eq!(yes_no.default_answer(), Some(Answer::YesNo(true)));
    }

    #[test]
    fn answer_serializes_for_saved_flow() {
        let text = serde_json::to_string(&Answer::Text("Work".to_owned())).unwrap();

        assert_eq!(text, r#"{"text":"Work"}"#);
        assert_eq!(
            serde_json::from_str::<Answer>(&text).unwrap(),
            Answer::Text("Work".to_owned())
        );
    }
}
