//! The questions of `pace project init`: the key and the name from the
//! directory, then the client and the role, each none, an existing one or a
//! new one asked inline, and the billable flag.

use std::path::PathBuf;

use jiff::civil::Date;
use pace_core::{Catalog, Client, ClientKey, ProjectId, ProjectKey, ProjectRecord, Role, RoleId};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_store::{
    catalog::{CLIENTS_FILE, ROLES_FILE, project_record_path},
    field_edit::{FieldEdit, FieldValue},
};

use crate::project::{MANIFEST_FILES, Manifest};
use crate::questions::add_client::{AddClientFlow, ClientFlags};
use crate::questions::add_project::{AddProjectFlow, ProjectFlags};
use crate::questions::add_role::{AddRoleFlow, RoleFlags};
use crate::questions::{Answer, Ended, Flow, Question, QuestionKind, told};

/// The item of the client choice that adds a new client.
const NEW_CLIENT_ITEM: &str = "a new client";

/// The item of the role choice that adds a new role.
const NEW_ROLE_ITEM: &str = "a new role";

/// Where and when `pace project init` runs, and the IDs it may give.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitInputs {
    /// `PACE_HOME`, where the record, the clients and the roles are.
    pub home: PathBuf,
    /// The directory that becomes the project's: the working directory.
    pub dir: PathBuf,
    /// The manifest in `dir`, if it has one.
    pub manifest: Option<Manifest>,
    /// Today, the first day of a new client's rate.
    pub today: Date,
    /// The ID of a new record, unless the manifest gives one.
    pub project_id: ProjectId,
    /// The ID of a new role.
    pub role_id: RoleId,
}

/// What the flags of `pace project init` gave, as typed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InitFlags {
    /// `--key`
    pub key: Option<String>,
    /// `--name`, `--client`, `--role` and `--billable`/`--not-billable`
    pub project: ProjectFlags,
    /// `--visible`: a new manifest is `pace.toml`, not `.pace.toml`
    pub visible: bool,
}

impl InitFlags {
    /// The flags that answer a question of a new record, as typed: the
    /// ones a record that exists already leaves unused.
    #[must_use]
    pub fn record_flags(&self) -> Vec<&'static str> {
        let project = &self.project;
        [
            (project.name.is_some(), "--name"),
            (project.client.is_some(), "--client"),
            (project.role.is_some(), "--role"),
            (project.billable == Some(true), "--billable"),
            (project.billable == Some(false), "--not-billable"),
        ]
        .into_iter()
        .filter_map(|(given, flag)| given.then_some(flag))
        .collect()
    }
}

/// What `pace project init` made: the record, new or found, and what it
/// wrote besides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitProject {
    /// The project's record.
    pub record: ProjectRecord,
    /// Whether the record is new; else its key had one already.
    pub record_added: bool,
    /// The manifest of the directory.
    pub manifest: PathBuf,
    /// What changes in the manifest.
    pub manifest_change: ManifestChange,
    /// The client added on the way, if any.
    pub client: Option<Client>,
    /// The role added on the way, if any.
    pub role: Option<Role>,
}

/// What `pace project init` changes in the manifest of the directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestChange {
    /// A new manifest, with the record's ID and key.
    Added,
    /// The record's ID, which the manifest lacked; with `name_removed`,
    /// its name goes, as the record holds it.
    IdAdded {
        /// Whether the manifest's `name` is removed.
        name_removed: bool,
    },
    /// Only the manifest's name goes, as the record holds it.
    NameRemoved,
    /// Nothing: the manifest has the ID already, and no name.
    Unchanged,
}

impl ManifestChange {
    /// The change of `manifest`, or of a new one if there is none.
    const fn of(manifest: Option<&Manifest>) -> Self {
        match manifest {
            None => Self::Added,
            Some(manifest) if manifest.id.is_none() => Self::IdAdded {
                name_removed: manifest.name.is_some(),
            },
            Some(manifest) if manifest.name.is_some() => Self::NameRemoved,
            Some(_) => Self::Unchanged,
        }
    }
}

/// The record of the project: the key still to ask, the record its key
/// has, or the questions of a new one.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RecordStage {
    /// The key is still to ask.
    Unkeyed,
    /// The key has a record: only the manifest is written.
    Linked(ProjectRecord),
    /// A new record, asked by its own flow.
    Asking(Box<AddProjectFlow>),
}

