//! The schema of pace's JSON and CSV, version 1: the column names in one
//! place, and the writers of both formats.
//!
//! A row is a set of cells, each under a [`Column`]. JSON writes a document
//! `{"version": 1, "kind": …, "rows": […]}`, each row an object with the
//! keys of its cells, in the order of [`Column`]. CSV writes a row of the
//! kind's column names, then one line per row; a column the row has no cell
//! for stays empty. A change to a name or a meaning is a new version.

use std::collections::BTreeMap;

use jiff::{SignedDuration, ZonedDateTime};
use pace_core::Money;
use pace_error::{ErrorKind, PaceError, PaceResult};
use pace_time::format_duration;
use serde::{Serialize, Serializer, ser::SerializeMap as _};

/// The version of the schema; a change to it is a new version.
pub const SCHEMA_VERSION: u32 = 1;

/// A column of the schema: the key of a JSON row and the name of a CSV
/// column. The order of the variants is the order of the keys and columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Column {
    /// The part of the report a row belongs to: `time`, `target`, …
    Part,
    /// What the row counts within its part: `role`, `project`, `day`, …
    Group,
    /// The key of a role, project or client; the name of a kind or source.
    Key,
    /// The ID of a role, project or entry.
    Id,
    /// The short ID of an entry.
    Short,
    /// The readable name of a role, project or client.
    Name,
    /// The key of a client.
    Client,
    /// A day, `YYYY-MM-DD`.
    Date,
    /// When something begins, in RFC 9557 with its zone.
    Start,
    /// When something ends, in RFC 9557 with its zone.
    End,
    /// How many.
    Count,
    /// Net time in seconds.
    NetSeconds,
    /// Net time, readable.
    Net,
    /// A share in whole percent.
    Percent,
    /// A target in seconds.
    TargetSeconds,
    /// A target, readable.
    Target,
    /// A budget in seconds.
    BudgetSeconds,
    /// A budget, readable.
    Budget,
    /// Billed time in seconds: net time rounded by the client's rule.
    BilledSeconds,
    /// Billed time, readable.
    Billed,
    /// Hold time in seconds.
    HeldSeconds,
    /// Hold time, readable.
    Held,
    /// Pause time in seconds.
    PausedSeconds,
    /// Pause time, readable.
    Paused,
    /// Money: a decimal string with every digit of the minor unit.
    Amount,
    /// The ISO 4217 code of the amount's currency.
    Currency,
    /// The reason of a hold.
    Reason,
    /// A text: the note of an entry or a catch.
    Note,
    /// The ID of a second entry.
    OtherId,
    /// The short ID of a second entry.
    OtherShort,
    /// Whether the time counts a running entry up to now.
    Running,
}

impl Column {
    /// The column's name, in snake case: `net_seconds`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        COLUMN_NAMES[self as usize].1
    }
}

/// The name of each column, in the order of [`Column`].
const COLUMN_NAMES: [(Column, &str); 31] = [
    (Column::Part, "part"),
    (Column::Group, "group"),
    (Column::Key, "key"),
    (Column::Id, "id"),
    (Column::Short, "short"),
    (Column::Name, "name"),
    (Column::Client, "client"),
    (Column::Date, "date"),
    (Column::Start, "start"),
    (Column::End, "end"),
    (Column::Count, "count"),
    (Column::NetSeconds, "net_seconds"),
    (Column::Net, "net"),
    (Column::Percent, "percent"),
    (Column::TargetSeconds, "target_seconds"),
    (Column::Target, "target"),
    (Column::BudgetSeconds, "budget_seconds"),
    (Column::Budget, "budget"),
    (Column::BilledSeconds, "billed_seconds"),
    (Column::Billed, "billed"),
    (Column::HeldSeconds, "held_seconds"),
    (Column::Held, "held"),
    (Column::PausedSeconds, "paused_seconds"),
    (Column::Paused, "paused"),
    (Column::Amount, "amount"),
    (Column::Currency, "currency"),
    (Column::Reason, "reason"),
    (Column::Note, "note"),
    (Column::OtherId, "other_id"),
    (Column::OtherShort, "other_short"),
    (Column::Running, "running"),
];

/// A duration of the schema: two columns, `<name>_seconds` and a
/// readable `<name>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Length {
    /// Net time.
    Net,
    /// A target.
    Target,
    /// A budget.
    Budget,
    /// Billed time.
    Billed,
    /// Hold time.
    Held,
    /// Pause time.
    Paused,
}

impl Length {
    /// The column of the seconds and the column of the readable duration.
    #[must_use]
    pub const fn columns(self) -> (Column, Column) {
        match self {
            Self::Net => (Column::NetSeconds, Column::Net),
            Self::Target => (Column::TargetSeconds, Column::Target),
            Self::Budget => (Column::BudgetSeconds, Column::Budget),
            Self::Billed => (Column::BilledSeconds, Column::Billed),
            Self::Held => (Column::HeldSeconds, Column::Held),
            Self::Paused => (Column::PausedSeconds, Column::Paused),
        }
    }
}

