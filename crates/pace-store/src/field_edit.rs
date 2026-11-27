//! The field writer of pace's TOML files: it sets, removes and appends
//! single fields, so the comments and the order of a file edited by hand
//! survive.
//!
//! [`write_field_edits`] takes a list of [`FieldEdit`]s. Each names a file,
//! a key path such as `["acme", "name"]`, and its [`FieldChange`]. A key
//! that exists keeps its place, its comment and the formatting around it;
//! a new key goes at the end of its table, a new table at the end of the
//! file. A file that doesn't exist yet starts with `version = 1`.
//!
//! The writer knows nothing of what the files mean: the callers do.

use std::{
    io,
    ops::Range,
    path::{Path, PathBuf},
};

use jiff::civil::Date;
use pace_error::{ErrorKind, PaceError, PaceResult, Snippet};
use tempfile::NamedTempFile;
use toml_edit::{
    Array, ArrayOfTables, DocumentMut, InlineTable, Item, RawString, Table, TableLike, Value,
};

use crate::{
    disk::{FileOp, io_error, prepare_temporary_file},
    renames::{Prepared, persist, rename_all},
    toml_file::not_valid_toml,
};

/// The text a file that pace starts holds before its first edit.
const NEW_FILE: &str = "version = 1\n";

/// How to take back the part of a write that a file keeps when a rename
/// fails and pace can't put the file back.
const KEPT_FIELDS_HINT: &str = "edit the file by hand to take back the change";

/// A value that [`FieldChange::Set`] writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue {
    /// `name = "ACME Inc."`
    String(String),
    /// `seats = 12`
    Integer(i64),
    /// `active = true`
    Boolean(bool),
    /// A local date: `from = 2026-07-01`.
    Date(Date),
    /// A decimal, written with the text it was given: `95.50` stays
    /// `95.50`, `1200` stays `1200`.
    Decimal(String),
    /// `tags = ["software", "staplers"]`
    Strings(Vec<String>),
    /// `contact = { name = "Bill", since = 2026-07-01 }`, its keys in the
    /// order given.
    InlineTable(Vec<(String, Self)>),
}

/// What a [`FieldEdit`] does at its key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldChange {
    /// Set the key to the value; the tables on its path are made if
    /// missing.
    Set(FieldValue),
    /// Remove the key, if the file holds it.
    Remove,
    /// Append a table with these fields to the array of tables at the key,
    /// such as `[[acme.rates]]`.
    AppendTable(Vec<(String, FieldValue)>),
}

/// One change to one key of one TOML file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldEdit {
    /// The file.
    pub file: PathBuf,
    /// The key path from the top of the file: `["time-zone"]`,
    /// `["acme", "name"]`.
    pub key: Vec<String>,
    /// What to do there.
    pub change: FieldChange,
}

impl FieldEdit {
    /// Set `key` of `file` to `value`.
    #[must_use]
    pub fn set(file: impl AsRef<Path>, key: &[&str], value: FieldValue) -> Self {
        Self::new(file, key, FieldChange::Set(value))
    }

    /// Remove `key` of `file`.
    #[must_use]
    pub fn remove(file: impl AsRef<Path>, key: &[&str]) -> Self {
        Self::new(file, key, FieldChange::Remove)
    }

    /// Append a table of `fields` to the array of tables `key` of `file`.
    #[must_use]
    pub fn append_table(
        file: impl AsRef<Path>,
        key: &[&str],
        fields: Vec<(String, FieldValue)>,
    ) -> Self {
        Self::new(file, key, FieldChange::AppendTable(fields))
    }

    fn new(file: impl AsRef<Path>, key: &[&str], change: FieldChange) -> Self {
        Self {
            file: file.as_ref().to_path_buf(),
            key: key.iter().map(|&part| part.to_owned()).collect(),
            change,
        }
    }
}

/// Apply `edits`, grouped by file in the order each file first comes up.
///
/// Each file is read, or started with `version = 1`; its parent directory
/// is created. Every changed file is written to a temporary file next to
/// it first, then the renames follow, all or none. A file that the edits
/// leave as it was is not written.
///
/// # Errors
///
/// If a file can't be read or written, if it is no valid TOML, or if a
/// key's path runs through a value that is no table. An error before the
/// renames leaves every file as it was.
pub fn write_field_edits(edits: Vec<FieldEdit>) -> PaceResult<()> {
    write_field_edits_with(edits, persist)
}