/// A client or a role made inline: not wanted, its key to ask, its own
/// questions, or made.
#[derive(Debug, Clone, PartialEq, Eq)]
enum NewPart<F: Flow> {
    /// Not wanted.
    Unwanted,
    /// Its key is asked next.
    Keying,
    /// Its own flow asks the rest.
    Asking(F),
    /// Made, with the edits that write it.
    Made(Ended<F::Made>),
}

/// The flow of `pace project init`: it asks for what [`InitFlags`] and the
/// directory leave open and ends with the record, the manifest and a new
/// client or role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitProjectFlow {
    catalog: Catalog,
    inputs: InitInputs,
    flags: InitFlags,
    record: RecordStage,
    new_client: NewPart<AddClientFlow>,
    new_role: NewPart<AddRoleFlow>,
}

impl InitProjectFlow {
    /// The flow that makes `inputs.dir` a project of `catalog`; `flags`
    /// answer their questions. A manifest in the directory gives the key.
    ///
    /// # Errors
    ///
    /// If `--key` names another key than the manifest (exit code 2), or if
    /// the manifest has an ID and its key a record, so there is nothing to
    /// do (exit code 3).
    pub fn new(catalog: &Catalog, inputs: InitInputs, flags: InitFlags) -> PaceResult<Self> {
        let mut flow = Self {
            catalog: catalog.clone(),
            inputs,
            flags,
            record: RecordStage::Unkeyed,
            new_client: NewPart::Unwanted,
            new_role: NewPart::Unwanted,
        };
        let Some(manifest) = flow.inputs.manifest.clone() else {
            return Ok(flow);
        };
        if let Some(flag) = flow.flags.key.as_deref()
            && flag != manifest.key.as_str()
        {
            return Err(other_key_than_manifest(flag, &manifest));
        }
        let linked = catalog.project(&manifest.key).is_some();
        if linked && manifest.id.is_some() && manifest.name.is_none() {
            return Err(linked_already(&manifest));
        }
        flow.take_key(manifest.key).map_err(|message| {
            PaceError::new(
                ErrorKind::Internal,
                "the key of the manifest doesn't fit: {message}",
            )
            .attach_context("message", message)
            .ask_report()
        })?;
        Ok(flow)
    }

    /// The directory's name, if it has one.
    fn dir_name(&self) -> Option<String> {
        self.inputs
            .dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
    }

    /// The question of the key, with the directory's name as its default.
    fn key_question(&self) -> Question {
        let slug = self.dir_name().as_deref().and_then(ProjectKey::slug_of);
        Question {
            id: "key",
            text: "What is the project's key, such as acme-webshop?".to_owned(),
            kind: QuestionKind::Text {
                default: slug.as_ref().map(ToString::to_string),
            },
            flag: "--key",
            flag_answer: self.flags.key.clone().map(Answer::Text),
            default_note: slug.map_or_else(String::new, |slug| {
                format!("key `{slug}` from the directory name")
            }),
        }
    }

    /// Take `key` as the project's key: the record it has, or the flow of
    /// a new one.
    fn take_key(&mut self, key: ProjectKey) -> Result<(), String> {
        if let Some(record) = self.catalog.project(&key) {
            self.record = RecordStage::Linked(record.clone());
            return Ok(());
        }
        let manifest = self.inputs.manifest.as_ref();
        let id = manifest
            .and_then(|manifest| manifest.id)
            .unwrap_or(self.inputs.project_id);
        let (name, note) = match (
            manifest.and_then(|manifest| manifest.name.clone()),
            self.dir_name(),
        ) {
            (Some(name), _) => {
                let note = format!("name `{name}` from the manifest");
                (name, note)
            }
            (None, Some(name)) => {
                let note = format!("name `{name}` from the directory name");
                (name, note)
            }
            (None, None) => (key.to_string(), format!("name `{key}` from the key")),
        };
        let record_file = project_record_path(&self.inputs.home, &key);
        let project = AddProjectFlow::new(
            &self.catalog,
            &record_file,
            key,
            id,
            self.flags.project.clone(),
        )
        .map_err(|error| told(&error))?
        .with_name_default(name, note)
        .asking_without_offers();
        self.record = RecordStage::Asking(Box::new(project));
        Ok(())
    }

