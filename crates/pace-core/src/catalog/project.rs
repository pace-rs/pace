//! Project records: what pace knows of a project beyond its key, such as
//! its client, its role and whether its time is billable.

use jiff::{SignedDuration, civil::Date};

use crate::{Archived, ClientKey, ProjectId, ProjectKey, RoleKey};

/// The record of a project.
///
/// Entries refer to the project by its ID; people type its key. The record
/// names its client and its role by key, as its file does; the
/// [`Catalog`](crate::Catalog) resolves them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRecord {
    /// Its identity, which a new key never changes.
    pub id: ProjectId,
    /// What people type: `acme-webshop`.
    pub key: ProjectKey,
    /// What people read: `Webshop relaunch`.
    pub name: String,
    /// Who gets billed for it.
    pub client: Option<ClientKey>,
    /// The role its time goes to.
    pub role: Option<RoleKey>,
    /// Whether its entries are billable unless they say otherwise.
    pub billable: bool,
    /// The time it may take.
    pub budget: Option<SignedDuration>,
    /// Its first day.
    pub starts_on: Option<Date>,
    /// Its last day.
    pub ends_on: Option<Date>,
    /// The result that makes it done.
    pub done_when: Option<String>,
    /// Whether its entries may overlap others.
    pub allow_overlap: bool,
    /// When and why it was archived; `None` while it is in use.
    pub archived: Option<Archived>,
}
