//! The questions of `pace project add`: the project's name, client, role
//! and billable flag, each with a default; its budget, days and result come
//! from flags only.

use std::path::{Path, PathBuf};

use jiff::{SignedDuration, civil::Date};
use pace_core::{Catalog, ClientKey, ProjectId, ProjectKey, ProjectRecord, RoleKey};
use pace_error::{ErrorKind, PaceError, PaceResult, closest_name};
use pace_store::field_edit::{FieldEdit, FieldValue};
use pace_time::parse_duration;

use crate::questions::{Answer, Ended, Flow, Optional, Question, QuestionKind, told};

/// What the choice of a client or a role offers first, and Enter takes.
const NONE_ITEM: &str = "none";

/// What the flags of `pace project add` gave, as typed.
///
/// What they leave out is asked, but for the budget, the days, the result
/// and the overlap, which only flags give. Each value passes its question's
/// check.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectFlags {
    /// `--name`
    pub name: Option<String>,
    /// `--client`, a client's key
    pub client: Option<String>,
    /// `--role`, a role's key
    pub role: Option<String>,
    /// `--billable` (`true`) or `--not-billable` (`false`)
    pub billable: Option<bool>,
    /// `--budget`, such as `120h`
    pub budget: Option<String>,
    /// `--starts-on`, such as `2026-12-01`
    pub starts_on: Option<String>,
    /// `--ends-on`, such as `2027-03-31`
    pub ends_on: Option<String>,
    /// `--done-when`, the result that makes the project done
    pub done_when: Option<String>,
    /// `--allow-overlap`
    pub allow_overlap: bool,
}

/// A client or a role the flow offers: its key and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Offered<K> {
    key: K,
    name: String,
}

/// The flow of a new project record: it asks for what [`ProjectFlags`]
/// leave open and ends with the fields of `projects/<key>.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddProjectFlow {
    record_file: PathBuf,
    id: ProjectId,
    key: ProjectKey,
    flags: ProjectFlags,
    /// Every client, which the client question offers.
    clients: Vec<Offered<ClientKey>>,
    /// The roles in use, which the role question offers.
    roles_in_use: Vec<Offered<RoleKey>>,
    /// Every role's key, archived or not, which `--role` may name.
    role_keys: Vec<RoleKey>,
    /// What Enter takes for the name, and the note that names where it
    /// comes from; the key, unless [`AddProjectFlow::with_name_default`]
    /// says otherwise.
    name_default: (String, String),
    /// Whether the client and the role are asked with nothing to pick, as
    /// `pace project init` adds an item for a new one.
    asks_without_offers: bool,
    name: Option<String>,
    client: Optional<ClientKey>,
    role: Optional<RoleKey>,
    billable: Option<bool>,
    budget: Optional<SignedDuration>,
    starts_on: Optional<Date>,
    ends_on: Optional<Date>,
    done_when: Optional<String>,
}

impl AddProjectFlow {
    /// The flow of the project `key` with the ID `id`, written to
    /// `record_file`; `flags` answer their questions, and the clients and
    /// roles of `catalog` are what the project may name.
    ///
    /// # Errors
    ///
    /// If `catalog` has a record of `key` already, archived or not: the
    /// error names `pace project edit`.
    pub fn new(
        catalog: &Catalog,
        record_file: &Path,
        key: ProjectKey,
        id: ProjectId,
        flags: ProjectFlags,
    ) -> PaceResult<Self> {
        if catalog.project(&key).is_some() {
            return Err(PaceError::new(
                ErrorKind::InvalidInput,
                "the project `{key}` has a record already",
            )
            .attach_context("key", key.to_string())
            .attach_hint(format!("change it with `pace project edit {key}`")));
        }
        Ok(Self {
            record_file: record_file.to_path_buf(),
            id,
            flags,
            clients: catalog
                .clients()
                .map(|client| Offered {
                    key: client.key.clone(),
                    name: client.name.clone(),
                })
                .collect(),
            roles_in_use: catalog
                .roles()
                .filter(|role| role.archived.is_none())
                .map(|role| Offered {
                    key: role.key.clone(),
                    name: role.name.clone(),
                })
                .collect(),
            role_keys: catalog.roles().map(|role| role.key.clone()).collect(),
            name_default: (key.to_string(), format!("name `{key}` from the key")),
            asks_without_offers: false,
            name: None,
            client: Optional::Unasked,
            role: Optional::Unasked,
            billable: None,
            budget: Optional::Unasked,
            starts_on: Optional::Unasked,
            ends_on: Optional::Unasked,
            done_when: Optional::Unasked,
            key,
        })
    }

