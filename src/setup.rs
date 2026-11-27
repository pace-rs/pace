//! The guided tour of `pace setup`: a step machine without I/O that asks
//! for the settings, the roles, the clients and this directory's project,
//! and ends with the field edits of what changed.
//!
//! [`SetupFlow`] is a [`Flow`] of the question machine, in five steps: the
//! time zone and the week start; the roles, with new ones asked as `pace
//! role add` asks them; the clients, new ones as `pace client add` asks
//! them, or a new dated rate; this directory, made a project as `pace
//! project init` makes it, or its record changed; and the summary, one line
//! per file and field, which the user accepts or not. Each question has
//! the current value as its default, so Enter keeps it, and only a field
//! whose answer differs becomes an edit. The flow writes nothing: the
//! command writes the edits once the user accepts the summary.
//!
//! The flow keeps each answer with its question and the value it replaced
//! as a [`SavedAnswer`]. [`SetupFlow::saved`] gives them with the step as a
//! [`SavedSetup`], which serializes; [`SetupFlow::replayed`] answers a new
//! flow with them again, and stops at the first question whose value
//! changed since.

pub mod rate;
pub mod record;
mod saved;
pub mod setup_file;
mod step_questions;
pub mod summary;

use std::path::PathBuf;

use jiff::civil::Date;
use pace_core::{
    Catalog, Client, ClientKey, Ids, ProjectId, ProjectKey, ProjectRecord, Role, RoleId, RoleKey,
};
use pace_error::{PaceError, PaceResult};
use pace_store::catalog::{CLIENTS_FILE, ROLES_FILE, project_record_path};
use pace_store::field_edit::{FieldChange, FieldEdit, FieldValue};

use crate::config::{Key, setting_value};
use crate::project::Manifest;
use crate::questions::add_client::{AddClientFlow, ClientFlags};
use crate::questions::add_role::{AddRoleFlow, RoleFlags};
use crate::questions::init_project::{InitFlags, InitInputs, InitProjectFlow};
use crate::questions::{Answer, Ended, Flow, Question, told};
use crate::setup::rate::NewRateFlow;
use crate::setup::record::ChangeRecordFlow;
pub use crate::setup::saved::{SAVED_SETUP_VERSION, SavedAnswer, SavedSetup, SetupStep};
use crate::setup::step_questions::key_question;
use crate::setup::summary::{SetupChange, change_line, file_label};

/// The days the week may start on, as the setting `week-start` writes
/// them, in the order the question offers them.
pub(crate) const WEEK_DAYS: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];

/// What setup finds before it asks: the current settings, the catalog,
/// the working directory and the day.
#[derive(Debug)]
pub struct SetupInputs {
    /// The global config file, which the settings go to.
    pub config_file: PathBuf,
    /// `time-zone` as the global file sets it, else its default.
    pub time_zone: String,
    /// `week-start` as the global file sets it, else its default.
    pub week_start: String,
    /// `PACE_HOME`, where the roles, the clients and the records are.
    pub home: PathBuf,
    /// The roles, clients and project records.
    pub catalog: Catalog,
    /// The working directory, which the fourth step offers to make a
    /// project.
    pub dir: PathBuf,
    /// The manifest in `dir`, if it has one.
    pub manifest: Option<Manifest>,
    /// Today, the first day of a new rate.
    pub today: Date,
    /// Where the IDs of new roles and records come from.
    pub ids: Box<dyn Ids>,
}

/// What setup ended with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupOutcome {
    /// Every answer kept its value.
    NothingToChange,
    /// The user declined the summary's `changes`.
    Declined {
        /// The summary's lines.
        changes: Vec<String>,
    },
    /// The user accepted the summary's `changes`, which go to `files`.
    Accepted {
        /// The summary's lines.
        changes: Vec<String>,
        /// The files the changes go to, as the summary names them.
        files: Vec<String>,
    },
}

/// What this directory may become in the fourth step.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DirectoryOffer {
    /// It has no manifest: make it a project.
    MakeProject,
    /// Its manifest names `key`, but lacks the record or the record's ID.
    Link(ProjectKey),
    /// Its manifest links it to this record: change the record.
    ChangeRecord(ProjectRecord),
}

/// Where the roles step stands.
#[derive(Debug)]
enum RoleStage {
    /// Whether to add a role.
    Offer,
    /// The new role's key.
    Keying,
    /// The new role's own questions.
    Asking(Box<AddRoleFlow>),
}