/// What a document of the schema holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaKind {
    /// The report of `reflect`.
    Reflect,
}

impl SchemaKind {
    /// The kind's name, the `kind` of its JSON document.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Reflect => "reflect",
        }
    }

    /// The columns of the kind's CSV, in their order.
    #[must_use]
    pub const fn columns(self) -> &'static [Column] {
        match self {
            Self::Reflect => REFLECT_COLUMNS,
        }
    }
}

/// The columns of `reflect`.
const REFLECT_COLUMNS: &[Column] = &[
    Column::Part,
    Column::Group,
    Column::Key,
    Column::Id,
    Column::Short,
    Column::Name,
    Column::Client,
    Column::Date,
    Column::Start,
    Column::End,
    Column::Count,
    Column::NetSeconds,
    Column::Net,
    Column::Percent,
    Column::TargetSeconds,
    Column::Target,
    Column::BudgetSeconds,
    Column::Budget,
    Column::BilledSeconds,
    Column::Billed,
    Column::HeldSeconds,
    Column::Held,
    Column::PausedSeconds,
    Column::Paused,
    Column::Amount,
    Column::Currency,
    Column::Reason,
    Column::Note,
    Column::OtherId,
    Column::OtherShort,
    Column::Running,
];

/// The value of a cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cell {
    /// A string; in JSON a string.
    Text(String),
    /// A whole number; in JSON a number.
    Number(i64),
    /// Yes or no; in JSON a boolean, in CSV `true` or `false`.
    Flag(bool),
}

impl Cell {
    /// The cell as a CSV field, quoted when it needs to be.
    fn csv_field(&self) -> String {
        match self {
            Self::Text(text) => csv_quoted(text),
            Self::Number(number) => number.to_string(),
            Self::Flag(flag) => flag.to_string(),
        }
    }
}

impl Serialize for Cell {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Text(text) => serializer.serialize_str(text),
            Self::Number(number) => serializer.serialize_i64(*number),
            Self::Flag(flag) => serializer.serialize_bool(*flag),
        }
    }
}

/// A row of the schema: its cells by column.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Row {
    cells: BTreeMap<Column, Cell>,
}

impl Row {
    /// A row of `part` and `group`.
    #[must_use]
    pub fn of(part: &str, group: &str) -> Self {
        Self::default()
            .text(Column::Part, part)
            .text(Column::Group, group)
    }

    /// The cell of `column`.
    #[must_use]
    pub fn get(&self, column: Column) -> Option<&Cell> {
        self.cells.get(&column)
    }

    /// The columns the row has cells for, in their order.
    pub fn columns(&self) -> impl Iterator<Item = Column> + '_ {
        self.cells.keys().copied()
    }

    /// The row with `text` under `column`.
    #[must_use]
    pub fn text(mut self, column: Column, text: impl Into<String>) -> Self {
        _ = self.cells.insert(column, Cell::Text(text.into()));
        self
    }

    /// The row with `text` under `column`, if there is a text.
    #[must_use]
    pub fn maybe_text(self, column: Column, text: Option<impl Into<String>>) -> Self {
        match text {
            Some(text) => self.text(column, text),
            None => self,
        }
    }

    /// The row with `number` under `column`.
    #[must_use]
    pub fn number(mut self, column: Column, number: impl Into<i64>) -> Self {
        _ = self.cells.insert(column, Cell::Number(number.into()));
        self
    }

    /// The row with a count under [`Column::Count`].
    #[must_use]
    pub fn count(self, count: usize) -> Self {
        self.number(Column::Count, i64::try_from(count).unwrap_or(i64::MAX))
    }

    /// The row with `flag` under `column`.
    #[must_use]
    pub fn flag(mut self, column: Column, flag: bool) -> Self {
        _ = self.cells.insert(column, Cell::Flag(flag));
        self
    }

    /// The row with `duration` in its two columns: whole seconds and the
    /// readable duration, `1h 30m`.
    #[must_use]
    pub fn duration(self, length: Length, duration: SignedDuration) -> Self {
        let (seconds, readable) = length.columns();
        self.number(seconds, duration.as_secs())
            .text(readable, format_duration(duration))
    }

    /// The row with `time` under `column`, in RFC 9557 with its zone.
    #[must_use]
    pub fn time(self, column: Column, time: &ZonedDateTime) -> Self {
        self.text(column, time.to_string())
    }

    /// The row with `money`: the decimal under [`Column::Amount`], the
    /// currency's code under [`Column::Currency`].
    #[must_use]
    pub fn money(self, money: &Money) -> Self {
        self.text(Column::Amount, money.to_string())
            .text(Column::Currency, money.currency().code())
    }
}

