//! The state of `pace setup` as it saves: the step, and each answer with
//! the question it answered and the value it replaced.

use serde::{Deserialize, Serialize};

use crate::questions::{Answer, Question, QuestionKind};

/// The `version` of a saved setup that this pace writes and reads.
pub const SAVED_SETUP_VERSION: u32 = 1;

/// The step that setup asks in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SetupStep {
    /// The time zone and the week start.
    Settings,
    /// The roles.
    Roles,
    /// The clients.
    Clients,
    /// This directory.
    Directory,
    /// The summary, and whether to write it.
    Summary,
    /// Every question has its answer.
    Ended,
}

impl SetupStep {
    /// The step as the offer to continue names it: "the roles".
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Settings => "the settings",
            Self::Roles => "the roles",
            Self::Clients => "the clients",
            Self::Directory => "this directory",
            Self::Summary | Self::Ended => "the summary",
        }
    }
}

/// An answer that setup took, with the question it answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SavedAnswer {
    /// The question's ID, such as `time-zone`.
    pub question: String,
    /// The answer.
    pub answer: Answer,
    /// The value the answer replaced: the question's default, which is the
    /// value in the file, such as `system`; `None` if it had none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaced: Option<String>,
    /// For a choice, the item picked, such as ``give `acme` a new rate``.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picked: Option<String>,
}

impl SavedAnswer {
    /// `answer` to `question`, with the value it replaced and the item it
    /// picked.
    #[must_use]
    pub fn of(question: &Question, answer: Answer) -> Self {
        Self {
            question: question.id.to_owned(),
            replaced: shown_default(question),
            picked: picked_item(question, &answer),
            answer,
        }
    }

    /// What changed in `question` since this answer was saved, as the user
    /// reads it before the question is asked again; `None` if the answer
    /// still fits.
    #[must_use]
    pub fn changed_since(&self, question: &Question) -> Option<String> {
        let now = shown_default(question);
        if now != self.replaced {
            return Some(format!(
                "Since setup stopped, the value changed: {} → {}.",
                quoted(self.replaced.as_deref()),
                quoted(now.as_deref())
            ));
        }
        let picked = self.picked.as_deref()?;
        (picked_item(question, &self.answer).as_deref() != Some(picked))
            .then(|| format!("Since setup stopped, the items changed: pick `{picked}` again."))
    }
}

/// `value` in backticks, or `none`.
fn quoted(value: Option<&str>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| format!("`{value}`"))
}

/// The value Enter keeps in `question`: the default text, the default
/// item, or `yes` or `no`.
fn shown_default(question: &Question) -> Option<String> {
    match &question.kind {
        QuestionKind::Text { default } => default.clone(),
        QuestionKind::Choice { items, default } => items.get(*default).cloned(),
        QuestionKind::YesNo { default } => Some(if *default { "yes" } else { "no" }.to_owned()),
    }
}

/// The item of `question` that `answer` picks, if it is a choice.
fn picked_item(question: &Question, answer: &Answer) -> Option<String> {
    match (&question.kind, answer) {
        (QuestionKind::Choice { items, .. }, Answer::Choice(index)) => items.get(*index).cloned(),
        _ => None,
    }
}

/// Where setup stands, as it saves: the step, and the answers so far in
/// the order they were given.
///
/// The sub-flows hold the catalog, which doesn't serialize, so a saved
/// setup keeps the answers, and
/// [`SetupFlow::replayed`](crate::setup::SetupFlow::replayed) gives them
/// again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SavedSetup {
    /// The format's version, [`SAVED_SETUP_VERSION`].
    pub version: u32,
    /// The step setup asks in.
    pub step: SetupStep,
    /// The answers so far.
    pub answers: Vec<SavedAnswer>,
}