/// Where the clients step stands.
#[derive(Debug)]
enum ClientStage {
    /// What to do next: go on, add a client or give one a new rate.
    Offer,
    /// The new client's key.
    Keying,
    /// The new client's own questions.
    Asking(Box<AddClientFlow>),
    /// A client's new rate.
    Rate(Box<NewRateFlow>),
}

/// Where the directory step stands.
#[derive(Debug)]
enum DirectoryStage {
    /// Whether to take the offer.
    Offer,
    /// The questions of `pace project init`.
    Init(Box<InitProjectFlow>),
    /// The questions of the record.
    Record(Box<ChangeRecordFlow>),
}

/// The live state of setup, with the sub-flow a step asks through.
#[derive(Debug)]
enum Stage {
    Settings,
    Roles(RoleStage),
    Clients(ClientStage),
    Directory(DirectoryStage),
    Summary,
    Ended,
}

/// A flow answered with a saved setup's answers, and what changed since,
/// if that stopped the replay.
#[derive(Debug)]
pub struct Replayed {
    /// The flow, at the first question without an answer.
    pub flow: SetupFlow,
    /// What changed in the question the replay stopped at, such as
    /// "Since setup stopped, the value changed: `system` → `UTC`."
    pub changed: Option<String>,
}

/// The flow of `pace setup`: see the module's documentation.
#[derive(Debug)]
pub struct SetupFlow {
    inputs: SetupInputs,
    offer: DirectoryOffer,
    /// The catalog with the roles and clients added so far.
    catalog: Catalog,
    stage: Stage,
    time_zone: Option<String>,
    changes: Vec<SetupChange>,
    accepted: Option<bool>,
    answers: Vec<SavedAnswer>,
}

impl SetupFlow {
    /// The flow over `inputs`, at its first question.
    #[must_use]
    pub fn new(inputs: SetupInputs) -> Self {
        let offer = match &inputs.manifest {
            None => DirectoryOffer::MakeProject,
            Some(manifest) => match inputs.catalog.project(&manifest.key) {
                Some(record) if manifest.id.is_some() && manifest.name.is_none() => {
                    DirectoryOffer::ChangeRecord(record.clone())
                }
                _ => DirectoryOffer::Link(manifest.key.clone()),
            },
        };
        Self {
            catalog: inputs.catalog.clone(),
            inputs,
            offer,
            stage: Stage::Settings,
            time_zone: None,
            changes: Vec::new(),
            accepted: None,
            answers: Vec::new(),
        }
    }

    /// The flow over `inputs`, answered with the answers of `saved`, as
    /// far as each answers the question the flow gives, the value it
    /// replaced is still the question's default and the flow still takes
    /// it. Where a value changed since, such as by a hand edit, or an
    /// answer no longer fits, such as a role key another device added,
    /// [`Replayed::changed`] says how.
    #[must_use]
    pub fn replayed(inputs: SetupInputs, saved: &SavedSetup) -> Replayed {
        let mut flow = Self::new(inputs);
        for saved in &saved.answers {
            let Some(question) = flow.next_question() else {
                break;
            };
            if question.id != saved.question {
                break;
            }
            if let Some(changed) = saved.changed_since(&question) {
                return Replayed {
                    flow,
                    changed: Some(changed),
                };
            }
            if let Err(refusal) = flow.take_answer(saved.answer.clone()) {
                return Replayed {
                    changed: Some(saved.refused_since(&refusal)),
                    flow,
                };
            }
        }
        Replayed {
            flow,
            changed: None,
        }
    }

    /// Where the flow stands, to save.
    #[must_use]
    pub fn saved(&self) -> SavedSetup {
        SavedSetup {
            version: SAVED_SETUP_VERSION,
            step: self.step(),
            answers: self.answers.clone(),
        }
    }

    /// The step the flow asks in.
    #[must_use]
    pub const fn step(&self) -> SetupStep {
        match self.stage {
            Stage::Settings => SetupStep::Settings,
            Stage::Roles(_) => SetupStep::Roles,
            Stage::Clients(_) => SetupStep::Clients,
            Stage::Directory(_) => SetupStep::Directory,
            Stage::Summary => SetupStep::Summary,
            Stage::Ended => SetupStep::Ended,
        }
    }