/// [`write_field_edits`], with `rename` as the step that renames a
/// temporary file over its file.
fn write_field_edits_with(
    edits: Vec<FieldEdit>,
    rename: impl FnMut(NamedTempFile, &Path) -> io::Result<()>,
) -> PaceResult<()> {
    let mut prepared = Vec::new();
    for (path, edits) in edits_by_file(edits) {
        let before = read_if_exists(&path)?;
        let start = before.as_deref().unwrap_or(NEW_FILE);
        let text = edited_text(&path, start, &edits)?;
        if text == start {
            continue;
        }
        let (temporary, path) = prepare_temporary_file(&path, &text)?;
        prepared.push(Prepared {
            temporary,
            path,
            before,
        });
    }
    rename_all(prepared, rename, KEPT_FIELDS_HINT)
}

/// `edits` grouped by file, the files in the order they first come up.
fn edits_by_file(edits: Vec<FieldEdit>) -> Vec<(PathBuf, Vec<FieldEdit>)> {
    let mut files: Vec<(PathBuf, Vec<FieldEdit>)> = Vec::new();
    for edit in edits {
        match files.iter_mut().find(|(file, _)| *file == edit.file) {
            Some((_, group)) => group.push(edit),
            None => files.push((edit.file.clone(), vec![edit])),
        }
    }
    files
}

/// The text of `path`; `None` if it doesn't exist.
fn read_if_exists(path: &Path) -> PaceResult<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error(FileOp::Read, path, error)),
    }
}

/// `text` of `path` with `edits` applied.
fn edited_text(path: &Path, text: &str, edits: &[FieldEdit]) -> PaceResult<String> {
    let mut document = text.parse::<DocumentMut>().map_err(|error| {
        not_valid_toml(
            ErrorKind::InputOutput,
            "pace::store",
            path,
            text,
            error.span().unwrap_or(0..0),
            error.message(),
        )
    })?;
    for edit in edits {
        apply(&mut document, edit).map_err(|refused| refused.into_error(path, text))?;
    }
    Ok(document.to_string())
}

/// Why [`apply`] refused an edit, before it knows the file.
enum Refused {
    /// The value at `key` is no table, but the path runs through it.
    NoTable { key: Vec<String> },
    /// The value at `key` is no array of tables to append to.
    NoArrayOfTables { key: Vec<String> },
    /// A bug of the caller.
    Internal(Box<PaceError>),
}

impl Refused {
    fn into_error(self, path: &Path, text: &str) -> Box<PaceError> {
        let (guidance, what, key) = match self {
            Self::Internal(error) => return error,
            Self::NoTable { key } => (
                "{file} holds {key} as a value, not as a table",
                "a table",
                key,
            ),
            Self::NoArrayOfTables { key } => (
                "{file} holds {key} as a value, not as an array of tables",
                "an array of tables",
                key,
            ),
        };
        let dotted = key.join(".");
        let error = PaceError::new(ErrorKind::InputOutput, guidance)
            .attach_context("file", path.display().to_string())
            .attach_context("key", dotted.clone())
            .attach_hint(format!("make `{dotted}` {what} in the file, or remove it"))
            .attach_error_code("pace::store::field-edit");
        match span_of(text, &key) {
            Some(span) => error.attach_snippet(Snippet::new(
                path.display().to_string(),
                text,
                span,
                "this value",
            )),
            None => error,
        }
    }
}

/// Apply `edit` to `document`.
fn apply(document: &mut DocumentMut, edit: &FieldEdit) -> Result<(), Refused> {
    let Some((last, parents)) = edit.key.split_last() else {
        return Err(Refused::Internal(
            PaceError::new(
                ErrorKind::Internal,
                "pace tried to edit a field without a key",
            )
            .ask_report(),
        ));
    };
    match &edit.change {
        FieldChange::Remove => remove_key(document.as_item_mut(), parents, last),
        FieldChange::Set(value) => {
            let value = toml_value(value)?;
            let parent = parent_table(document, parents)?;
            set_value(parent.table, last, value, parent.inline);
        }
        FieldChange::AppendTable(fields) => {
            let mut appended = Table::new();
            for (key, value) in fields {
                _ = appended.insert(key, Item::Value(toml_value(value)?));
            }
            let parent = parent_table(document, parents)?;
            append_table(parent, last, appended).map_err(|()| Refused::NoArrayOfTables {
                key: edit.key.clone(),
            })?;
        }
    }
    Ok(())
}