impl Serialize for Row {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.cells.len()))?;
        for (column, cell) in &self.cells {
            map.serialize_entry(column.name(), cell)?;
        }
        map.end()
    }
}

/// The document of `kind`: what JSON writes, and what a template of the
/// report receives.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SchemaDocument<'rows> {
    version: u32,
    kind: &'static str,
    rows: &'rows [Row],
}

/// `rows` as the document of `kind`, in the schema's version.
#[must_use]
pub const fn schema_document(kind: SchemaKind, rows: &[Row]) -> SchemaDocument<'_> {
    SchemaDocument {
        version: SCHEMA_VERSION,
        kind: kind.name(),
        rows,
    }
}

/// `rows` as the JSON document of `kind`, indented.
///
/// # Errors
///
/// An internal error if serde can't write it, which is a bug.
pub fn schema_json(kind: SchemaKind, rows: &[Row]) -> PaceResult<String> {
    serde_json::to_string_pretty(&schema_document(kind, rows)).map_err(|error| {
        PaceError::with_source(ErrorKind::Internal, "cannot write JSON", error).ask_report()
    })
}

/// `rows` as the lines of the CSV of `kind`: the column names, then one
/// line per row, quoted as RFC 4180 says.
#[must_use]
pub fn schema_csv(kind: SchemaKind, rows: &[Row]) -> Vec<String> {
    let columns = kind.columns();
    let header = columns
        .iter()
        .map(|column| column.name())
        .collect::<Vec<_>>()
        .join(",");
    let lines = rows.iter().map(|row| {
        columns
            .iter()
            .map(|column| row.get(*column).map_or_else(String::new, Cell::csv_field))
            .collect::<Vec<_>>()
            .join(",")
    });
    std::iter::once(header).chain(lines).collect()
}

/// `text` as a CSV field: in double quotes, its quotes doubled, when it
/// holds a comma, a quote or a line break.
fn csv_quoted(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_field_with_comma_quote_or_line_break_is_quoted() {
        assert_eq!(csv_quoted("plain text"), "plain text");
        assert_eq!(csv_quoted("Bob, Alice"), "\"Bob, Alice\"");
        assert_eq!(csv_quoted(r#"say "hi""#), r#""say ""hi""""#);
        assert_eq!(csv_quoted("two\nlines"), "\"two\nlines\"");
    }

    #[test]
    fn csv_leaves_columns_without_cell_empty() {
        let rows = [Row::of("time", "tracked")
            .duration(Length::Net, SignedDuration::from_mins(90))
            .flag(Column::Running, false)];

        let lines = schema_csv(SchemaKind::Reflect, &rows);

        assert!(lines[0].starts_with("part,group,key,id,"), "{}", lines[0]);
        assert!(lines[0].ends_with(",running"), "{}", lines[0]);
        let cells: Vec<&str> = lines[1].split(',').collect();
        assert_eq!(cells.len(), REFLECT_COLUMNS.len());
        assert_eq!(cells[..3], ["time", "tracked", ""]);
        assert!(lines[1].contains(",5400,1h 30m,"), "{}", lines[1]);
        assert!(lines[1].ends_with(",false"), "{}", lines[1]);
    }

    #[test]
    fn json_row_has_keys_of_its_cells_in_column_order() {
        let row = Row::of("revenue", "total")
            .money(&Money::from_minor(9550, "EUR".parse().unwrap()))
            .count(2);

        let json = serde_json::to_string(&row).unwrap();

        assert_eq!(
            json,
            r#"{"part":"revenue","group":"total","count":2,"amount":"95.50","currency":"EUR"}"#
        );
    }

    #[test]
    fn json_document_names_version_and_kind() {
        let json = schema_json(SchemaKind::Reflect, &[]).unwrap();

        assert_eq!(
            json,
            "{\n  \"version\": 1,\n  \"kind\": \"reflect\",\n  \"rows\": []\n}"
        );
    }

    #[test]
    fn column_names_list_every_column_once_in_order() {
        assert_eq!(COLUMN_NAMES.len(), Column::Running as usize + 1);
        for (index, (column, _)) in COLUMN_NAMES.iter().enumerate() {
            assert_eq!(*column as usize, index, "{column:?}");
        }
    }

    #[test]
    fn every_length_pairs_seconds_with_readable_column() {
        let lengths = [
            Length::Net,
            Length::Target,
            Length::Budget,
            Length::Billed,
            Length::Held,
            Length::Paused,
        ];
        for length in lengths {
            let (seconds, readable) = length.columns();
            assert_eq!(seconds.name(), format!("{}_seconds", readable.name()));
        }
    }
}