    /// This flow with `name` as what Enter takes for the name, and
    /// `note` as what pace says when it takes it without a terminal.
    #[must_use]
    pub fn with_name_default(mut self, name: String, note: String) -> Self {
        self.name_default = (name, note);
        self
    }

    /// This flow asking the client and the role even with nothing to
    /// pick but `none`, for a caller that adds an item of its own.
    #[must_use]
    pub const fn asking_without_offers(mut self) -> Self {
        self.asks_without_offers = true;
        self
    }

    /// Offer the client `key` named `name`, made while the flow runs.
    pub fn offer_client(&mut self, key: ClientKey, name: String) {
        self.clients.push(Offered { key, name });
    }

    /// Offer the role `key` named `name`, made while the flow runs.
    pub fn offer_role(&mut self, key: RoleKey, name: String) {
        self.role_keys.push(key.clone());
        self.roles_in_use.push(Offered { key, name });
    }

    /// The question of the client; `None` once answered, or if there is
    /// no client to pick and `--client` names none.
    fn client_question(&self) -> Option<Question> {
        let open = self.client == Optional::Unasked
            && (self.asks_without_offers
                || !self.clients.is_empty()
                || self.flags.client.is_some());
        open.then(|| Question {
            id: "client",
            text: "Which client is it for?".to_owned(),
            kind: QuestionKind::Choice {
                items: offered_items(&self.clients),
                default: 0,
            },
            flag: "--client",
            flag_answer: self.flags.client.clone().map(Answer::Text),
            default_note: "no client".to_owned(),
        })
    }

    /// The question of the role; `None` once answered, or if there is no
    /// role in use to pick and `--role` names none.
    fn role_question(&self) -> Option<Question> {
        let open = self.role == Optional::Unasked
            && (self.asks_without_offers
                || !self.roles_in_use.is_empty()
                || self.flags.role.is_some());
        open.then(|| Question {
            id: "role",
            text: "Which role does its time go to?".to_owned(),
            kind: QuestionKind::Choice {
                items: offered_items(&self.roles_in_use),
                default: 0,
            },
            flag: "--role",
            flag_answer: self.flags.role.clone().map(Answer::Text),
            default_note: "no role".to_owned(),
        })
    }

    /// The question of the billable flag: yes with a client, else no.
    fn billable_question(&self) -> Option<Question> {
        let has_client = matches!(self.client, Optional::Answered(Some(_)));
        self.billable.is_none().then(|| Question {
            id: "billable",
            text: "Is its time billable?".to_owned(),
            kind: QuestionKind::YesNo {
                default: has_client,
            },
            flag: if has_client {
                "--not-billable"
            } else {
                "--billable"
            },
            flag_answer: self.flags.billable.map(Answer::YesNo),
            default_note: if has_client {
                "billable, as it has a client"
            } else {
                "not billable, as it has no client"
            }
            .to_owned(),
        })
    }

    /// The questions only a flag answers: the budget, the days and the
    /// result. pace doesn't ask them, but checks each flag's value.
    fn flag_question(&self) -> Option<Question> {
        let flags = &self.flags;
        let asks = [
            (
                self.budget == Optional::Unasked,
                "budget",
                "--budget",
                &flags.budget,
            ),
            (
                self.starts_on == Optional::Unasked,
                "starts-on",
                "--starts-on",
                &flags.starts_on,
            ),
            (
                self.ends_on == Optional::Unasked,
                "ends-on",
                "--ends-on",
                &flags.ends_on,
            ),
            (
                self.done_when == Optional::Unasked,
                "done-when",
                "--done-when",
                &flags.done_when,
            ),
        ];
        let (_, id, flag, value) = asks
            .into_iter()
            .find(|(unasked, _, _, value)| *unasked && value.is_some())?;
        Some(Question {
            id,
            text: format!("What is the project's {id}?"),
            kind: QuestionKind::Text { default: None },
            flag,
            flag_answer: value.clone().map(Answer::Text),
            default_note: String::new(),
        })
    }