/// Remove `key` of the table at `path` below `item`. A path that the file
/// doesn't hold as tables changes nothing.
fn remove_key(item: &mut Item, path: &[String], key: &str) {
    let mut item = item;
    for part in path {
        let Some(next) = item
            .as_table_like_mut()
            .and_then(|table| table.get_mut(part))
        else {
            return;
        };
        item = next;
    }
    let inline = item.is_inline_table();
    let Some(table) = item.as_table_like_mut() else {
        return;
    };
    let was_last = table.iter().last().is_some_and(|(last, _)| last == key);
    if let Some(Item::Value(removed)) = table.remove(key)
        && inline
        && was_last
        && let Some(space) = trailing_space(&removed)
        && let Some((_, Item::Value(last))) = table.iter_mut().last()
    {
        last.decor_mut().set_suffix(space);
    }
}

/// The table that a [`FieldChange::Set`] or [`FieldChange::AppendTable`]
/// writes into.
struct Parent<'document> {
    table: &'document mut dyn TableLike,
    /// Whether it is an inline table.
    inline: bool,
    /// The position for a new table: after every table of the file.
    position: isize,
}

/// The table at `path` in `document`; the tables missing on the way are
/// made, as implicit tables at the end of the file, or as inline tables
/// inside an inline table.
fn parent_table<'document>(
    document: &'document mut DocumentMut,
    path: &[String],
) -> Result<Parent<'document>, Refused> {
    let mut position = next_table_position(document.as_table());
    let mut item = document.as_item_mut();
    for (depth, part) in path.iter().enumerate() {
        let inline = item.is_inline_table();
        let no_table = || Refused::NoTable {
            key: path[..depth].to_vec(),
        };
        let table = item.as_table_like_mut().ok_or_else(no_table)?;
        if table.get(part).is_none_or(Item::is_none) {
            insert_new(table, part, new_table(inline, &mut position), inline);
        }
        item = table.get_mut(part).ok_or_else(no_table)?;
    }
    let inline = item.is_inline_table();
    let table = item
        .as_table_like_mut()
        .ok_or_else(|| Refused::NoTable { key: path.to_vec() })?;
    Ok(Parent {
        table,
        inline,
        position,
    })
}

/// Insert `item` at `key`, a key that `table` doesn't hold yet, as its last
/// key. In an inline table, the space before the closing brace moves on
/// from the value that was last to the new one.
fn insert_new(table: &mut dyn TableLike, key: &str, mut item: Item, inline: bool) {
    if inline
        && let Some((_, Item::Value(last))) = table.iter_mut().last()
        && let Some(space) = trailing_space(last)
        && let Item::Value(value) = &mut item
    {
        last.decor_mut().set_suffix("");
        value.decor_mut().set_suffix(space);
    }
    _ = table.insert(key, item);
}

/// The space after `value`: in an inline table, the space before its
/// closing brace when `value` is its last.
fn trailing_space(value: &Value) -> Option<String> {
    value
        .decor()
        .suffix()
        .and_then(RawString::as_str)
        .filter(|suffix| !suffix.is_empty() && suffix.trim().is_empty())
        .map(str::to_owned)
}

/// A new, empty table: inline inside an inline table, else implicit at
/// `position`, which moves on.
fn new_table(inline: bool, position: &mut isize) -> Item {
    if inline {
        return Item::Value(Value::InlineTable(InlineTable::new()));
    }
    let mut table = Table::new();
    table.set_implicit(true);
    table.set_position(Some(*position));
    *position += 1;
    Item::Table(table)
}

/// The position after every table of `table`, so a new table goes at the
/// end of the file.
fn next_table_position(table: &Table) -> isize {
    let mut next = table.position().map_or(0, |position| position + 1);
    for (_, item) in table {
        let tables: Vec<&Table> = match item {
            Item::Table(table) => vec![table],
            Item::ArrayOfTables(array) => array.iter().collect(),
            Item::None | Item::Value(_) => Vec::new(),
        };
        for table in tables {
            next = next.max(next_table_position(table));
        }
    }
    next
}

/// Set `key` of `table` to `value`. A value that exists keeps the
/// comment and the spaces around it.
fn set_value(table: &mut dyn TableLike, key: &str, mut value: Value, inline: bool) {
    if table.get(key).is_none_or(Item::is_none) {
        insert_new(table, key, Item::Value(value), inline);
        return;
    }
    if let Some(item) = table.get_mut(key) {
        if let Item::Value(old) = item {
            *value.decor_mut() = old.decor().clone();
        }
        *item = Item::Value(value);
    }
}

