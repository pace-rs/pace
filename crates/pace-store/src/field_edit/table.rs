//! Whole tables for the field writer: a table alone as its own TOML text,
//! and its replacement by such a text, in its place in the file.
//!
//! A table of a TOML file is more than its header: `[acme]` has its
//! `[[acme.rates]]`, which may stand anywhere in the file. `toml_edit` orders
//! the tables of a file by their positions, so a replacement takes the
//! positions from the first table of the one it replaces, and the tables
//! after it move on to make room.

use std::path::Path;

use pace_error::{ErrorKind, PaceError, PaceResult, Snippet};
use toml_edit::{DocumentMut, Item, RawString, Table, TomlError};

use crate::{
    field_edit::{next_table_position, parent_table, parsed_document},
    toml_file::TomlCause,
};

/// The table `key` of `text`, the file `path`, alone as a TOML text;
/// `None` if the file has no such table.
///
/// The text holds the comments above and inside the table and every table
/// below it, such as `[[acme.rates]]`. It starts at its first comment or
/// header, without the blank lines above.
///
/// # Errors
///
/// If `text` is no valid TOML.
pub fn table_text(path: &Path, text: &str, key: &[&str]) -> PaceResult<Option<String>> {
    let document = parsed_document(path, text)?;
    let Some((last, parents)) = key.split_last() else {
        return Ok(None);
    };
    let mut item = document.as_item();
    for part in key {
        let Some(next) = item.as_table_like().and_then(|table| table.get(part)) else {
            return Ok(None);
        };
        item = next;
    }
    let mut alone = DocumentMut::new();
    let mut table = alone.as_table_mut();
    for part in parents {
        let mut implicit = Table::new();
        implicit.set_implicit(true);
        _ = table.insert(part, Item::Table(implicit));
        let Some(Item::Table(next)) = table.get_mut(part) else {
            return Ok(None);
        };
        table = next;
    }
    _ = table.insert(last, item.clone());
    Ok(Some(
        alone
            .to_string()
            .trim_start_matches(['\n', '\r'])
            .to_owned(),
    ))
}

/// Replace the table `key` of `document` with the table that `text` holds
/// at `key`. Its tables take the place of the first table of the old one,
/// keeping their own order; the blank lines above the old table stay. A
/// table that `document` lacks goes at the end of the file.
///
/// # Errors
///
/// If `text` is no valid TOML, holds a key outside of the table `key`, or
/// holds no table `key`: errors of the kind `InvalidInput`.
pub(super) fn replace_table(
    document: &mut DocumentMut,
    key: &[String],
    text: &str,
) -> Result<(), Box<PaceError>> {
    let dotted = key.join(".");
    let mut fragment = text
        .parse::<DocumentMut>()
        .map_err(|error| not_valid_table(&dotted, text, &error))?;
    let mut replacement = take_only_table(&mut fragment, key, &dotted)?;

    let old = item_at(document.as_item(), key).and_then(first_table);
    let start = old
        .as_ref()
        .map_or_else(|| next_table_position(document.as_table()), |(at, _)| *at);
    let leading = old.map_or_else(|| "\n".to_owned(), |(_, prefix)| leading_blank(&prefix));

    let mut fragment_positions = Vec::new();
    each_table(&mut replacement, &mut |table| {
        fragment_positions.extend(table.position());
    });
    fragment_positions.sort_unstable();
    let room = isize::try_from(fragment_positions.len()).unwrap_or(isize::MAX);
    for (_, item) in document.as_table_mut().iter_mut() {
        each_table(item, &mut |table| {
            if let Some(position) = table.position().filter(|position| *position > start) {
                table.set_position(Some(position.saturating_add(room)));
            }
        });
    }
    let mut first = true;
    each_table(&mut replacement, &mut |table| {
        let Some(position) = table.position() else {
            return;
        };
        let rank = fragment_positions.partition_point(|earlier| *earlier < position);
        table.set_position(Some(
            start.saturating_add(isize::try_from(rank).unwrap_or(isize::MAX)),
        ));
        if rank == 0 && first {
            first = false;
            let own = prefix_of(table);
            table
                .decor_mut()
                .set_prefix(format!("{leading}{}", own.trim_start_matches(['\n', '\r'])));
        }
    });

    let Some((last, parents)) = key.split_last() else {
        return Ok(());
    };
    let parent = parent_table(document, parents).map_err(|_| no_table_in(&dotted))?;
    _ = parent.table.insert(last, replacement);
    Ok(())
}

