//! Roles: the lasting responsibilities that time goes to, such as work or
//! health, and how a role or a project is archived.

use jiff::{SignedDuration, civil::Date};

use crate::{RoleId, RoleKey};

keyword_enum!(
    /// What kind of role it is: a lasting area, an area that keeps producing
    /// projects, or the system of organising time itself.
    RoleKind, "role kind", {
        /// A lasting responsibility, such as work or health.
        Area => "area",
        /// A role that keeps producing projects.
        AreaProject => "area-project",
        /// The system itself: time spent planning and reviewing.
        Meta => "meta",
    }
);

keyword_enum!(
    /// Why something was archived.
    ArchiveReason, "reason to archive", {
        /// Its result is reached.
        Completed => "completed",
        /// It was given up before its result.
        Abandoned => "abandoned",
        /// Its time is over, such as a contract that ended.
        Ended => "ended",
        /// Nothing happens in it any more.
        Inactive => "inactive",
    }
);

/// When and why something was archived. pace archives, it never deletes, so
/// old entries keep resolving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Archived {
    /// The day it was archived.
    pub on: Date,
    /// Why.
    pub reason: ArchiveReason,
}

/// A role: a lasting responsibility that time goes to, such as `work`.
///
/// Entries refer to the role by its ID; people type its key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    /// Its identity, which a new key never changes.
    pub id: RoleId,
    /// What people type: `work`.
    pub key: RoleKey,
    /// Area, area project or meta.
    pub kind: RoleKind,
    /// What people read: `Work`.
    pub name: String,
    /// Why the role exists.
    pub purpose: Option<String>,
    /// The time the role should get each week.
    pub weekly_target: Option<SignedDuration>,
    /// When and why it was archived; `None` while it is in use.
    pub archived: Option<Archived>,
}
