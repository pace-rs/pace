//! The questions of a project's record in `pace setup`: its name, client,
//! role and billable flag, each with the record's value as the default, so
//! Enter keeps it.

use std::path::{Path, PathBuf};

use pace_core::{Catalog, ClientKey, ProjectRecord, RoleKey};
use pace_store::field_edit::{FieldEdit, FieldValue};

use crate::questions::{Answer, Optional, Question, QuestionKind, told};
use crate::setup::summary::SetupChange;

/// What the choice of a client or a role offers first.
const NONE_ITEM: &str = "none";

/// The flow that changes a project's record: it asks for each field and
/// ends with the changes of the fields whose answer differs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeRecordFlow {
    record_file: PathBuf,
    record: ProjectRecord,
    /// The catalog the record lives in: a client answer must leave it
    /// readable.
    catalog: Catalog,
    /// The clients the choice offers after `none`: key and name.
    clients: Vec<(ClientKey, String)>,
    /// The roles the choice offers after `none`: those in use, and the
    /// record's own even if archived.
    roles: Vec<(RoleKey, String)>,
    name: Option<String>,
    client: Optional<ClientKey>,
    role: Optional<RoleKey>,
    billable: Option<bool>,
}

impl ChangeRecordFlow {
    /// The flow that changes `record`, kept in `record_file`; its client
    /// and role are picked from `catalog`.
    #[must_use]
    pub fn new(catalog: &Catalog, record_file: &Path, record: ProjectRecord) -> Self {
        let clients = catalog
            .clients()
            .map(|client| (client.key.clone(), client.name.clone()))
            .collect();
        let roles = catalog
            .roles()
            .filter(|role| role.archived.is_none() || record.role.as_ref() == Some(&role.key))
            .map(|role| (role.key.clone(), role.name.clone()))
            .collect();
        Self {
            record_file: record_file.to_path_buf(),
            record,
            catalog: catalog.clone(),
            clients,
            roles,
            name: None,
            client: Optional::Unasked,
            role: Optional::Unasked,
            billable: None,
        }
    }

    /// The next question; `None` once every question has its answer.
    #[must_use]
    pub fn next_question(&self) -> Option<Question> {
        let question = |id, text: &str, kind| Question {
            id,
            text: text.to_owned(),
            kind,
            flag: "pace project edit",
            flag_answer: None,
            default_note: String::new(),
        };
        if self.name.is_none() {
            return Some(question(
                "record-name",
                "What is the project's name?",
                QuestionKind::Text {
                    default: Some(self.record.name.clone()),
                },
            ));
        }
        if self.client == Optional::Unasked {
            return Some(question(
                "record-client",
                "Which client is it for?",
                choice(&self.clients, self.record.client.as_ref()),
            ));
        }
        if self.role == Optional::Unasked {
            return Some(question(
                "record-role",
                "Which role does its time go to?",
                choice(&self.roles, self.record.role.as_ref()),
            ));
        }
        self.billable.is_none().then(|| {
            question(
                "record-billable",
                "Is its time billable?",
                QuestionKind::YesNo {
                    default: self.record.billable,
                },
            )
        })
    }

    /// Take the answer to the question [`ChangeRecordFlow::next_question`]
    /// gave.
    ///
    /// # Errors
    ///
    /// If the name is blank, or the pick is none of the items.
    pub fn take_answer(&mut self, answer: Answer) -> Result<(), String> {
        let Some(question) = self.next_question() else {
            return Err("every question has its answer".to_owned());
        };
        match (question.id, answer) {
            ("record-name", Answer::Text(name)) => {
                let name = name.trim();
                if name.is_empty() {
                    return Err("a project needs a name".to_owned());
                }
                self.name = Some(name.to_owned());
            }
            ("record-client", Answer::Choice(index)) => {
                let client = picked(&self.clients, index)?;
                self.check_client(client.as_ref())?;
                self.client = Optional::Answered(client);
            }
            ("record-role", Answer::Choice(index)) => {
                self.role = Optional::Answered(picked(&self.roles, index)?);
            }
            ("record-billable", Answer::YesNo(billable)) => self.billable = Some(billable),
            (id, answer) => return Err(format!("{answer:?} is no answer to `{id}`")),
        }
        Ok(())
    }