    /// Take `answer` as the project's client: an item of the choice, or a
    /// key as `--client` gives it.
    fn take_client(&mut self, answer: Answer) -> Result<(), String> {
        let known: Vec<&str> = self
            .clients
            .iter()
            .map(|client| client.key.as_str())
            .collect();
        let picked = match answer {
            Answer::Choice(index) => picked_key(&self.clients, index)?,
            Answer::Text(text) => Some(known_key(&text, &known, "client")?),
            Answer::YesNo(_) => return Err("pick a client".to_owned()),
        };
        self.client = Optional::Answered(picked);
        Ok(())
    }

    /// Take `answer` as the project's role: an item of the choice, or a key
    /// as `--role` gives it, also of an archived role.
    fn take_role(&mut self, answer: Answer) -> Result<(), String> {
        let known: Vec<&str> = self.role_keys.iter().map(RoleKey::as_str).collect();
        let picked = match answer {
            Answer::Choice(index) => picked_key(&self.roles_in_use, index)?,
            Answer::Text(text) => Some(known_key(&text, &known, "role")?),
            Answer::YesNo(_) => return Err("pick a role".to_owned()),
        };
        self.role = Optional::Answered(picked);
        Ok(())
    }
}

impl Flow for AddProjectFlow {
    type Made = ProjectRecord;

    fn next_question(&self) -> Option<Question> {
        if self.name.is_none() {
            return Some(Question {
                id: "name",
                text: "What is the project's name?".to_owned(),
                kind: QuestionKind::Text {
                    default: Some(self.name_default.0.clone()),
                },
                flag: "--name",
                flag_answer: self.flags.name.clone().map(Answer::Text),
                default_note: self.name_default.1.clone(),
            });
        }
        self.client_question()
            .or_else(|| self.role_question())
            .or_else(|| self.billable_question())
            .or_else(|| self.flag_question())
    }

    fn take_answer(&mut self, answer: Answer) -> Result<(), String> {
        let Some(question) = self.next_question() else {
            return Err("every question has its answer".to_owned());
        };
        match (question.id, answer) {
            ("name", Answer::Text(name)) => {
                let name = name.trim();
                if name.is_empty() {
                    return Err("a project needs a name".to_owned());
                }
                self.name = Some(name.to_owned());
            }
            ("client", answer) => self.take_client(answer)?,
            ("role", answer) => self.take_role(answer)?,
            ("billable", Answer::YesNo(billable)) => self.billable = Some(billable),
            ("budget", Answer::Text(budget)) => {
                let budget = parse_duration(budget.trim()).map_err(|error| told(&error))?;
                self.budget = Optional::Answered(Some(budget));
            }
            ("starts-on", Answer::Text(day)) => {
                self.starts_on = Optional::Answered(Some(parsed_day(&day)?));
            }
            ("ends-on", Answer::Text(day)) => {
                self.ends_on = Optional::Answered(Some(parsed_day(&day)?));
            }
            ("done-when", Answer::Text(result)) => {
                let result = result.trim();
                if result.is_empty() {
                    return Err("write the result that makes the project done".to_owned());
                }
                self.done_when = Optional::Answered(Some(result.to_owned()));
            }
            (id, answer) => return Err(format!("{answer:?} is no answer to `{id}`")),
        }
        Ok(())
    }