    /// The summary that the question whether to write it follows: each
    /// change by file and field; `None` before the summary.
    #[must_use]
    pub fn summary(&self) -> Option<Vec<String>> {
        matches!(self.stage, Stage::Summary).then(|| self.summary_lines())
    }

    /// The summary's lines: each change by file and field.
    fn summary_lines(&self) -> Vec<String> {
        self.changes
            .iter()
            .map(|change| change_line(change, &self.inputs.home))
            .collect()
    }

    /// Take `answer` to the question `id` of the current stage.
    fn take(&mut self, id: &str, answer: Answer) -> Result<(), String> {
        match (&mut self.stage, answer) {
            (Stage::Settings, Answer::Text(text)) if id == "time-zone" => {
                self.take_time_zone(&text)
            }
            (Stage::Settings, Answer::Choice(index)) if id == "week-start" => {
                self.take_week_start(index)
            }
            (Stage::Roles(RoleStage::Offer), Answer::YesNo(add)) => {
                self.stage = if add {
                    Stage::Roles(RoleStage::Keying)
                } else {
                    Stage::Clients(ClientStage::Offer)
                };
                Ok(())
            }
            (Stage::Roles(RoleStage::Keying), Answer::Text(text)) => self.take_role_key(&text),
            (Stage::Roles(RoleStage::Asking(flow)), answer) => flow.take_answer(answer),
            (Stage::Clients(ClientStage::Offer), Answer::Choice(index)) => {
                self.take_client_pick(index)
            }
            (Stage::Clients(ClientStage::Keying), Answer::Text(text)) => {
                self.take_client_key(&text)
            }
            (Stage::Clients(ClientStage::Asking(flow)), answer) => flow.take_answer(answer),
            (Stage::Clients(ClientStage::Rate(flow)), answer) => flow.take_answer(answer),
            (Stage::Directory(DirectoryStage::Offer), Answer::YesNo(take)) => {
                self.take_directory_offer(take)
            }
            (Stage::Directory(DirectoryStage::Init(flow)), answer) => flow.take_answer(answer),
            (Stage::Directory(DirectoryStage::Record(flow)), answer) => flow.take_answer(answer),
            (Stage::Summary, Answer::YesNo(accept)) => {
                self.accepted = Some(accept);
                self.stage = Stage::Ended;
                Ok(())
            }
            (_, answer) => Err(format!("{answer:?} is no answer to `{id}`")),
        }
    }

    /// Take `text` as the time zone; a zone other than the current one is
    /// a change of the global file.
    fn take_time_zone(&mut self, text: &str) -> Result<(), String> {
        let value = setting_value(Key::TimeZone, text.trim()).map_err(|error| told(&error))?;
        let zone = value.as_str().unwrap_or_default().to_owned();
        if zone != self.inputs.time_zone {
            self.changes.push(setting_change(
                &self.inputs,
                Key::TimeZone,
                &zone,
                &self.inputs.time_zone,
            ));
        }
        self.time_zone = Some(zone);
        Ok(())
    }

    /// Take the day at `index` of [`WEEK_DAYS`] as the week's start, and
    /// go on to the roles.
    fn take_week_start(&mut self, index: usize) -> Result<(), String> {
        let day = *WEEK_DAYS.get(index).ok_or("pick one of the days")?;
        if day != self.inputs.week_start {
            self.changes.push(setting_change(
                &self.inputs,
                Key::WeekStart,
                day,
                &self.inputs.week_start,
            ));
        }
        self.stage = Stage::Roles(RoleStage::Offer);
        Ok(())
    }

    /// Take `text` as the new role's key, and start its questions.
    fn take_role_key(&mut self, text: &str) -> Result<(), String> {
        let key: RoleKey = text
            .trim()
            .parse()
            .map_err(|error: Box<PaceError>| told(&error))?;
        let id = RoleId::try_from(self.inputs.ids.next_id()).map_err(|error| told(&error))?;
        let roles_file = self.inputs.home.join(ROLES_FILE);
        let flow = AddRoleFlow::new(&self.catalog, &roles_file, key, id, RoleFlags::default())
            .map_err(|error| told(&error))?;
        self.stage = Stage::Roles(RoleStage::Asking(Box::new(flow)));
        Ok(())
    }

