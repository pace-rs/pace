//! The filters of a report: projects, roles, clients, tags and devices;
//! several of one kind combine with "or", different kinds with "and".

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{Catalog, ClientKey, DeviceId, Entry, ProjectRef, RoleId, Tag};

/// Which entries a report counts. An empty set lets every entry through;
/// an entry passes a set when it matches one of its values, and the filter
/// when it passes every set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportFilter {
    /// The entries of these projects, by ID or by key.
    pub projects: BTreeSet<ProjectRef>,
    /// The entries whose effective role is one of these.
    pub roles: BTreeSet<RoleId>,
    /// The entries of projects of these clients.
    pub clients: BTreeSet<ClientKey>,
    /// The entries with one of these tags.
    pub tags: BTreeSet<Tag>,
    /// The entries begun or logged on one of these devices.
    pub devices: BTreeSet<DeviceId>,
}

impl ReportFilter {
    /// Whether the report counts `entry`.
    #[must_use]
    pub fn matches(&self, entry: &Entry, catalog: &Catalog) -> bool {
        let project = entry
            .project
            .as_ref()
            .map(|project| canonical_project(project, catalog));
        let client = entry
            .project
            .as_ref()
            .and_then(|project| catalog.resolve(project))
            .and_then(|record| record.client.as_ref());
        let role = entry.effective_role(catalog);

        any_of(&self.projects, |wanted| {
            project.as_ref() == Some(&canonical_project(wanted, catalog))
        }) && any_of(&self.roles, |wanted| role == Some(*wanted))
            && any_of(&self.clients, |wanted| client == Some(wanted))
            && any_of(&self.tags, |wanted| entry.tags.contains(wanted))
            && any_of(&self.devices, |wanted| entry.device == *wanted)
    }
}

/// Whether `wanted` is empty, or one of its values `matches`.
fn any_of<T>(wanted: &BTreeSet<T>, matches: impl Fn(&T) -> bool) -> bool {
    wanted.is_empty() || wanted.iter().any(matches)
}

/// The project as a report names it: by the ID of its record when the
/// catalog holds one, else as the entry names it. So an entry recorded with
/// a key counts for the project of the record with that key.
pub(crate) fn canonical_project(project: &ProjectRef, catalog: &Catalog) -> ProjectRef {
    catalog
        .resolve(project)
        .map_or_else(|| project.clone(), |record| ProjectRef::Id(record.id))
}