    /// Check that the catalog still reads with `client` as the record's
    /// client: a client's rates for this project tie it to that client.
    ///
    /// # Errors
    ///
    /// The catalog's refusal, told with its hint.
    fn check_client(&self, client: Option<&ClientKey>) -> Result<(), String> {
        let changed = ProjectRecord {
            client: client.cloned(),
            ..self.record.clone()
        };
        let projects = self
            .catalog
            .projects()
            .filter(|project| project.key != self.record.key)
            .cloned()
            .chain([changed])
            .collect();
        Catalog::new(
            self.catalog.roles().cloned().collect(),
            self.catalog.clients().cloned().collect(),
            projects,
        )
        .map(drop)
        .map_err(|error| told(&error))
    }

    /// The changes of the fields whose answer differs from the record.
    #[must_use]
    pub fn finish(self) -> Vec<SetupChange> {
        let file = self.record_file.as_path();
        let record = &self.record;
        let mut changes = Vec::new();
        if let Some(name) = self.name.filter(|name| *name != record.name) {
            changes.push(SetupChange::replacing(
                FieldEdit::set(file, &["name"], FieldValue::String(name)),
                record.name.clone(),
            ));
        }
        if let Optional::Answered(client) = self.client
            && client != record.client
        {
            changes.push(key_change(
                file,
                "client",
                record.client.as_ref().map(ToString::to_string),
                client.map(|key| key.to_string()),
            ));
        }
        if let Optional::Answered(role) = self.role
            && role != record.role
        {
            changes.push(key_change(
                file,
                "role",
                record.role.as_ref().map(ToString::to_string),
                role.map(|key| key.to_string()),
            ));
        }
        if let Some(billable) = self
            .billable
            .filter(|billable| *billable != record.billable)
        {
            changes.push(SetupChange::replacing(
                FieldEdit::set(file, &["billable"], FieldValue::Boolean(billable)),
                record.billable.to_string(),
            ));
        }
        changes
    }
}

/// The choice of one of `offered`, after `none`, with `current` picked.
fn choice<K: PartialEq + std::fmt::Display>(
    offered: &[(K, String)],
    current: Option<&K>,
) -> QuestionKind {
    let default = current
        .and_then(|current| offered.iter().position(|(key, _)| key == current))
        .map_or(0, |index| index + 1);
    QuestionKind::Choice {
        items: std::iter::once(NONE_ITEM.to_owned())
            .chain(offered.iter().map(|(key, name)| format!("{key}: {name}")))
            .collect(),
        default,
    }
}

/// The key the item at `index` of [`choice`] stands for; `None` for
/// `none`.
fn picked<K: Clone>(offered: &[(K, String)], index: usize) -> Result<Option<K>, String> {
    index.checked_sub(1).map_or(Ok(None), |index| {
        offered
            .get(index)
            .map(|(key, _)| Some(key.clone()))
            .ok_or_else(|| "pick one of the items".to_owned())
    })
}