/// Append `appended` to the array of tables `key` of `parent`. A new array
/// is an array of tables at the parent's position, or an array of inline
/// tables inside an inline table; an array of inline tables that exists
/// takes it inline. `Err` if `key` holds something else.
fn append_table(parent: Parent<'_>, key: &str, mut appended: Table) -> Result<(), ()> {
    let table = parent.table;
    if table.get(key).is_none_or(Item::is_none) {
        let array = if parent.inline {
            let mut array = Array::new();
            array.push(appended.into_inline_table());
            Item::Value(Value::Array(array))
        } else {
            appended.set_position(Some(parent.position));
            let mut array = ArrayOfTables::new();
            array.push(appended);
            Item::ArrayOfTables(array)
        };
        insert_new(table, key, array, parent.inline);
        return Ok(());
    }
    match table.get_mut(key) {
        Some(Item::ArrayOfTables(array)) => {
            array.push(appended);
            Ok(())
        }
        Some(Item::Value(Value::Array(array))) if array.iter().all(Value::is_inline_table) => {
            array.push(appended.into_inline_table());
            Ok(())
        }
        _ => Err(()),
    }
}

/// `value` as a TOML value.
fn toml_value(value: &FieldValue) -> Result<Value, Refused> {
    Ok(match value {
        FieldValue::String(text) => Value::from(text.as_str()),
        FieldValue::Integer(number) => Value::from(*number),
        FieldValue::Boolean(flag) => Value::from(*flag),
        FieldValue::Date(date) => parsed_value(&date.to_string(), "a date", Value::is_datetime)?,
        FieldValue::Decimal(text) => parsed_value(text, "a decimal", |value| {
            value.is_float() || value.is_integer()
        })?,
        FieldValue::Strings(texts) => Value::Array(texts.iter().map(String::as_str).collect()),
        FieldValue::InlineTable(fields) => {
            let mut table = InlineTable::new();
            for (key, value) in fields {
                _ = table.insert(key, toml_value(value)?);
            }
            Value::InlineTable(table)
        }
    })
}

/// `text` as a TOML value that `fits`, with `text` as its repr.
fn parsed_value(text: &str, what: &str, fits: impl Fn(&Value) -> bool) -> Result<Value, Refused> {
    text.parse::<Value>()
        .ok()
        .filter(|value| fits(value))
        .ok_or_else(|| {
            Refused::Internal(
                PaceError::new(ErrorKind::Internal, "pace can't write `{text}` as {what}")
                    .attach_context("text", text)
                    .attach_context("what", what)
                    .ask_report(),
            )
        })
}

