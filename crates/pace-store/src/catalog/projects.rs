//! The project records, `projects/<key>.toml`: one file per project with
//! its ID, key, name, client, role, billing, budget, days and archive.

use pace_core::{ProjectKey, ProjectRecord};
use pace_error::PaceResult;

use crate::{
    catalog::{
        Field, SourceMentions,
        fields::{Fields, Owner, archived, duration, parsed, required, text},
    },
    toml_file::{TomlFile, TomlValue},
};

/// The keys of a project record. `[remotes]` comes with phase 5.
const PROJECT_KEYS: [&str; 13] = [
    "version",
    "id",
    "key",
    "name",
    "client",
    "role",
    "billable",
    "budget",
    "starts-on",
    "ends-on",
    "done-when",
    "allow-overlap",
    "archived",
];

/// The project record in `file`.
///
/// # Errors
///
/// If it is no valid TOML, has another `version`, holds an unknown key,
/// lacks its ID, key or name, holds another key than its file name, or a
/// value doesn't fit.
pub(super) fn read_project(
    file: &TomlFile<'_>,
    mentions: &mut SourceMentions<'_>,
) -> PaceResult<ProjectRecord> {
    let document = file.parse()?;
    let table = document.table();
    file.check_version(table)?;
    let fields = Fields::of(file, table, &PROJECT_KEYS)?;
    let stem = file
        .path
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    let owner = Owner {
        noun: "project",
        key: &stem,
        span: 0..0,
    };
    let id = required(file, fields.get("id"), &owner, "id")?;
    let key = required(file, fields.get("key"), &owner, "key")?;
    let name = required(file, fields.get("name"), &owner, "name")?;
    let key = own_key(file, key, &stem)?;
    mentions.record("project", key.as_str(), Field::Id, id.span());
    let client = fields.read("client", |value| {
        mentions.record("project", key.as_str(), Field::Client, value.span());
        parsed(file, "client", value, "the key of the project's client")
    })?;
    let role = fields.read("role", |value| {
        mentions.record("project", key.as_str(), Field::Role, value.span());
        parsed(file, "role", value, "the key of the project's role")
    })?;
    let flag = |name| fields.read(name, |value| file.boolean(name, value));
    Ok(ProjectRecord {
        id: parsed(file, "id", id, "the project's ID, a UUIDv7")?,
        name: text(file, "name", name, "the project's name")?,
        client,
        role,
        billable: flag("billable")?.unwrap_or(false),
        budget: fields.read("budget", |value| duration(file, "budget", value))?,
        starts_on: fields.read("starts-on", |value| file.date("starts-on", value))?,
        ends_on: fields.read("ends-on", |value| file.date("ends-on", value))?,
        done_when: fields.read("done-when", |value| {
            text(file, "done-when", value, "the result that makes it done")
        })?,
        allow_overlap: flag("allow-overlap")?.unwrap_or(false),
        archived: fields.read("archived", |value| archived(file, value, &owner))?,
        key,
    })
}

/// The record's `key`, which must be the name of its file, `stem`.
///
/// # Errors
///
/// If it is no project key, or another one than `stem`; the error names
/// both.
fn own_key(file: &TomlFile<'_>, value: TomlValue<'_>, stem: &str) -> PaceResult<ProjectKey> {
    let key: ProjectKey = parsed(
        file,
        "key",
        value,
        "the project's key like \"acme-webshop\"",
    )?;
    if key.as_str() == stem {
        return Ok(key);
    }
    let name = file
        .path
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    Err(file
        .error_at(
            value.span(),
            "the project file {file} holds the key `{key}`",
            [("file", name), ("key", key.as_str().to_owned())],
            format!("rename the file to {key}.toml, or write `key = \"{stem}\"`"),
            "this key",
        )
        .attach_error_code(format!("{}::key-not-file-name", file.code)))
}
