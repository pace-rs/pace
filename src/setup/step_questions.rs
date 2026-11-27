//! The questions that the steps of `pace setup` ask themselves: the
//! settings, whether to add a role, what to do with the clients, the offer
//! for this directory and the summary.

use crate::output::listed;
use crate::questions::{Question, QuestionKind};
use crate::setup::{DirectoryOffer, SetupFlow, WEEK_DAYS};

impl SetupFlow {
    /// The question of the first step: the time zone, then the week start.
    pub(crate) fn settings_question(&self) -> Question {
        if self.time_zone.is_none() {
            return question(
                "time-zone",
                "Which time zone do you live in, such as Europe/Berlin, or system?".to_owned(),
                QuestionKind::Text {
                    default: Some(self.inputs.time_zone.clone()),
                },
                "pace config set time-zone",
            );
        }
        let current = WEEK_DAYS
            .iter()
            .position(|day| *day == self.inputs.week_start)
            .unwrap_or(0);
        question(
            "week-start",
            "On which day does your week start?".to_owned(),
            QuestionKind::Choice {
                items: WEEK_DAYS.iter().map(|day| (*day).to_owned()).collect(),
                default: current,
            },
            "pace config set week-start",
        )
    }

    /// The question whether to add a role, with the roles in use.
    pub(crate) fn roles_question(&self) -> Question {
        let keys: Vec<String> = self
            .catalog
            .roles()
            .filter(|role| role.archived.is_none())
            .map(|role| format!("`{}`", role.key))
            .collect();
        let text = match keys.as_slice() {
            [] => "You have no roles yet. Add one?".to_owned(),
            [one] => format!("You have the role {one}. Add another?"),
            _ => format!("You have the roles {}. Add another?", listed(&keys)),
        };
        question(
            "add-role",
            text,
            QuestionKind::YesNo { default: false },
            "pace role add",
        )
    }

    /// The question what to do with the clients: go on, add one, or give
    /// one a new rate.
    pub(crate) fn clients_question(&self) -> Question {
        let keys: Vec<String> = self
            .catalog
            .clients()
            .map(|client| format!("`{}`", client.key))
            .collect();
        let text = match keys.as_slice() {
            [] => "You have no clients yet. What next?".to_owned(),
            [one] => format!("You have the client {one}. What next?"),
            _ => format!("You have the clients {}. What next?", listed(&keys)),
        };
        let items = ["go on".to_owned(), "add a client".to_owned()]
            .into_iter()
            .chain(keys.iter().map(|key| format!("give {key} a new rate")))
            .collect();
        question(
            "clients",
            text,
            QuestionKind::Choice { items, default: 0 },
            "pace client add",
        )
    }

    /// The question whether to take the offer for this directory.
    pub(crate) fn directory_question(&self) -> Question {
        let dir = self.inputs.dir.display();
        let text = match &self.offer {
            DirectoryOffer::MakeProject => format!("Make {dir} a project?"),
            DirectoryOffer::Link(key) => {
                format!("Link {dir} to the project `{key}` of its manifest?")
            }
            DirectoryOffer::ChangeRecord(record) => format!(
                "Change the record of the project `{}` ({})?",
                record.key, record.name
            ),
        };
        question(
            "directory",
            text,
            QuestionKind::YesNo { default: false },
            "pace project init",
        )
    }

    /// The question of the summary: write these changes?
    pub(crate) fn summary_question() -> Question {
        question(
            "accept",
            "Write these changes?".to_owned(),
            QuestionKind::YesNo { default: true },
            "pace setup",
        )
    }
}

/// A question of setup's own: no flag answers it, and `command` is what
/// does its part without a terminal.
pub(crate) const fn question(
    id: &'static str,
    text: String,
    kind: QuestionKind,
    command: &'static str,
) -> Question {
    Question {
        id,
        text,
        kind,
        flag: command,
        flag_answer: None,
        default_note: String::new(),
    }
}

/// The question of a new role's or client's key, which has no default.
pub(crate) fn key_question(id: &'static str, text: &str, command: &'static str) -> Question {
    question(
        id,
        text.to_owned(),
        QuestionKind::Text { default: None },
        command,
    )
}