/// The change of the key field `field` from `before` to `after`: set, or
/// removed for none.
fn key_change(
    file: &Path,
    field: &str,
    before: Option<String>,
    after: Option<String>,
) -> SetupChange {
    let edit = after.map_or_else(
        || FieldEdit::remove(file, &[field]),
        |key| FieldEdit::set(file, &[field], FieldValue::String(key)),
    );
    SetupChange { edit, before }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jiff::civil::date;
    use pace_core::{Client, Money, ProjectKey, Rate, Rates, Role, RoleKind, Rounding};

    use super::*;

    fn record() -> ProjectRecord {
        ProjectRecord {
            id: "0193a7c4-0000-7000-8000-000000000001".parse().unwrap(),
            key: "webshop".parse().unwrap(),
            name: "Webshop".to_owned(),
            client: Some("acme".parse().unwrap()),
            role: None,
            billable: true,
            budget: None,
            starts_on: None,
            ends_on: None,
            done_when: None,
            allow_overlap: false,
            archived: None,
        }
    }

    fn catalog() -> Catalog {
        catalog_with_project_rates(BTreeMap::new())
    }

    /// The catalog whose client `acme` has `project_rates`.
    fn catalog_with_project_rates(project_rates: BTreeMap<ProjectKey, Rates>) -> Catalog {
        let client = Client {
            key: "acme".parse().unwrap(),
            name: "ACME".to_owned(),
            currency: "EUR".parse().unwrap(),
            rounding: Rounding::NONE,
            rates: Rates::default(),
            project_rates,
        };
        let role = Role {
            id: "0193a7c4-0000-7000-8000-000000000002".parse().unwrap(),
            key: "work".parse().unwrap(),
            kind: RoleKind::Area,
            name: "Work".to_owned(),
            purpose: None,
            weekly_target: None,
            archived: None,
        };
        Catalog::new(vec![role], vec![client], vec![record()]).unwrap()
    }

    fn flow() -> ChangeRecordFlow {
        ChangeRecordFlow::new(&catalog(), Path::new("webshop.toml"), record())
    }

    /// Each question's default, as Enter gives it.
    fn defaults(flow: &mut ChangeRecordFlow) {
        while let Some(question) = flow.next_question() {
            flow.take_answer(question.default_answer().unwrap())
                .unwrap();
        }
    }

    #[test]
    fn record_values_are_defaults_and_enter_changes_nothing() {
        let mut flow = flow();
        let first = flow.next_question();

        defaults(&mut flow);

        assert_eq!(first.unwrap().id, "record-name");
        assert_eq!(flow.finish(), []);
    }

    #[test]
    fn client_question_picks_current_client() {
        let mut flow = flow();
        flow.take_answer(Answer::Text("Webshop".to_owned()))
            .unwrap();

        assert_eq!(
            flow.next_question().unwrap().kind,
            QuestionKind::Choice {
                items: vec!["none".to_owned(), "acme: ACME".to_owned()],
                default: 1
            }
        );
    }

    #[test]
    fn changed_fields_only_are_written() {
        let mut flow = flow();
        flow.take_answer(Answer::Text("Shop".to_owned())).unwrap();
        flow.take_answer(Answer::Choice(0)).unwrap();
        flow.take_answer(Answer::Choice(1)).unwrap();
        flow.take_answer(Answer::YesNo(true)).unwrap();

        let changes = flow.finish();

        assert_eq!(
            changes,
            [
                SetupChange::replacing(
                    FieldEdit::set(
                        "webshop.toml",
                        &["name"],
                        FieldValue::String("Shop".to_owned())
                    ),
                    "Webshop"
                ),
                SetupChange::replacing(FieldEdit::remove("webshop.toml", &["client"]), "acme"),
                SetupChange::added(FieldEdit::set(
                    "webshop.toml",
                    &["role"],
                    FieldValue::String("work".to_owned())
                )),
            ]
        );
    }

    #[test]
    fn blank_name_is_told() {
        let mut flow = flow();

        assert_eq!(
            flow.take_answer(Answer::Text("  ".to_owned())),
            Err("a project needs a name".to_owned())
        );
    }

    #[test]
    fn client_whose_catalog_would_not_read_is_told_and_asked_again() {
        let hourly = Money::parse_decimal("120", "EUR".parse().unwrap()).unwrap();
        let webshop_rates = Rates::new(vec![Rate {
            hourly,
            from: date(2026, 1, 1),
        }])
        .unwrap();
        let catalog = catalog_with_project_rates(BTreeMap::from([(
            "webshop".parse().unwrap(),
            webshop_rates,
        )]));
        let mut flow = ChangeRecordFlow::new(&catalog, Path::new("webshop.toml"), record());
        flow.take_answer(Answer::Text("Webshop".to_owned()))
            .unwrap();

        let message = flow.take_answer(Answer::Choice(0)).unwrap_err();

        assert_eq!(
            message,
            "the client `acme` has rates for the project `webshop`, which has no client: make \
             `acme` the project's client, or remove the rates"
        );
        assert_eq!(flow.next_question().unwrap().id, "record-client");
        defaults(&mut flow);
        assert_eq!(flow.finish(), []);
    }
}