/// Where the value at `key` stands in `text`, for a snippet.
fn span_of(text: &str, key: &[String]) -> Option<Range<usize>> {
    let document = toml_edit::Document::parse(text).ok()?;
    let mut item = document.as_item();
    for part in key {
        item = item.as_table_like()?.get(part)?;
    }
    item.span()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::scratch_dir;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn string(text: &str) -> FieldValue {
        FieldValue::String(text.to_owned())
    }

    #[test]
    fn failed_second_rename_puts_first_file_back() -> TestResult {
        let dir = scratch_dir()?;
        let global = dir.path().join("pace.global.toml");
        let roles = dir.path().join("roles.toml");
        std::fs::write(&global, "version = 1\ntime-zone = \"Europe/Berlin\"\n")?;
        let mut renames = 0;

        let written = write_field_edits_with(
            vec![
                FieldEdit::set(&global, &["time-zone"], string("Europe/Lisbon")),
                FieldEdit::set(&roles, &["developer", "name"], string("Developer")),
            ],
            |temporary, path| {
                renames += 1;
                if renames == 2 {
                    Err(io::Error::other("the test fails this rename"))
                } else {
                    persist(temporary, path)
                }
            },
        );

        let error = written.err().ok_or("the second rename fails")?;
        assert_eq!(
            error.to_string(),
            format!("cannot write {}", roles.display())
        );
        assert_eq!(
            std::fs::read_to_string(&global)?,
            "version = 1\ntime-zone = \"Europe/Berlin\"\n"
        );
        assert!(!roles.exists(), "the write created it");
        Ok(())
    }

    #[test]
    fn key_in_inline_table_is_set_inside_it() -> TestResult {
        let text = "version = 1\n[acme]\ncontact = { name = \"Bill\" }\n";
        let edits = [
            FieldEdit::set("clients.toml", &["acme", "contact", "name"], string("Ann")),
            FieldEdit::set(
                "clients.toml",
                &["acme", "contact", "phone", "office"],
                string("555"),
            ),
        ];

        let edited = edited_text(Path::new("clients.toml"), text, &edits)?;

        assert_eq!(
            edited,
            "version = 1\n[acme]\ncontact = { name = \"Ann\", phone = { office = \"555\" } }\n"
        );
        Ok(())
    }

    #[test]
    fn remove_below_missing_table_in_inline_table_changes_nothing() -> TestResult {
        let text = "version = 1\n[acme]\ncontact = { name = \"Bill\" }\n";
        let edits = [FieldEdit::remove(
            "clients.toml",
            &["acme", "contact", "phone", "office"],
        )];

        let edited = edited_text(Path::new("clients.toml"), text, &edits)?;

        assert_eq!(edited, text);
        Ok(())
    }

    #[test]
    fn remove_of_last_key_in_inline_table_keeps_space_before_brace() -> TestResult {
        let text = "version = 1\nc = { a = 1, b = 2 }\n";
        let edits = [FieldEdit::remove("roles.toml", &["c", "b"])];

        let edited = edited_text(Path::new("roles.toml"), text, &edits)?;

        assert_eq!(edited, "version = 1\nc = { a = 1 }\n");
        Ok(())
    }

    #[test]
    fn append_into_inline_table_writes_inline_array() -> TestResult {
        let text = "version = 1\nacme = { name = \"a\" }\n";
        let rate = || vec![("hourly".to_owned(), FieldValue::Integer(1))];
        let edits = [
            FieldEdit::append_table("clients.toml", &["acme", "rates"], rate()),
            FieldEdit::append_table("clients.toml", &["acme", "rates"], rate()),
        ];

        let edited = edited_text(Path::new("clients.toml"), text, &edits)?;

        assert_eq!(
            edited,
            "version = 1\nacme = { name = \"a\", rates = [{ hourly = 1 }, { hourly = 1 }] }\n"
        );
        Ok(())
    }

    #[test]
    fn append_to_inline_array_of_tables_adds_inline_table() -> TestResult {
        let text = "version = 1\n[acme]\nrates = [{ hourly = 1 }]\n";
        let edits = [FieldEdit::append_table(
            "clients.toml",
            &["acme", "rates"],
            vec![("hourly".to_owned(), FieldValue::Integer(2))],
        )];

        let edited = edited_text(Path::new("clients.toml"), text, &edits)?;

        assert_eq!(
            edited,
            "version = 1\n[acme]\nrates = [{ hourly = 1 }, { hourly = 2 }]\n"
        );
        Ok(())
    }

    #[test]
    fn append_to_value_is_refused() -> TestResult {
        let text = "version = 1\n[acme]\nrates = \"none\"\n";
        let edits = [FieldEdit::append_table(
            "clients.toml",
            &["acme", "rates"],
            Vec::new(),
        )];

        let error = edited_text(Path::new("clients.toml"), text, &edits)
            .err()
            .ok_or("acme.rates is no array of tables")?;

        assert_eq!(
            error.to_string(),
            "clients.toml holds acme.rates as a value, not as an array of tables"
        );
        assert_eq!(
            error.hint(),
            Some("make `acme.rates` an array of tables in the file, or remove it")
        );
        Ok(())
    }

    #[test]
    fn decimal_that_is_no_number_is_bug() -> TestResult {
        let edits = [FieldEdit::set(
            "roles.toml",
            &["developer", "hourly"],
            FieldValue::Decimal("95,50".to_owned()),
        )];

        let error = edited_text(Path::new("roles.toml"), NEW_FILE, &edits)
            .err()
            .ok_or("95,50 is no decimal")?;

        assert_eq!(error.to_string(), "pace can't write `95,50` as a decimal");
        Ok(())
    }

    #[test]
    fn edit_without_key_is_bug() -> TestResult {
        let edits = [FieldEdit::remove("roles.toml", &[])];

        let error = edited_text(Path::new("roles.toml"), NEW_FILE, &edits)
            .err()
            .ok_or("an edit needs a key")?;

        assert_eq!(
            error.to_string(),
            "pace tried to edit a field without a key"
        );
        Ok(())
    }
}