    /// Take `text` as the key of the new client, and start its questions:
    /// its name, its currency and its hourly rate, which holds from today.
    fn take_new_client_key(&mut self, text: &str) -> Result<(), String> {
        let key: ClientKey = text
            .trim()
            .parse()
            .map_err(|error: Box<PaceError>| told(&error))?;
        let flags = ClientFlags {
            from: Some(self.inputs.today.to_string()),
            rounding_step: Some(String::new()),
            ..ClientFlags::default()
        };
        let clients_file = self.inputs.home.join(CLIENTS_FILE);
        let flow = AddClientFlow::new(&self.catalog, &clients_file, key, self.inputs.today, flags)
            .map_err(|error| told(&error))?;
        self.new_client = NewPart::Asking(flow);
        Ok(())
    }

    /// Take `text` as the key of the new role, and start its questions, as
    /// `pace role add` asks them.
    fn take_new_role_key(&mut self, text: &str) -> Result<(), String> {
        let key = text
            .trim()
            .parse()
            .map_err(|error: Box<PaceError>| told(&error))?;
        let roles_file = self.inputs.home.join(ROLES_FILE);
        let flow = AddRoleFlow::new(
            &self.catalog,
            &roles_file,
            key,
            self.inputs.role_id,
            RoleFlags::default(),
        )
        .map_err(|error| told(&error))?;
        self.new_role = NewPart::Asking(flow);
        Ok(())
    }

    /// The new record's flow and `answer`: a new client or role picked,
    /// an answer to the new one's questions, or to the record's own.
    fn take_record_answer(&mut self, answer: Answer) -> Result<(), String> {
        let RecordStage::Asking(project) = &mut self.record else {
            return Err("every question has its answer".to_owned());
        };
        match (&mut self.new_client, &mut self.new_role, answer) {
            (NewPart::Keying, _, Answer::Text(text)) => self.take_new_client_key(&text),
            (NewPart::Asking(client), _, answer) => {
                client.take_answer(answer)?;
                if client.next_question().is_none() {
                    let NewPart::Asking(client) =
                        std::mem::replace(&mut self.new_client, NewPart::Unwanted)
                    else {
                        return Err("the new client has no questions".to_owned());
                    };
                    let ended = client.finish().map_err(|error| told(&error))?;
                    project.offer_client(ended.made.key.clone(), ended.made.name.clone());
                    project.take_answer(Answer::Text(ended.made.key.to_string()))?;
                    self.new_client = NewPart::Made(ended);
                }
                Ok(())
            }
            (_, NewPart::Keying, Answer::Text(text)) => self.take_new_role_key(&text),
            (_, NewPart::Asking(role), answer) => {
                role.take_answer(answer)?;
                if role.next_question().is_none() {
                    let NewPart::Asking(role) =
                        std::mem::replace(&mut self.new_role, NewPart::Unwanted)
                    else {
                        return Err("the new role has no questions".to_owned());
                    };
                    let ended = role.finish().map_err(|error| told(&error))?;
                    project.offer_role(ended.made.key.clone(), ended.made.name.clone());
                    project.take_answer(Answer::Text(ended.made.key.to_string()))?;
                    self.new_role = NewPart::Made(ended);
                }
                Ok(())
            }
            (_, _, answer) => {
                let new_item = project
                    .next_question()
                    .and_then(|question| match question.kind {
                        QuestionKind::Choice { items, .. } => Some((question.id, items.len())),
                        _ => None,
                    });
                match (new_item, &answer) {
                    (Some(("client", new)), Answer::Choice(index)) if *index == new => {
                        self.new_client = NewPart::Keying;
                    }
                    (Some(("role", new)), Answer::Choice(index)) if *index == new => {
                        self.new_role = NewPart::Keying;
                    }
                    _ => project.take_answer(answer)?,
                }
                Ok(())
            }
        }
    }