    /// Take the item at `index` of the clients question.
    fn take_client_pick(&mut self, index: usize) -> Result<(), String> {
        self.stage = match index {
            0 => Stage::Directory(DirectoryStage::Offer),
            1 => Stage::Clients(ClientStage::Keying),
            _ => {
                let client = self
                    .catalog
                    .clients()
                    .nth(index - 2)
                    .cloned()
                    .ok_or("pick one of the items")?;
                let clients_file = self.inputs.home.join(CLIENTS_FILE);
                Stage::Clients(ClientStage::Rate(Box::new(NewRateFlow::new(
                    &clients_file,
                    client,
                    self.inputs.today,
                ))))
            }
        };
        Ok(())
    }

    /// Take `text` as the new client's key, and start its questions.
    fn take_client_key(&mut self, text: &str) -> Result<(), String> {
        let key: ClientKey = text
            .trim()
            .parse()
            .map_err(|error: Box<PaceError>| told(&error))?;
        let clients_file = self.inputs.home.join(CLIENTS_FILE);
        let flow = AddClientFlow::new(
            &self.catalog,
            &clients_file,
            key,
            self.inputs.today,
            ClientFlags::default(),
        )
        .map_err(|error| told(&error))?;
        self.stage = Stage::Clients(ClientStage::Asking(Box::new(flow)));
        Ok(())
    }

    /// Take the offer for this directory, or go on to the summary.
    fn take_directory_offer(&mut self, take: bool) -> Result<(), String> {
        if !take {
            self.stage = Stage::Summary;
            return Ok(());
        }
        self.stage = Stage::Directory(match &self.offer {
            DirectoryOffer::MakeProject | DirectoryOffer::Link(_) => {
                let inputs = self.init_inputs().map_err(|error| told(&error))?;
                let flow = InitProjectFlow::new(&self.catalog, inputs, InitFlags::default())
                    .map_err(|error| told(&error))?;
                DirectoryStage::Init(Box::new(flow))
            }
            DirectoryOffer::ChangeRecord(record) => {
                let record_file = project_record_path(&self.inputs.home, &record.key);
                DirectoryStage::Record(Box::new(ChangeRecordFlow::new(
                    &self.catalog,
                    &record_file,
                    record.clone(),
                )))
            }
        });
        Ok(())
    }

    /// The inputs of `pace project init` in this directory.
    fn init_inputs(&self) -> PaceResult<InitInputs> {
        Ok(InitInputs {
            home: self.inputs.home.clone(),
            dir: self.inputs.dir.clone(),
            manifest: self.inputs.manifest.clone(),
            today: self.inputs.today,
            project_id: ProjectId::try_from(self.inputs.ids.next_id())?,
            role_id: RoleId::try_from(self.inputs.ids.next_id())?,
        })
    }

    /// Finish a sub-flow that has every answer: its changes join the
    /// summary, and what it made joins the catalog. A summary without
    /// changes ends the flow.
    fn settle(&mut self) -> PaceResult<()> {
        let stage = std::mem::replace(&mut self.stage, Stage::Ended);
        self.stage = match stage {
            Stage::Roles(RoleStage::Asking(flow)) if flow.next_question().is_none() => {
                let Ended { edits, made } = flow.finish()?;
                self.catalog = with_role(&self.catalog, made)?;
                self.changes
                    .extend(edits.into_iter().map(SetupChange::added));
                Stage::Roles(RoleStage::Offer)
            }
            Stage::Clients(ClientStage::Asking(flow)) if flow.next_question().is_none() => {
                let Ended { edits, made } = flow.finish()?;
                self.catalog = with_client(&self.catalog, made)?;
                self.changes
                    .extend(edits.into_iter().map(SetupChange::added));
                Stage::Clients(ClientStage::Offer)
            }
            Stage::Clients(ClientStage::Rate(flow)) if flow.next_question().is_none() => {
                if let Some((change, client)) = flow.finish() {
                    self.catalog = with_client(&self.catalog, client)?;
                    self.changes.push(change);
                }
                Stage::Clients(ClientStage::Offer)
            }
            Stage::Directory(DirectoryStage::Init(flow)) if flow.next_question().is_none() => {
                let Ended { edits, .. } = flow.finish()?;
                let before = self.inputs.manifest.as_ref().and_then(|m| m.name.clone());
                self.changes.extend(
                    edits
                        .into_iter()
                        .map(|edit| manifest_change(edit, before.as_ref())),
                );
                Stage::Summary
            }
            Stage::Directory(DirectoryStage::Record(flow)) if flow.next_question().is_none() => {
                self.changes.extend(flow.finish());
                Stage::Summary
            }
            other => other,
        };
        if matches!(self.stage, Stage::Summary) && self.changes.is_empty() {
            self.stage = Stage::Ended;
        }
        Ok(())
    }
}

