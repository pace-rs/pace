//! The roles file, `roles.toml`: a table per role key with its ID, kind,
//! name, purpose, weekly target and archive.

use pace_core::{Role, RoleKey};
use pace_error::PaceResult;

use crate::{
    catalog::{
        Field, SourceMentions,
        fields::{Fields, Owner, archived, duration, invalid_value, parsed, required, text},
    },
    toml_file::{TomlFile, TomlKey, TomlValue},
};

/// The keys of a role's table.
const ROLE_KEYS: [&str; 6] = ["id", "kind", "name", "purpose", "weekly-target", "archived"];

/// The roles of the roles file.
///
/// # Errors
///
/// If it is no valid TOML, has another `version`, or a role that
/// [`read_role`] refuses.
pub(super) fn read_roles(
    file: &TomlFile<'_>,
    mentions: &mut SourceMentions<'_>,
) -> PaceResult<Vec<Role>> {
    let document = file.parse()?;
    let table = document.table();
    file.check_version(table)?;
    table
        .entries()
        .into_iter()
        .filter(|(key, _)| key.name() != "version")
        .map(|(key, value)| read_role(file, key, value, mentions))
        .collect()
}

/// The role under the table `key`.
///
/// # Errors
///
/// If its key is no role key, it is no table, holds an unknown key, lacks
/// its ID, kind or name, or a value doesn't fit.
fn read_role(
    file: &TomlFile<'_>,
    key: TomlKey<'_>,
    value: TomlValue<'_>,
    mentions: &mut SourceMentions<'_>,
) -> PaceResult<Role> {
    let role_key: RoleKey = key
        .name()
        .parse()
        .map_err(|error| invalid_value(file, error, key.span()))?;
    let fields = Fields::of(file, file.table(key.name(), value)?, &ROLE_KEYS)?;
    let owner = Owner {
        noun: "role",
        key: key.name(),
        span: key.span(),
    };
    let id = required(file, fields.get("id"), &owner, "id")?;
    mentions.record("role", key.name(), Field::Id, id.span());
    let kind = required(file, fields.get("kind"), &owner, "kind")?;
    let name = required(file, fields.get("name"), &owner, "name")?;
    Ok(Role {
        id: parsed(file, "id", id, "the role's ID, a UUIDv7")?,
        key: role_key,
        kind: parsed(file, "kind", kind, "the role's kind")?,
        name: text(file, "name", name, "the role's name")?,
        purpose: fields.read("purpose", |value| {
            text(file, "purpose", value, "why the role exists")
        })?,
        weekly_target: fields.read("weekly-target", |value| {
            duration(file, "weekly-target", value)
        })?,
        archived: fields.read("archived", |value| archived(file, value, &owner))?,
    })
}