    /// The edits of the manifest: a new one with the record's ID and key,
    /// or the `id` an old one lacks and the `name` it still holds.
    fn manifest_edits(&self, record: &ProjectRecord) -> (PathBuf, Vec<FieldEdit>) {
        let id = || FieldValue::String(record.id.to_string());
        if let Some(manifest) = &self.inputs.manifest {
            let mut edits = Vec::new();
            if manifest.id.is_none() {
                edits.push(FieldEdit::set(&manifest.path, &["project", "id"], id()));
            }
            if manifest.name.is_some() {
                edits.push(FieldEdit::remove(&manifest.path, &["project", "name"]));
            }
            return (manifest.path.clone(), edits);
        }
        let [visible, hidden] = MANIFEST_FILES;
        let path = self
            .inputs
            .dir
            .join(if self.flags.visible { visible } else { hidden });
        let edits = vec![
            FieldEdit::set(&path, &["project", "id"], id()),
            FieldEdit::set(
                &path,
                &["project", "key"],
                FieldValue::String(record.key.to_string()),
            ),
        ];
        (path, edits)
    }
}

/// The error for `--key` with another key than the `manifest` of the
/// directory gives.
fn other_key_than_manifest(flag: &str, manifest: &Manifest) -> Box<PaceError> {
    PaceError::new(
        ErrorKind::InvalidInput,
        "`--key {flag}` is not the key `{key}` of the manifest {path}",
    )
    .attach_context("flag", flag)
    .attach_context("key", manifest.key.to_string())
    .attach_context("path", manifest.path.display().to_string())
    .attach_hint("leave `--key` out, or change the key in the manifest")
    .attach_exit_code(ExitCode::Usage)
}

/// The error for a `manifest` whose ID and record exist already: nothing
/// to do.
fn linked_already(manifest: &Manifest) -> Box<PaceError> {
    PaceError::new(
        ErrorKind::InvalidInput,
        "the manifest {path} links its directory to the project `{key}` already",
    )
    .attach_context("path", manifest.path.display().to_string())
    .attach_context("key", manifest.key.to_string())
    .attach_hint(format!(
        "`pace project edit {}` changes the project's record",
        manifest.key
    ))
    .attach_exit_code(ExitCode::NothingToDo)
}

/// What `part` made, if it was made; its edits go to `edits`.
fn made_part<F: Flow>(part: NewPart<F>, edits: &mut Vec<FieldEdit>) -> Option<F::Made> {
    match part {
        NewPart::Made(ended) => {
            edits.extend(ended.edits);
            Some(ended.made)
        }
        NewPart::Unwanted | NewPart::Keying | NewPart::Asking(_) => None,
    }
}

/// The question of the key of a new `noun`, such as `client`, with
/// `example` as an example; it has no default.
fn new_key_question(noun: &'static str, example: &str) -> Question {
    Question {
        id: "new-key",
        text: format!("What is the new {noun}'s key, such as {example}?"),
        kind: QuestionKind::Text { default: None },
        flag: if noun == "client" {
            "--client"
        } else {
            "--role"
        },
        flag_answer: None,
        default_note: String::new(),
    }
}

/// `question`, a choice of the record's flow, with `item` added last.
fn with_item(mut question: Question, item: &str) -> Question {
    if let QuestionKind::Choice { items, .. } = &mut question.kind {
        items.push(item.to_owned());
    }
    question
}

impl Flow for InitProjectFlow {
    type Made = InitProject;

    fn next_question(&self) -> Option<Question> {
        let project = match &self.record {
            RecordStage::Unkeyed => return Some(self.key_question()),
            RecordStage::Linked(_) => return None,
            RecordStage::Asking(project) => project,
        };
        match (&self.new_client, &self.new_role) {
            (NewPart::Keying, _) => return Some(new_key_question("client", "acme")),
            (NewPart::Asking(client), _) => return client.next_question(),
            (_, NewPart::Keying) => return Some(new_key_question("role", "work")),
            (_, NewPart::Asking(role)) => return role.next_question(),
            _ => {}
        }
        let question = project.next_question()?;
        Some(match question.id {
            "client" => with_item(question, NEW_CLIENT_ITEM),
            "role" => with_item(question, NEW_ROLE_ITEM),
            _ => question,
        })
    }

    fn take_answer(&mut self, answer: Answer) -> Result<(), String> {
        if self.record != RecordStage::Unkeyed {
            return self.take_record_answer(answer);
        }
        let Answer::Text(text) = answer else {
            return Err("type the project's key".to_owned());
        };
        let key = text
            .trim()
            .parse()
            .map_err(|error: Box<PaceError>| told(&error))?;
        self.take_key(key)
    }