impl Flow for SetupFlow {
    type Made = SetupOutcome;

    fn next_question(&self) -> Option<Question> {
        match &self.stage {
            Stage::Settings => Some(self.settings_question()),
            Stage::Roles(RoleStage::Offer) => Some(self.roles_question()),
            Stage::Roles(RoleStage::Keying) => Some(key_question(
                "role-key",
                "What is the new role's key, such as work?",
                "pace role add",
            )),
            Stage::Roles(RoleStage::Asking(flow)) => flow.next_question(),
            Stage::Clients(ClientStage::Offer) => Some(self.clients_question()),
            Stage::Clients(ClientStage::Keying) => Some(key_question(
                "client-key",
                "What is the new client's key, such as acme?",
                "pace client add",
            )),
            Stage::Clients(ClientStage::Asking(flow)) => flow.next_question(),
            Stage::Clients(ClientStage::Rate(flow)) => flow.next_question(),
            Stage::Directory(DirectoryStage::Offer) => Some(self.directory_question()),
            Stage::Directory(DirectoryStage::Init(flow)) => flow.next_question(),
            Stage::Directory(DirectoryStage::Record(flow)) => flow.next_question(),
            Stage::Summary => Some(Self::summary_question()),
            Stage::Ended => None,
        }
    }

    fn take_answer(&mut self, answer: Answer) -> Result<(), String> {
        let question = self
            .next_question()
            .ok_or("every question has its answer")?;
        self.take(question.id, answer.clone())?;
        self.answers.push(SavedAnswer::of(&question, answer));
        self.settle().map_err(|error| told(&error))
    }

    fn finish(self) -> PaceResult<Ended<SetupOutcome>> {
        if self.changes.is_empty() {
            return Ok(Ended {
                edits: Vec::new(),
                made: SetupOutcome::NothingToChange,
            });
        }
        let changes = self.summary_lines();
        if self.accepted != Some(true) {
            return Ok(Ended {
                edits: Vec::new(),
                made: SetupOutcome::Declined { changes },
            });
        }
        let mut files: Vec<String> = Vec::new();
        for change in &self.changes {
            let file = file_label(&change.edit.file, &self.inputs.home);
            if !files.contains(&file) {
                files.push(file);
            }
        }
        Ok(Ended {
            edits: self.changes.into_iter().map(|change| change.edit).collect(),
            made: SetupOutcome::Accepted { changes, files },
        })
    }
}

/// The change of the setting `key` in the global file from `before` to
/// `value`.
fn setting_change(inputs: &SetupInputs, key: Key, value: &str, before: &str) -> SetupChange {
    let parts: Vec<&str> = key.name().split('.').collect();
    SetupChange::replacing(
        FieldEdit::set(
            &inputs.config_file,
            &parts,
            FieldValue::String(value.to_owned()),
        ),
        before,
    )
}

/// A change that `pace project init` decided: the manifest's name it
/// removes had `name`; everything else is new.
fn manifest_change(edit: FieldEdit, name: Option<&String>) -> SetupChange {
    match (&edit.change, name) {
        (FieldChange::Remove, Some(name)) => SetupChange::replacing(edit, name.clone()),
        _ => SetupChange::added(edit),
    }
}

/// `catalog` with `role` added.
fn with_role(catalog: &Catalog, role: Role) -> PaceResult<Catalog> {
    Catalog::new(
        catalog.roles().cloned().chain([role]).collect(),
        catalog.clients().cloned().collect(),
        catalog.projects().cloned().collect(),
    )
}

/// `catalog` with `client` added, or in place of the client of its key.
fn with_client(catalog: &Catalog, client: Client) -> PaceResult<Catalog> {
    let mut clients: Vec<Client> = catalog
        .clients()
        .filter(|known| known.key != client.key)
        .cloned()
        .collect();
    clients.push(client);
    Catalog::new(
        catalog.roles().cloned().collect(),
        clients,
        catalog.projects().cloned().collect(),
    )
}

#[cfg(test)]
mod tests;
