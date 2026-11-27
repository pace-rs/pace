//! The state of `pace setup` as it saves: the step, and each answer with
//! the question it answered.

use serde::{Deserialize, Serialize};

use crate::questions::Answer;

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

/// An answer that setup took, with the question it answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SavedAnswer {
    /// The question's ID, such as `time-zone`.
    pub question: String,
    /// The answer.
    pub answer: Answer,
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
    /// The step setup asks in.
    pub step: SetupStep,
    /// The answers so far.
    pub answers: Vec<SavedAnswer>,
}
