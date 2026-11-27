//! The fields of the catalog's records: known keys, required keys, and the
//! values they share, such as IDs, durations and an archive.

use std::{ops::Range, str::FromStr};

use jiff::SignedDuration;
use pace_core::Archived;
use pace_error::{PaceError, PaceResult};
use pace_time::parse_duration;

use crate::toml_file::{TomlFile, TomlKey, TomlTable, TomlValue};

/// The fields of one table, each a key that its reader knows.
#[derive(Debug)]
pub(super) struct Fields<'file>(Vec<(TomlKey<'file>, TomlValue<'file>)>);

impl<'file> Fields<'file> {
    /// The fields of `table`.
    ///
    /// # Errors
    ///
    /// If it holds a key that is not in `known`; the error names the
    /// closest.
    pub(super) fn of(
        file: &TomlFile<'_>,
        table: TomlTable<'file>,
        known: &[&str],
    ) -> PaceResult<Self> {
        let entries = table.entries();
        if let Some((key, _)) = entries.iter().find(|(key, _)| !known.contains(&key.name())) {
            return Err(file.unknown_key(key.name(), key.span(), known));
        }
        Ok(Self(entries))
    }

    /// The value of `name`, if the table holds it.
    pub(super) fn get(&self, name: &str) -> Option<TomlValue<'file>> {
        self.0
            .iter()
            .find(|(key, _)| key.name() == name)
            .map(|(_, value)| *value)
    }

    /// The value of `name`, read by `read`; `None` without it.
    ///
    /// # Errors
    ///
    /// If `read` refuses the value.
    pub(super) fn read<T>(
        &self,
        name: &str,
        read: impl FnOnce(TomlValue<'file>) -> PaceResult<T>,
    ) -> PaceResult<Option<T>> {
        self.get(name).map(read).transpose()
    }
}

/// The record a table holds, so an error about a missing key names it and
/// points at it.
#[derive(Debug, Clone)]
pub(super) struct Owner<'key> {
    /// What it is: `role`, `client`, `project`.
    pub(super) noun: &'static str,
    /// Its key.
    pub(super) key: &'key str,
    /// Where it stands in its file.
    pub(super) span: Range<usize>,
}

/// The value of `field`, which `owner` must hold.
///
/// # Errors
///
/// If it is `None`; the error names the owner and the field.
pub(super) fn required<'file>(
    file: &TomlFile<'_>,
    value: Option<TomlValue<'file>>,
    owner: &Owner<'_>,
    field: &str,
) -> PaceResult<TomlValue<'file>> {
    value.ok_or_else(|| {
        file.error_at(
            owner.span.clone(),
            "the {noun} `{key}` has no `{field}`",
            [
                ("noun", owner.noun.to_owned()),
                ("key", owner.key.to_owned()),
                ("field", field.to_owned()),
            ],
            format!("add `{field}` to the {}", owner.noun),
            &format!("this {}", owner.noun),
        )
        .attach_error_code(format!("{}::missing-key", file.code))
    })
}

/// The text of `name` as a `T`, such as a key, an ID or a kind; its error
/// points at the value.
///
/// # Errors
///
/// If the value is no text (the error says what it takes, `expects`), or
/// `T` refuses it.
pub(super) fn parsed<T>(
    file: &TomlFile<'_>,
    name: &str,
    value: TomlValue<'_>,
    expects: &str,
) -> PaceResult<T>
where
    T: FromStr<Err = Box<PaceError>>,
{
    file.string(name, value, expects)?
        .parse()
        .map_err(|error| invalid_value(file, error, value.span()))
}

/// `error` about the value at `span`, pointed at it, with the file's code
/// for a value that doesn't fit.
pub(super) fn invalid_value(
    file: &TomlFile<'_>,
    error: Box<PaceError>,
    span: Range<usize>,
) -> Box<PaceError> {
    file.point_at(error, span, "this value")
        .attach_error_code(format!("{}::invalid-value", file.code))
}

/// The value as owned text.
///
/// # Errors
///
/// If it is no text.
pub(super) fn text(
    file: &TomlFile<'_>,
    name: &str,
    value: TomlValue<'_>,
    expects: &str,
) -> PaceResult<String> {
    file.string(name, value, expects).map(str::to_owned)
}

/// The value as a duration that people type, such as `30h` or `15m`.
///
/// # Errors
///
/// If it is no text, or no positive duration.
pub(super) fn duration(
    file: &TomlFile<'_>,
    name: &str,
    value: TomlValue<'_>,
) -> PaceResult<SignedDuration> {
    let text = file.string(name, value, "a duration such as \"30h\"")?;
    parse_duration(text).map_err(|error| invalid_value(file, error, value.span()))
}

/// The value as an archive: `{ on = 2027-03-01, reason = "ended" }`.
///
/// # Errors
///
/// If it is no table, holds an unknown key, or lacks `on` or `reason`.
pub(super) fn archived(
    file: &TomlFile<'_>,
    value: TomlValue<'_>,
    owner: &Owner<'_>,
) -> PaceResult<Archived> {
    let fields = Fields::of(file, file.table("archived", value)?, &["on", "reason"])?;
    let owner = Owner {
        span: value.span(),
        ..owner.clone()
    };
    let on = required(file, fields.get("on"), &owner, "archived.on")?;
    let reason = required(file, fields.get("reason"), &owner, "archived.reason")?;
    Ok(Archived {
        on: file.date("on", on)?,
        reason: parsed(file, "reason", reason, "why it was archived")?,
    })
}
