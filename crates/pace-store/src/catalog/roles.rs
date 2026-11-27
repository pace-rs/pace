//! The roles file, `roles.toml`: a table per role key with its ID, kind,
//! name, purpose, weekly target and archive.

use pace_core::{Role, RoleKey};
use pace_error::PaceResult;

use crate::{
    catalog::{
        Field, Mistakes, SourceMentions, every,
        fields::{
            Fields, Owner, archived, duration, invalid_value, parsed, records, required, text,
        },
        only,
    },
    toml_file::{TomlFile, TomlKey, TomlValue},
};

/// The keys of a role's table.
const ROLE_KEYS: [&str; 6] = ["id", "kind", "name", "purpose", "weekly-target", "archived"];

/// The roles of the roles file.
///
/// # Errors
///
/// If it is no valid TOML, has another `version`, a typo of `version`
/// (see [`records`]), or a role that [`read_role`] refuses: every
/// role that it refuses, in their order.
pub(super) fn read_roles(
    file: &TomlFile<'_>,
    mentions: &mut SourceMentions<'_>,
) -> Result<Vec<Role>, Mistakes> {
    let document = file.parse().map_err(only)?;
    let table = document.table();
    file.check_version(table).map_err(only)?;
    every(
        records(file, table)
            .map_err(only)?
            .into_iter()
            .map(|(key, value)| read_role(file, key, value, mentions)),
    )
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
        table: key.name(),
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
