//! The catalog: the roles, clients and project records that rules, reports
//! and bills take as a value, with their money and rounding.

/// Declares an enum whose values people write as words, such as `area` or
/// `area-project`: `as_str`, `Display` and a `FromStr` whose error names
/// every word.
macro_rules! keyword_enum {
    (
        $(#[$doc:meta])* $name:ident, $noun:literal, {
            $($(#[$variant_doc:meta])* $variant:ident => $word:literal,)+
        }
    ) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $($(#[$variant_doc])* $variant,)+
        }

        impl $name {
            /// The word people write for it.
            #[must_use]
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $word,)+
                }
            }
        }

        impl std::str::FromStr for $name {
            type Err = Box<pace_error::PaceError>;

            fn from_str(word: &str) -> Result<Self, Self::Err> {
                match word {
                    $($word => Ok(Self::$variant),)+
                    _ => Err(pace_error::PaceError::new(
                        pace_error::ErrorKind::InvalidInput,
                        concat!("`{word}` is not a ", $noun),
                    )
                    .attach_context("word", word)
                    .attach_hint(concat!(
                        "a ", $noun, " is one of ",
                        keyword_enum!(@words $($word),+)
                    ))
                    .attach_error_code("pace::core::unknown-word")),
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
    (@words $first:literal $(, $word:literal)*) => {
        concat!("`", $first, "`" $(, ", `", $word, "`")*)
    };
}

mod client;
mod money;
mod project;
mod role;
mod rounding;

use std::{collections::BTreeMap, fmt};

pub use client::{Client, Rate, Rates};
use jiff::civil::Date;
pub use money::{Currency, Money, Totals};
use pace_error::{ErrorKind, PaceError, PaceResult, closest_name};
pub use project::ProjectRecord;
pub use role::{ArchiveReason, Archived, Role, RoleKind};
pub use rounding::{Rounding, RoundingMode, RoundingPer};

use crate::{ClientKey, ProjectId, ProjectKey, ProjectRef, RoleId, RoleKey};

/// The refusals of [`Catalog::collecting`], in the order of its checks.
pub(crate) type Refusals = Vec<Box<PaceError>>;

/// The roles, clients and project records, checked against each other.
///
/// The store reads them from their files; the core takes the catalog as a
/// value wherever a rule, a report or a bill needs it, and reads no file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalog {
    roles: BTreeMap<RoleKey, Role>,
    role_keys: BTreeMap<RoleId, RoleKey>,
    clients: BTreeMap<ClientKey, Client>,
    projects: BTreeMap<ProjectKey, ProjectRecord>,
    project_keys: BTreeMap<ProjectId, ProjectKey>,
}

impl Catalog {
    /// The catalog of `roles`, `clients` and `projects`.
    ///
    /// # Errors
    ///
    /// If two roles, clients or projects share a key, or two roles or
    /// projects an ID; if a project names a client or a role the catalog
    /// doesn't hold; if a client has rates for a project it doesn't hold, or
    /// a rate in another currency than its own. An unknown key names the
    /// closest known one.
    pub fn new(
        roles: Vec<Role>,
        clients: Vec<Client>,
        projects: Vec<ProjectRecord>,
    ) -> PaceResult<Self> {
        let (catalog, refusals) = Self::collecting(roles, clients, projects);
        refusals.into_iter().next().map_or(Ok(catalog), Err)
    }

    /// The catalog of `roles`, `clients` and `projects`, with every
    /// refusal that [`Catalog::new`] stops at the first of: for `pace
    /// doctor`, which names them all.
    ///
    /// A record with a key or an ID that another one has already is left
    /// out; a record that names a key the catalog doesn't hold stays in.
    /// The refusals come in the order [`Catalog::new`] checks them.
    #[must_use]
    pub fn collecting(
        roles: Vec<Role>,
        clients: Vec<Client>,
        projects: Vec<ProjectRecord>,
    ) -> (Self, Refusals) {
        let mut catalog = Self::default();
        let mut refusals = Vec::new();
        for role in roles {
            refusals.extend(catalog.insert_role(role).err());
        }
        for client in clients {
            refusals.extend(catalog.insert_client(client).err());
        }
        for project in projects {
            refusals.extend(catalog.insert_project(project).err());
        }
        refusals.extend(catalog.project_refusals());
        refusals.extend(catalog.client_refusals());
        (catalog, refusals)
    }

    fn insert_role(&mut self, role: Role) -> PaceResult<()> {
        if let Some(other) = self.role_keys.get(&role.id) {
            return Err(same_id("roles", other.as_str(), role.key.as_str(), role.id));
        }
        if self.roles.contains_key(&role.key) {
            return Err(same_key("roles", role.key.as_str()));
        }
        _ = self.role_keys.insert(role.id, role.key.clone());
        _ = self.roles.insert(role.key.clone(), role);
        Ok(())
    }

    fn insert_client(&mut self, client: Client) -> PaceResult<()> {
        if self.clients.contains_key(&client.key) {
            return Err(same_key("clients", client.key.as_str()));
        }
        _ = self.clients.insert(client.key.clone(), client);
        Ok(())
    }

    fn insert_project(&mut self, project: ProjectRecord) -> PaceResult<()> {
        if let Some(other) = self.project_keys.get(&project.id) {
            return Err(same_id(
                "projects",
                other.as_str(),
                project.key.as_str(),
                project.id,
            ));
        }
        if self.projects.contains_key(&project.key) {
            return Err(same_key("projects", project.key.as_str()));
        }
        _ = self.project_keys.insert(project.id, project.key.clone());
        _ = self.projects.insert(project.key.clone(), project);
        Ok(())
    }

    /// A refusal for each project's client and role that doesn't exist.
    fn project_refusals(&self) -> Refusals {
        let mut refusals = Vec::new();
        for project in self.projects.values() {
            if let Some(client) = project.client.as_ref()
                && !self.clients.contains_key(client)
            {
                refusals.push(unknown_reference(
                    ("project", project.key.as_str()),
                    "names",
                    ("client", "pace::core::unknown-client"),
                    client.as_str(),
                    self.clients.keys().map(ClientKey::as_str),
                ));
            }
            if let Some(role) = project.role.as_ref()
                && !self.roles.contains_key(role)
            {
                refusals.push(unknown_reference(
                    ("project", project.key.as_str()),
                    "names",
                    ("role", "pace::core::unknown-role"),
                    role.as_str(),
                    self.roles.keys().map(RoleKey::as_str),
                ));
            }
        }
        refusals
    }

    /// A refusal for each client's project rates that name no project of
    /// this client, and for each client with a rate in another currency
    /// than its own.
    fn client_refusals(&self) -> Refusals {
        let mut refusals = Vec::new();
        for client in self.clients.values() {
            for key in client.project_rates.keys() {
                match self.projects.get(key) {
                    None => refusals.push(unknown_reference(
                        ("client", client.key.as_str()),
                        "has rates for",
                        ("project", "pace::core::unknown-project"),
                        key.as_str(),
                        self.projects.keys().map(ProjectKey::as_str),
                    )),
                    Some(project) if project.client.as_ref() != Some(&client.key) => {
                        refusals.push(rates_for_project_of_other_client(client, project));
                    }
                    Some(_) => {}
                }
            }
            if let Some(rate) = client
                .all_rates()
                .find(|rate| rate.hourly.currency() != client.currency)
            {
                refusals.push(rate_in_other_currency(client, rate));
            }
        }
        refusals
    }

    /// The role with `key`.
    #[must_use]
    pub fn role(&self, key: &RoleKey) -> Option<&Role> {
        self.roles.get(key)
    }

    /// The role with `id`.
    #[must_use]
    pub fn role_by_id(&self, id: RoleId) -> Option<&Role> {
        self.role_keys.get(&id).and_then(|key| self.role(key))
    }

    /// The role of `project`: its key resolved to the role.
    #[must_use]
    pub fn role_of(&self, project: &ProjectRecord) -> Option<&Role> {
        project.role.as_ref().and_then(|key| self.role(key))
    }

    /// The client with `key`.
    #[must_use]
    pub fn client(&self, key: &ClientKey) -> Option<&Client> {
        self.clients.get(key)
    }

    /// The project with `key`.
    #[must_use]
    pub fn project(&self, key: &ProjectKey) -> Option<&ProjectRecord> {
        self.projects.get(key)
    }

    /// The project with `id`.
    #[must_use]
    pub fn project_by_id(&self, id: ProjectId) -> Option<&ProjectRecord> {
        self.project_keys.get(&id).and_then(|key| self.project(key))
    }

    /// The project an entry refers to, by its ID or by its key.
    #[must_use]
    pub fn resolve(&self, project: &ProjectRef) -> Option<&ProjectRecord> {
        match project {
            ProjectRef::Id(id) => self.project_by_id(*id),
            ProjectRef::Key(key) => self.project(key),
        }
    }

    /// The ID of the role of the project an entry refers to, if the catalog
    /// holds its record and the record names a role.
    #[must_use]
    pub fn project_role(&self, project: &ProjectRef) -> Option<RoleId> {
        self.resolve(project)
            .and_then(|record| self.role_of(record))
            .map(|role| role.id)
    }

    /// Whether the project an entry refers to is billable; no without a
    /// record.
    #[must_use]
    pub fn project_billable(&self, project: &ProjectRef) -> bool {
        self.resolve(project).is_some_and(|record| record.billable)
    }

    /// The client of the project an entry refers to; `None` without a
    /// record or a client.
    #[must_use]
    pub fn project_client(&self, project: &ProjectRef) -> Option<&ClientKey> {
        self.resolve(project)
            .and_then(|record| record.client.as_ref())
    }

    /// Whether the entries of the project an entry refers to may overlap
    /// others; no without a record.
    #[must_use]
    pub fn project_allows_overlap(&self, project: &ProjectRef) -> bool {
        self.resolve(project)
            .is_some_and(|record| record.allow_overlap)
    }

    /// The hourly rate of `project` on `day`: the project's rates first,
    /// then its client's. `None` without a client, and before the first
    /// rate (restart.md 3.7).
    #[must_use]
    pub fn rate_on(&self, project: &ProjectRecord, day: Date) -> Option<&Rate> {
        project
            .client
            .as_ref()
            .and_then(|client| self.client(client))
            .and_then(|client| client.rate_on(&project.key, day))
    }

    /// The roles, ordered by key.
    pub fn roles(&self) -> impl Iterator<Item = &Role> {
        self.roles.values()
    }

    /// The clients, ordered by key.
    pub fn clients(&self) -> impl Iterator<Item = &Client> {
        self.clients.values()
    }

    /// The project records, ordered by key.
    pub fn projects(&self) -> impl Iterator<Item = &ProjectRecord> {
        self.projects.values()
    }
}

/// The error for a rate of `client` in another currency than its own.
fn rate_in_other_currency(client: &Client, rate: &Rate) -> Box<PaceError> {
    PaceError::new(
        ErrorKind::InvalidInput,
        "the client `{client}` bills in {currency}, but its rate from {from} is {hourly} {other}",
    )
    .attach_context("client", client.key.as_str())
    .attach_context("currency", client.currency.code())
    .attach_context("from", rate.from.to_string())
    .attach_context("hourly", rate.hourly.to_string())
    .attach_context("other", rate.hourly.currency().code())
    .attach_hint("give every rate of a client in the client's currency")
    .attach_error_code("pace::core::rate-currency")
}

/// The error for two records of one kind with the same key.
fn same_key(plural: &str, key: &str) -> Box<PaceError> {
    PaceError::new(ErrorKind::InvalidInput, "two {plural} have the key `{key}`")
        .attach_context("plural", plural)
        .attach_context("key", key)
        .attach_hint("give each its own key")
        .attach_error_code("pace::core::same-key")
}

/// The error for two records of one kind with the same ID.
fn same_id(plural: &str, first: &str, second: &str, id: impl fmt::Display) -> Box<PaceError> {
    PaceError::new(
        ErrorKind::InvalidInput,
        "the {plural} `{first}` and `{second}` have the same ID {id}",
    )
    .attach_context("plural", plural)
    .attach_context("first", first)
    .attach_context("second", second)
    .attach_context("id", id.to_string())
    .attach_hint("a copied record keeps its ID; give the copy a new one")
    .attach_error_code("pace::core::same-id")
}

/// The error for a client's project rates for a project whose client is
/// another one, or none: `Catalog::rate_on` would never use them.
fn rates_for_project_of_other_client(client: &Client, project: &ProjectRecord) -> Box<PaceError> {
    let error = project.client.as_ref().map_or_else(
        || {
            PaceError::new(
                ErrorKind::InvalidInput,
                "the client `{client}` has rates for the project `{project}`, which has no client",
            )
            .attach_hint(format!(
                "make `{}` the project's client, or remove the rates",
                client.key
            ))
        },
        |owner| {
            PaceError::new(
                ErrorKind::InvalidInput,
                "the client `{client}` has rates for the project `{project}`, which belongs to \
                 the client `{owner}`",
            )
            .attach_context("owner", owner.as_str())
            .attach_hint(format!(
                "move the rates to the client `{owner}`, or make `{}` the project's client",
                client.key
            ))
        },
    );
    error
        .attach_context("client", client.key.as_str())
        .attach_context("project", project.key.as_str())
        .attach_error_code("pace::core::rate-project-client")
}

/// The error for a record, `(owner_noun, owner)`, whose `relation` to a
/// `noun` names a key the catalog doesn't hold, with the error `code` for
/// that noun; its hint names the closest key, else the known ones.
fn unknown_reference<'keys>(
    (owner_noun, owner): (&str, &str),
    relation: &str,
    (noun, code): (&str, &'static str),
    key: &str,
    known: impl Iterator<Item = &'keys str> + Clone,
) -> Box<PaceError> {
    let hint = closest_name(key, known.clone()).map_or_else(
        || {
            let known: Vec<String> = known.map(|known| format!("`{known}`")).collect();
            if known.is_empty() {
                format!("pace knows no {noun} yet")
            } else {
                format!("pace knows these {noun}s: {}", known.join(", "))
            }
        },
        |closest| format!("did you mean `{closest}`?"),
    );
    PaceError::new(
        ErrorKind::InvalidInput,
        "the {owner_noun} `{owner}` {relation} the {noun} `{key}`, which pace doesn't know",
    )
    .attach_context("owner_noun", owner_noun)
    .attach_context("owner", owner)
    .attach_context("relation", relation)
    .attach_context("noun", noun)
    .attach_context("key", key)
    .attach_hint(hint)
    .attach_error_code(code)
}