    fn finish(self) -> PaceResult<Ended<ProjectRecord>> {
        let record = ProjectRecord {
            id: self.id,
            key: self.key,
            name: self.name.unwrap_or_default(),
            client: self.client.into_value(),
            role: self.role.into_value(),
            billable: self.billable.unwrap_or(false),
            budget: self.budget.into_value(),
            starts_on: self.starts_on.into_value(),
            ends_on: self.ends_on.into_value(),
            done_when: self.done_when.into_value(),
            allow_overlap: self.flags.allow_overlap,
            archived: None,
        };
        Ok(Ended {
            edits: record_edits(&self.record_file, &record),
            made: record,
        })
    }
}

/// The items of the choice of `offered`: `none` first, then each as
/// `key: name`.
fn offered_items<K: std::fmt::Display>(offered: &[Offered<K>]) -> Vec<String> {
    std::iter::once(NONE_ITEM.to_owned())
        .chain(
            offered
                .iter()
                .map(|offered| format!("{}: {}", offered.key, offered.name)),
        )
        .collect()
}

/// The key the item at `index` of [`offered_items`] stands for; `None` for
/// `none`.
fn picked_key<K: Clone>(offered: &[Offered<K>], index: usize) -> Result<Option<K>, String> {
    index.checked_sub(1).map_or(Ok(None), |index| {
        offered
            .get(index)
            .map(|offered| Some(offered.key.clone()))
            .ok_or_else(|| "pick one of the items".to_owned())
    })
}

/// `text` as the key of one of `known`, a `noun` such as `client`.
///
/// # Errors
///
/// If `text` is no key, or none of `known`: the message names the closest
/// key, else the command that adds one.
fn known_key<K: std::str::FromStr<Err = Box<PaceError>>>(
    text: &str,
    known: &[&str],
    noun: &str,
) -> Result<K, String> {
    let text = text.trim();
    if !known.contains(&text) {
        let hint = closest_name(text, known.iter().copied()).map_or_else(
            || format!("`pace {noun} add {text}` adds it"),
            |closest| format!("did you mean `{closest}`?"),
        );
        return Err(told(
            &PaceError::new(
                ErrorKind::InvalidInput,
                "pace doesn't know the {noun} `{key}`",
            )
            .attach_context("noun", noun)
            .attach_context("key", text)
            .attach_hint(hint),
        ));
    }
    text.parse().map_err(|error: Box<PaceError>| told(&error))
}

/// `text` as a day, such as `2026-12-01`.
fn parsed_day(text: &str) -> Result<Date, String> {
    let text = text.trim();
    text.parse()
        .map_err(|_| format!("`{text}` is not a day: write it like `2026-12-01`"))
}