/// Take the item at `key` out of `fragment`, which may hold nothing else.
///
/// # Errors
///
/// If `fragment` holds a key outside of `key`, or nothing at `key`.
fn take_only_table(
    fragment: &mut DocumentMut,
    key: &[String],
    dotted: &str,
) -> Result<Item, Box<PaceError>> {
    let mut item = fragment.as_item_mut();
    let mut path = Vec::new();
    for (depth, part) in key.iter().enumerate() {
        let Some(table) = item.as_table_like_mut() else {
            return Err(no_table_in(dotted));
        };
        if let Some((other, _)) = table.iter().find(|(name, _)| *name != part.as_str()) {
            path.push(other.to_owned());
            return Err(outside_of_table(dotted, &path.join(".")));
        }
        path.push(part.clone());
        if depth + 1 == key.len() {
            return table
                .remove(part)
                .filter(|taken| !taken.is_none())
                .ok_or_else(|| no_table_in(dotted));
        }
        let Some(next) = table.get_mut(part) else {
            return Err(no_table_in(dotted));
        };
        item = next;
    }
    Err(no_table_in(dotted))
}

/// The item at `key` below `item`.
fn item_at<'item>(item: &'item Item, key: &[String]) -> Option<&'item Item> {
    key.iter()
        .try_fold(item, |item, part| item.as_table_like()?.get(part))
}

/// The position and the decor before the first table of `item`, the one
/// that stands first in the file.
fn first_table(item: &Item) -> Option<(isize, String)> {
    let mut item = item.clone();
    let mut first: Option<(isize, String)> = None;
    each_table(&mut item, &mut |table| {
        if let Some(position) = table.position()
            && first.as_ref().is_none_or(|(at, _)| position < *at)
        {
            first = Some((position, prefix_of(table)));
        }
    });
    first
}

/// The decor before the header of `table`.
fn prefix_of(table: &Table) -> String {
    table
        .decor()
        .prefix()
        .and_then(RawString::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// The blank lines at the start of `prefix`.
fn leading_blank(prefix: &str) -> String {
    let rest = prefix.trim_start_matches(['\n', '\r']);
    prefix[..prefix.len() - rest.len()].to_owned()
}

/// Call `visit` on each table of `item` and below it, the tables of its
/// arrays of tables included.
fn each_table(item: &mut Item, visit: &mut impl FnMut(&mut Table)) {
    match item {
        Item::Table(table) => {
            visit(table);
            for (_, below) in table.iter_mut() {
                each_table(below, visit);
            }
        }
        Item::ArrayOfTables(array) => {
            for table in array.iter_mut() {
                visit(table);
                for (_, below) in table.iter_mut() {
                    each_table(below, visit);
                }
            }
        }
        Item::None | Item::Value(_) => {}
    }
}

/// The error for a text of the table `key` that is no valid TOML.
fn not_valid_table(key: &str, text: &str, error: &TomlError) -> Box<PaceError> {
    let message = error.message().trim();
    PaceError::with_source(
        ErrorKind::InvalidInput,
        "the text for the table `{key}` is not valid TOML",
        TomlCause(message.to_owned()),
    )
    .attach_context("key", key)
    .attach_hint("fix the TOML at the marked spot")
    .attach_error_code("pace::store::table-syntax")
    .attach_snippet(Snippet::new(
        format!("[{key}]"),
        text,
        error.span().unwrap_or(0..0),
        message,
    ))
}

/// The error for a text of the table `key` that holds `other` too.
fn outside_of_table(key: &str, other: &str) -> Box<PaceError> {
    PaceError::new(
        ErrorKind::InvalidInput,
        "the text for the table `{key}` holds `{other}`, outside of the table",
    )
    .attach_context("key", key)
    .attach_context("other", other)
    .attach_hint(format!(
        "keep only `[{key}]` and the tables below it, such as `[{key}.…]`"
    ))
    .attach_error_code("pace::store::table-outside")
}

/// The error for a text of the table `key` without that table.
fn no_table_in(key: &str) -> Box<PaceError> {
    PaceError::new(
        ErrorKind::InvalidInput,
        "the text for the table `{key}` holds no table `{key}`",
    )
    .attach_context("key", key)
    .attach_hint(format!("keep the header `[{key}]`"))
    .attach_error_code("pace::store::table-missing")
}
