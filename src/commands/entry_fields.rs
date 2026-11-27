//! The flags of an entry's role, who decided and billable, which `begin`,
//! `log` and `adjust` share.

use pace_core::{Catalog, Change, DeterminedBy, RoleId, RoleKey};
use pace_error::PaceResult;

use crate::commands::role::known_role;
use crate::output::usage;

/// `--by-me` or `--by-others`: clap refuses both. Their help names no
/// default, as `adjust` keeps the entry's value where `begin` and `log`
/// leave it unmarked.
#[derive(clap::Args, Debug, Default)]
pub struct DecidedFlags {
    #[arg(
        long,
        conflicts_with = "by_others",
        help = "You decided how you spend the time"
    )]
    by_me: bool,

    #[arg(long, help = "Someone else decided how you spend the time")]
    by_others: bool,
}

impl DecidedFlags {
    /// Who decided, as the flags say; `None` leaves the entry unmarked.
    #[must_use]
    pub const fn determined_by(&self) -> Option<DeterminedBy> {
        match (self.by_me, self.by_others) {
            (true, _) => Some(DeterminedBy::Me),
            (_, true) => Some(DeterminedBy::Others),
            _ => None,
        }
    }
}

/// `--billable` or `--not-billable`: clap refuses both. Their help names
/// no default, as `adjust` keeps the entry's flag where `begin` and `log`
/// take the project's.
#[derive(clap::Args, Debug, Default)]
pub struct BillableFlags {
    #[arg(long, conflicts_with = "not_billable", help = "The time is billable")]
    billable: bool,

    #[arg(long, help = "The time is not billable")]
    not_billable: bool,
}

impl BillableFlags {
    /// Billable or not, as the flags say; `None` takes the project's flag.
    #[must_use]
    pub const fn billable(&self) -> Option<bool> {
        match (self.billable, self.not_billable) {
            (true, _) => Some(true),
            (_, true) => Some(false),
            _ => None,
        }
    }
}

/// The role `--role <key>` names, as its ID; without the flag, `None`,
/// which takes the project's role. An archived role is taken, and a note
/// says that it is archived.
///
/// # Errors
///
/// A usage error if `key` is no role key, or no role of `catalog`: it
/// names the closest key.
pub fn entry_role(
    key: Option<&str>,
    catalog: &Catalog,
    notes: &mut Vec<String>,
) -> PaceResult<Option<RoleId>> {
    key.map(|key| {
        let key: RoleKey = key.parse().map_err(usage)?;
        let role = known_role(catalog, &key).map_err(usage)?;
        if let Some(archived) = role.archived {
            notes.push(format!(
                "The role `{key}` was archived on {}; pace took it all the same.",
                archived.on
            ));
        }
        Ok(role.id)
    })
    .transpose()
}

/// The change `adjust --role <key>` makes: `""` removes the entry's own
/// role, so the project's applies again, as `--note ""` removes the note.
///
/// # Errors
///
/// As [`entry_role`].
pub fn role_change(
    key: Option<&str>,
    catalog: &Catalog,
    notes: &mut Vec<String>,
) -> PaceResult<Change<RoleId>> {
    match key {
        None => Ok(Change::Keep),
        Some("") => Ok(Change::Clear),
        Some(key) => Ok(entry_role(Some(key), catalog, notes)?.map_or(Change::Keep, Change::Set)),
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use pace_core::{ArchiveReason, Archived, Role, RoleKind};

    use super::*;

    fn role(key: &str, id: &str, archived: Option<Archived>) -> Role {
        Role {
            id: id.parse().unwrap(),
            key: key.parse().unwrap(),
            kind: RoleKind::Area,
            name: key.to_owned(),
            purpose: None,
            weekly_target: None,
            archived,
        }
    }

    fn catalog() -> Catalog {
        let archived = Archived {
            on: date(2026, 11, 20),
            reason: ArchiveReason::Inactive,
        };
        Catalog::new(
            vec![
                role("work", "0193a7c4-0000-7000-8000-0000000000a1", None),
                role(
                    "health",
                    "0193a7c4-0000-7000-8000-0000000000a2",
                    Some(archived),
                ),
            ],
            Vec::new(),
            Vec::new(),
        )
        .unwrap()
    }

    #[test]
    fn unknown_role_is_usage_error_naming_closest_key() {
        let error = entry_role(Some("wrok"), &catalog(), &mut Vec::new()).unwrap_err();

        assert_eq!(error.exit_code(), pace_error::ExitCode::Usage);
        assert_eq!(error.hint(), Some("did you mean `work`?"));
    }

    #[test]
    fn archived_role_is_taken_with_note() {
        let mut notes = Vec::new();

        let role = entry_role(Some("health"), &catalog(), &mut notes).unwrap();

        assert_eq!(
            role,
            Some("0193a7c4-0000-7000-8000-0000000000a2".parse().unwrap())
        );
        assert_eq!(
            notes,
            ["The role `health` was archived on 2026-11-20; pace took it all the same."]
        );
    }

    #[test]
    fn empty_role_of_adjust_clears_and_missing_flag_keeps() {
        let catalog = catalog();

        assert_eq!(
            role_change(Some(""), &catalog, &mut Vec::new()).unwrap(),
            Change::Clear
        );
        assert_eq!(
            role_change(None, &catalog, &mut Vec::new()).unwrap(),
            Change::Keep
        );
    }

    #[test]
    fn pair_of_flags_gives_one_value_or_none() {
        let decided = DecidedFlags {
            by_others: true,
            ..DecidedFlags::default()
        };
        let billable = BillableFlags {
            not_billable: true,
            ..BillableFlags::default()
        };

        assert_eq!(decided.determined_by(), Some(DeterminedBy::Others));
        assert_eq!(billable.billable(), Some(false));
        assert_eq!(DecidedFlags::default().determined_by(), None);
        assert_eq!(BillableFlags::default().billable(), None);
    }
}