/// The field edits that write `record` into `record_file`, its own file.
fn record_edits(record_file: &Path, record: &ProjectRecord) -> Vec<FieldEdit> {
    let field = |name: &str, value: FieldValue| FieldEdit::set(record_file, &[name], value);
    let text = |text: &str| FieldValue::String(text.to_owned());
    let mut edits = vec![
        field("id", text(&record.id.to_string())),
        field("key", text(record.key.as_str())),
        field("name", text(&record.name)),
    ];
    if let Some(client) = &record.client {
        edits.push(field("client", text(client.as_str())));
    }
    if let Some(role) = &record.role {
        edits.push(field("role", text(role.as_str())));
    }
    edits.push(field("billable", FieldValue::Boolean(record.billable)));
    if let Some(budget) = record.budget {
        edits.push(field("budget", FieldValue::String(format!("{budget:#}"))));
    }
    if let Some(day) = record.starts_on {
        edits.push(field("starts-on", FieldValue::Date(day)));
    }
    if let Some(day) = record.ends_on {
        edits.push(field("ends-on", FieldValue::Date(day)));
    }
    if let Some(result) = &record.done_when {
        edits.push(field("done-when", text(result)));
    }
    if record.allow_overlap {
        edits.push(field("allow-overlap", FieldValue::Boolean(true)));
    }
    edits
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jiff::civil::date;
    use pace_core::{ArchiveReason, Archived, Client, Rates, Role, RoleKind, Rounding};
    use pace_error::ExitCode;

    use super::*;
    use crate::prompt::Scripted;
    use crate::questions::ask_flow;

    const ID: &str = "0193a7c4-0000-7000-8000-000000000001";

    fn client(key: &str, name: &str) -> Client {
        Client {
            key: key.parse().unwrap(),
            name: name.to_owned(),
            currency: "EUR".parse().unwrap(),
            rounding: Rounding::NONE,
            rates: Rates::default(),
            project_rates: BTreeMap::new(),
        }
    }

    fn role(key: &str, id: &str, archived: Option<Archived>) -> Role {
        Role {
            id: id.parse().unwrap(),
            key: key.parse().unwrap(),
            kind: RoleKind::Area,
            name: key.to_uppercase(),
            purpose: None,
            weekly_target: None,
            archived,
        }
    }

    /// A catalog with the client `acme`, the role `work` and the archived
    /// role `old`.
    fn catalog() -> Catalog {
        Catalog::new(
            vec![
                role("work", "0193a7c4-0000-7000-8000-0000000000a1", None),
                role(
                    "old",
                    "0193a7c4-0000-7000-8000-0000000000a2",
                    Some(Archived {
                        on: date(2026, 1, 1),
                        reason: ArchiveReason::Ended,
                    }),
                ),
            ],
            vec![client("acme", "ACME GmbH")],
            Vec::new(),
        )
        .unwrap()
    }

    fn flow_in(catalog: &Catalog, flags: ProjectFlags) -> AddProjectFlow {
        AddProjectFlow::new(
            catalog,
            Path::new("projects/webshop.toml"),
            "webshop".parse().unwrap(),
            ID.parse().unwrap(),
            flags,
        )
        .unwrap()
    }

    fn set(key: &str, value: FieldValue) -> FieldEdit {
        FieldEdit::set("projects/webshop.toml", &[key], value)
    }

    fn text(value: &str) -> FieldValue {
        FieldValue::String(value.to_owned())
    }

    #[test]
    fn every_flag_given_asks_nothing_and_writes_each_field() {
        let flags = ProjectFlags {
            name: Some("Webshop relaunch".to_owned()),
            client: Some("acme".to_owned()),
            role: Some("work".to_owned()),
            billable: Some(true),
            budget: Some("120h".to_owned()),
            starts_on: Some("2026-12-01".to_owned()),
            ends_on: Some("2027-03-31".to_owned()),
            done_when: Some("the new shop is live".to_owned()),
            allow_overlap: true,
        };
        let asker = Scripted::typing([]);

        let result = ask_flow(flow_in(&catalog(), flags), &asker).unwrap();

        assert_eq!(asker.questions(), Vec::<String>::new());
        assert_eq!(
            result.edits,
            [
                set("id", text(ID)),
                set("key", text("webshop")),
                set("name", text("Webshop relaunch")),
                set("client", text("acme")),
                set("role", text("work")),
                set("billable", FieldValue::Boolean(true)),
                set("budget", text("120h")),
                set("starts-on", FieldValue::Date(date(2026, 12, 1))),
                set("ends-on", FieldValue::Date(date(2027, 3, 31))),
                set("done-when", text("the new shop is live")),
                set("allow-overlap", FieldValue::Boolean(true)),
            ]
        );
    }

    #[test]
    fn without_terminal_takes_defaults_and_names_them() {
        let result = ask_flow(
            flow_in(&catalog(), ProjectFlags::default()),
            &Scripted::without_terminal(),
        )
        .unwrap();

        assert_eq!(
            result.defaults,
            [
                "name `webshop` from the key",
                "no client",
                "no role",
                "not billable, as it has no client"
            ]
        );
        assert_eq!(result.made.name, "webshop");
        assert!(!result.made.billable);
    }

    #[test]
    fn billable_defaults_to_yes_with_client() {
        let flags = ProjectFlags {
            client: Some("acme".to_owned()),
            ..ProjectFlags::default()
        };

        let result = ask_flow(flow_in(&catalog(), flags), &Scripted::without_terminal()).unwrap();

        assert!(result.made.billable);
        assert_eq!(
            result.defaults.last().map(String::as_str),
            Some("billable, as it has a client")
        );
    }

    #[test]
    fn terminal_offers_none_and_clients_and_roles_in_use() {
        let asker = Scripted::typing(["Webshop"])
            .and_choosing([1, 1])
            .and_confirming([true]);

        let result = ask_flow(flow_in(&catalog(), ProjectFlags::default()), &asker).unwrap();

        assert_eq!(
            asker.offered(),
            [vec!["none", "acme: ACME GmbH"], vec!["none", "work: WORK"]]
        );
        assert_eq!(result.made.client, Some("acme".parse().unwrap()));
        assert_eq!(result.made.role, Some("work".parse().unwrap()));
        assert!(result.made.billable);
    }

    #[test]
    fn empty_catalog_asks_neither_client_nor_role() {
        let asker = Scripted::typing([""]).and_confirming([false]);

        let result = ask_flow(
            flow_in(&Catalog::default(), ProjectFlags::default()),
            &asker,
        )
        .unwrap();

        assert_eq!(
            asker.questions(),
            ["What is the project's name?", "Is its time billable?"]
        );
        assert_eq!(result.made.client, None);
    }

    #[test]
    fn unknown_client_flag_is_usage_error_naming_closest_key() {
        let flags = ProjectFlags {
            client: Some("acmee".to_owned()),
            ..ProjectFlags::default()
        };

        let error =
            ask_flow(flow_in(&catalog(), flags), &Scripted::without_terminal()).unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(
            error.to_string(),
            "`--client \"acmee\"` doesn't fit: pace doesn't know the client `acmee`: did you mean `acme`?"
        );
    }

    #[test]
    fn role_flag_without_roles_names_role_add() {
        let flags = ProjectFlags {
            role: Some("work".to_owned()),
            ..ProjectFlags::default()
        };

        let error = ask_flow(
            flow_in(&Catalog::default(), flags),
            &Scripted::without_terminal(),
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .ends_with("pace doesn't know the role `work`: `pace role add work` adds it"),
            "{error}"
        );
    }

    #[test]
    fn role_flag_may_name_archived_role() {
        let flags = ProjectFlags {
            role: Some("old".to_owned()),
            ..ProjectFlags::default()
        };

        let result = ask_flow(flow_in(&catalog(), flags), &Scripted::without_terminal()).unwrap();

        assert_eq!(result.made.role, Some("old".parse().unwrap()));
    }

    #[test]
    fn flag_only_values_that_do_not_fit_name_their_flag() {
        let budget = ProjectFlags {
            budget: Some("lots".to_owned()),
            ..ProjectFlags::default()
        };
        let day = ProjectFlags {
            ends_on: Some("March".to_owned()),
            ..ProjectFlags::default()
        };
        let asker = Scripted::without_terminal();

        let budget = ask_flow(flow_in(&catalog(), budget), &asker).unwrap_err();
        let day = ask_flow(flow_in(&catalog(), day), &asker).unwrap_err();

        assert!(
            budget
                .to_string()
                .starts_with("`--budget \"lots\"` doesn't fit"),
            "{budget}"
        );
        assert_eq!(
            day.to_string(),
            "`--ends-on \"March\"` doesn't fit: `March` is not a day: write it like `2026-12-01`"
        );
    }

    #[test]
    fn key_with_record_names_project_edit() {
        let mut catalog = catalog();
        let result = ask_flow(
            flow_in(&catalog, ProjectFlags::default()),
            &Scripted::without_terminal(),
        )
        .unwrap();
        catalog = Catalog::new(
            catalog.roles().cloned().collect(),
            catalog.clients().cloned().collect(),
            vec![result.made],
        )
        .unwrap();

        let error = AddProjectFlow::new(
            &catalog,
            Path::new("projects/webshop.toml"),
            "webshop".parse().unwrap(),
            ID.parse().unwrap(),
            ProjectFlags::default(),
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "the project `webshop` has a record already"
        );
        assert_eq!(
            error.hint(),
            Some("change it with `pace project edit webshop`")
        );
    }
}