    fn finish(self) -> PaceResult<Ended<InitProject>> {
        let mut edits = Vec::new();
        let client = made_part(self.new_client.clone(), &mut edits);
        let role = made_part(self.new_role.clone(), &mut edits);
        let (record, record_added) = match self.record.clone() {
            RecordStage::Asking(project) => {
                let ended = project.finish()?;
                edits.extend(ended.edits);
                (ended.made, true)
            }
            RecordStage::Linked(record) => (record, false),
            RecordStage::Unkeyed => {
                return Err(PaceError::new(
                    ErrorKind::Internal,
                    "the flow of `pace project init` ended without a key",
                )
                .ask_report());
            }
        };
        let (manifest, manifest_edits) = self.manifest_edits(&record);
        let manifest_change = ManifestChange::of(self.inputs.manifest.as_ref());
        edits.extend(manifest_edits);
        Ok(Ended {
            edits,
            made: InitProject {
                record,
                record_added,
                manifest,
                manifest_change,
                client,
                role,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;

    use jiff::civil::date;
    use pace_core::{Rates, RoleKind, Rounding};

    use super::*;
    use crate::prompt::Scripted;
    use crate::questions::ask_flow;

    const PROJECT_ID: &str = "0193a7c4-0000-7000-8000-000000000001";
    const ROLE_ID: &str = "0193a7c4-0000-7000-8000-0000000000a1";
    const MANIFEST_ID: &str = "0193a7c4-0000-7000-8000-0000000000ff";

    fn inputs(dir: &str, manifest: Option<Manifest>) -> InitInputs {
        InitInputs {
            home: PathBuf::from("/home"),
            dir: PathBuf::from(dir),
            manifest,
            today: date(2026, 11, 26),
            project_id: PROJECT_ID.parse().unwrap(),
            role_id: ROLE_ID.parse().unwrap(),
        }
    }

    fn manifest(id: Option<&str>, name: Option<&str>) -> Manifest {
        Manifest {
            path: PathBuf::from("/work/webshop/.pace.toml"),
            id: id.map(|id| id.parse().unwrap()),
            key: "webshop".parse().unwrap(),
            name: name.map(str::to_owned),
            default_tags: std::collections::BTreeSet::new(),
        }
    }

    fn record(key: &str, name: &str) -> ProjectRecord {
        ProjectRecord {
            id: MANIFEST_ID.parse().unwrap(),
            key: key.parse().unwrap(),
            name: name.to_owned(),
            client: None,
            role: None,
            billable: false,
            budget: None,
            starts_on: None,
            ends_on: None,
            done_when: None,
            allow_overlap: false,
            archived: None,
        }
    }

    /// A catalog with the client `acme`, the role `work` and the record of
    /// `records`.
    fn catalog(records: Vec<ProjectRecord>) -> Catalog {
        let acme = Client {
            key: "acme".parse().unwrap(),
            name: "ACME GmbH".to_owned(),
            currency: "EUR".parse().unwrap(),
            rounding: Rounding::NONE,
            rates: Rates::default(),
            project_rates: BTreeMap::new(),
        };
        let work = Role {
            id: "0193a7c4-0000-7000-8000-0000000000a2".parse().unwrap(),
            key: "work".parse().unwrap(),
            kind: RoleKind::Area,
            name: "Work".to_owned(),
            purpose: None,
            weekly_target: None,
            archived: None,
        };
        Catalog::new(vec![work], vec![acme], records).unwrap()
    }

    fn flow(catalog: &Catalog, inputs: InitInputs, flags: InitFlags) -> InitProjectFlow {
        InitProjectFlow::new(catalog, inputs, flags).unwrap()
    }

    fn files(edits: &[FieldEdit]) -> Vec<String> {
        let mut files: Vec<String> = edits
            .iter()
            .map(|edit| edit.file.display().to_string())
            .collect();
        files.dedup();
        files
    }

    #[test]
    fn without_terminal_takes_key_and_name_from_directory_and_names_defaults() {
        let flow = flow(
            &Catalog::default(),
            inputs("/work/Web Shop", None),
            InitFlags::default(),
        );

        let result = ask_flow(flow, &Scripted::without_terminal()).unwrap();

        assert_eq!(
            result.defaults,
            [
                "key `web-shop` from the directory name",
                "name `Web Shop` from the directory name",
                "no client",
                "no role",
                "not billable, as it has no client"
            ]
        );
        assert_eq!(result.made.record.name, "Web Shop");
        assert!(result.made.record_added);
        assert_eq!(result.made.manifest_change, ManifestChange::Added);
        assert_eq!(
            files(&result.edits),
            ["/home/projects/web-shop.toml", "/work/Web Shop/.pace.toml"]
        );
        assert_eq!(
            result.edits[result.edits.len() - 2..],
            [
                FieldEdit::set(
                    "/work/Web Shop/.pace.toml",
                    &["project", "id"],
                    FieldValue::String(PROJECT_ID.to_owned())
                ),
                FieldEdit::set(
                    "/work/Web Shop/.pace.toml",
                    &["project", "key"],
                    FieldValue::String("web-shop".to_owned())
                ),
            ]
        );
    }

    #[test]
    fn new_client_asked_inline_is_written_with_rate_from_today() {
        let asker = Scripted::typing(["", "", "acme2", "Acme Two", "eur", "90"])
            .and_choosing([2, 0])
            .and_confirming([true]);

        let result = ask_flow(
            flow(
                &catalog(Vec::new()),
                inputs("/work/webshop", None),
                InitFlags::default(),
            ),
            &asker,
        )
        .unwrap();

        assert_eq!(
            asker.offered()[0],
            ["none", "acme: ACME GmbH", "a new client"]
        );
        assert_eq!(
            asker.questions()[3..6],
            [
                "What is the new client's key, such as acme?",
                "What is the client's name?",
                "Which currency does the client pay in, such as EUR?",
            ]
        );
        let client = result.made.client.unwrap();
        assert_eq!(client.name, "Acme Two");
        assert_eq!(
            client.rates.iter().next().map(|rate| rate.from),
            Some(date(2026, 11, 26))
        );
        assert_eq!(result.made.record.client, Some(client.key));
        assert!(result.made.record.billable);
        assert_eq!(
            files(&result.edits),
            [
                "/home/clients.toml",
                "/home/projects/webshop.toml",
                "/work/webshop/.pace.toml"
            ]
        );
    }

    #[test]
    fn new_role_asked_inline_as_role_add_asks_it() {
        let asker = Scripted::typing(["", "", "garden", "Garden", "", ""])
            .and_choosing([0, 2, 0])
            .and_confirming([false]);

        let result = ask_flow(
            flow(
                &catalog(Vec::new()),
                inputs("/work/webshop", None),
                InitFlags::default(),
            ),
            &asker,
        )
        .unwrap();

        assert_eq!(asker.offered()[1], ["none", "work: Work", "a new role"]);
        let role = result.made.role.unwrap();
        assert_eq!(role.id, ROLE_ID.parse().unwrap());
        assert_eq!(role.name, "Garden");
        assert_eq!(result.made.record.role, Some(role.key));
        assert_eq!(files(&result.edits)[0], "/home/roles.toml");
    }

    #[test]
    fn new_client_key_that_exists_is_told_and_asked_again() {
        let asker = Scripted::typing(["", "", "acme", "acme2", "", "EUR", ""])
            .and_choosing([2, 0])
            .and_confirming([true]);

        let result = ask_flow(
            flow(
                &catalog(Vec::new()),
                inputs("/work/webshop", None),
                InitFlags::default(),
            ),
            &asker,
        )
        .unwrap();

        assert_eq!(
            asker.told(),
            ["the client `acme` exists already: change it with `pace client edit acme`"]
        );
        assert_eq!(
            result.made.client.map(|client| client.name),
            Some("acme2".to_owned())
        );
    }

    #[test]
    fn flags_skip_their_questions() {
        let flags = InitFlags {
            key: Some("acme-webshop".to_owned()),
            project: ProjectFlags {
                name: Some("Webshop relaunch".to_owned()),
                client: Some("acme".to_owned()),
                role: Some("work".to_owned()),
                billable: Some(false),
                ..ProjectFlags::default()
            },
            visible: true,
        };
        let asker = Scripted::typing([]);

        let result = ask_flow(
            flow(&catalog(Vec::new()), inputs("/work/webshop", None), flags),
            &asker,
        )
        .unwrap();

        assert_eq!(asker.questions(), Vec::<String>::new());
        assert_eq!(result.made.record.key, "acme-webshop".parse().unwrap());
        assert!(!result.made.record.billable);
        assert_eq!(result.made.manifest, Path::new("/work/webshop/pace.toml"));
    }

    #[test]
    fn key_with_record_writes_only_manifest_with_record_id() {
        let catalog = catalog(vec![record("webshop", "Webshop")]);
        let asker = Scripted::typing([""]);

        let result = ask_flow(
            flow(
                &catalog,
                inputs("/work/webshop", None),
                InitFlags::default(),
            ),
            &asker,
        )
        .unwrap();

        assert_eq!(
            asker.questions(),
            ["What is the project's key, such as acme-webshop?"]
        );
        assert!(!result.made.record_added);
        assert_eq!(files(&result.edits), ["/work/webshop/.pace.toml"]);
        assert_eq!(
            result.edits[0],
            FieldEdit::set(
                "/work/webshop/.pace.toml",
                &["project", "id"],
                FieldValue::String(MANIFEST_ID.to_owned())
            )
        );
    }

    #[test]
    fn manifest_of_phase_two_gets_record_with_its_name_and_loses_name() {
        let flow = flow(
            &Catalog::default(),
            inputs("/work/webshop", Some(manifest(None, Some("Webshop")))),
            InitFlags::default(),
        );

        let result = ask_flow(flow, &Scripted::without_terminal()).unwrap();

        assert_eq!(result.defaults[0], "name `Webshop` from the manifest");
        assert_eq!(result.made.record.name, "Webshop");
        assert_eq!(
            result.made.manifest_change,
            ManifestChange::IdAdded { name_removed: true }
        );
        assert_eq!(
            result.edits[result.edits.len() - 2..],
            [
                FieldEdit::set(
                    "/work/webshop/.pace.toml",
                    &["project", "id"],
                    FieldValue::String(PROJECT_ID.to_owned())
                ),
                FieldEdit::remove("/work/webshop/.pace.toml", &["project", "name"]),
            ]
        );
    }

    #[test]
    fn manifest_id_without_record_becomes_record_id() {
        let flow = flow(
            &Catalog::default(),
            inputs("/work/webshop", Some(manifest(Some(MANIFEST_ID), None))),
            InitFlags::default(),
        );

        let result = ask_flow(flow, &Scripted::without_terminal()).unwrap();

        assert_eq!(result.made.record.id, MANIFEST_ID.parse().unwrap());
        assert_eq!(files(&result.edits), ["/home/projects/webshop.toml"]);
    }

    #[test]
    fn manifest_with_id_and_record_has_nothing_to_do() {
        let error = InitProjectFlow::new(
            &catalog(vec![record("webshop", "Webshop")]),
            inputs("/work/webshop", Some(manifest(Some(MANIFEST_ID), None))),
            InitFlags::default(),
        )
        .unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::NothingToDo);
        assert_eq!(
            error.to_string(),
            "the manifest /work/webshop/.pace.toml links its directory to the project `webshop` already"
        );
    }

    #[test]
    fn key_flag_other_than_manifest_key_is_usage_error() {
        let flags = InitFlags {
            key: Some("shop".to_owned()),
            ..InitFlags::default()
        };

        let error = InitProjectFlow::new(
            &Catalog::default(),
            inputs("/work/webshop", Some(manifest(None, None))),
            flags,
        )
        .unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
    }

    #[test]
    fn directory_name_without_slug_without_terminal_names_key_flag() {
        let flow = flow(
            &Catalog::default(),
            inputs("/work/日本", None),
            InitFlags::default(),
        );

        let error = ask_flow(flow, &Scripted::without_terminal()).unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(error.hint(), Some("answer it with `--key`"));
    }

    #[test]
    fn record_flags_name_what_record_that_exists_leaves_unused() {
        let flags = InitFlags {
            project: ProjectFlags {
                name: Some("Shop".to_owned()),
                billable: Some(false),
                ..ProjectFlags::default()
            },
            ..InitFlags::default()
        };

        assert_eq!(flags.record_flags(), ["--name", "--not-billable"]);
    }
}
