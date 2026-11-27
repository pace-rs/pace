//! pace's TOML files, read with spans so every error can point at its line.
//!
//! Config files, project manifests and event files share this reader.

use std::{
    fmt::{self, Display},
    ops::Range,
    path::Path,
};

use jiff::civil::Date;
use pace_error::{ErrorKind, PaceError, PaceResult, Snippet};
use toml::{
    Spanned,
    de::{DeInteger, DeString, DeTable, DeValue},
};

/// A TOML file and how to talk about it in errors.
#[derive(Debug)]
pub struct TomlFile<'file> {
    /// The file, for messages and snippets.
    pub path: &'file Path,
    /// Its content.
    pub text: &'file str,
    /// For messages: `the global config file`, `the project manifest`.
    pub what: &'static str,
    /// What its keys are called: `setting`, `key`.
    pub noun: &'static str,
    /// The prefix of error codes: `pace::config`, `pace::project`.
    pub code: &'static str,
    /// What its errors are related to: the configuration, the project.
    pub kind: ErrorKind,
    /// The `version` this pace reads.
    pub version: i64,
}

/// The TOML error behind a file that is no valid TOML. It shows only its
/// message: its own rendering repeats the snippet the error already shows.
#[derive(Debug)]
pub(crate) struct TomlCause(pub(crate) String);

impl Display for TomlCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TomlCause {}

/// The error for `path` with the content `text`, which is no valid TOML:
/// the TOML parser's `message` about `span`. Its code is `{code}::syntax`.
pub(crate) fn not_valid_toml(
    kind: ErrorKind,
    code: &str,
    path: &Path,
    text: &str,
    span: Range<usize>,
    message: &str,
) -> Box<PaceError> {
    let message = message.trim();
    PaceError::with_source(
        kind,
        "the file {file} is not valid TOML",
        TomlCause(message.to_owned()),
    )
    .attach_context("file", path.display().to_string())
    .attach_hint("fix the TOML at the marked spot")
    .attach_error_code(format!("{code}::syntax"))
    .attach_snippet(Snippet::new(
        path.display().to_string(),
        text,
        span,
        message,
    ))
}

/// A TOML file as [`TomlFile::parse`] read it: its top table, with the
/// spans of its keys and values.
#[derive(Debug)]
pub struct TomlDocument<'text>(Spanned<DeTable<'text>>);

impl TomlDocument<'_> {
    /// The file's top table.
    #[must_use]
    pub fn table(&self) -> TomlTable<'_> {
        TomlTable(self.0.get_ref())
    }
}

/// A table of a parsed TOML file.
#[derive(Debug, Clone, Copy)]
pub struct TomlTable<'file>(&'file DeTable<'file>);

impl<'file> TomlTable<'file> {
    /// The table's keys and values, in the order they stand in the file.
    #[must_use]
    pub fn entries(self) -> Vec<(TomlKey<'file>, TomlValue<'file>)> {
        let mut entries: Vec<_> = self
            .0
            .iter()
            .map(|(key, value)| (TomlKey(key), TomlValue(value)))
            .collect();
        entries.sort_by_key(|(key, _)| key.span().start);
        entries
    }

    /// The value of `name`, if the table holds it.
    #[must_use]
    pub fn get(self, name: &str) -> Option<TomlValue<'file>> {
        self.0.get(name).map(TomlValue)
    }
}

/// A key of a TOML table, and where it stands in the file.
#[derive(Debug, Clone, Copy)]
pub struct TomlKey<'file>(&'file Spanned<DeString<'file>>);

impl<'file> TomlKey<'file> {
    /// The key as written, without quotes.
    #[must_use]
    pub fn name(self) -> &'file str {
        self.0.get_ref()
    }

    /// Where the key stands in the file.
    #[must_use]
    pub fn span(self) -> Range<usize> {
        self.0.span()
    }
}

/// A value in a TOML file, and where it stands. [`TomlFile`] reads it as
/// text, a number or a table.
#[derive(Debug, Clone, Copy)]
pub struct TomlValue<'file>(&'file Spanned<DeValue<'file>>);

impl<'file> TomlValue<'file> {
    /// Where the value stands in the file.
    #[must_use]
    pub fn span(self) -> Range<usize> {
        self.0.span()
    }

    /// The tables of an array of tables, such as `[[event]]`; none for any
    /// other value.
    #[must_use]
    pub fn tables(self) -> Vec<TomlTable<'file>> {
        self.0.get_ref().as_array().map_or_else(Vec::new, |items| {
            items
                .iter()
                .filter_map(|item| item.get_ref().as_table())
                .map(TomlTable)
                .collect()
        })
    }

    /// The value's TOML type with its article: `a string`, `an integer`.
    fn type_with_article(self) -> String {
        with_article(self.0.get_ref().type_str())
    }
}

impl<'file> TomlFile<'file> {
    /// The file as a table with spans.
    ///
    /// # Errors
    ///
    /// If it is no valid TOML; the snippet points at the spot, and the TOML
    /// error is the cause.
    pub fn parse(&self) -> PaceResult<TomlDocument<'file>> {
        DeTable::parse(self.text)
            .map(TomlDocument)
            .map_err(|error| {
                not_valid_toml(
                    self.kind,
                    self.code,
                    self.path,
                    self.text,
                    error.span().unwrap_or(0..0),
                    error.message(),
                )
            })
    }

    fn snippet(&self, span: Range<usize>, label: &str) -> Snippet {
        Snippet::new(self.path.display().to_string(), self.text, span, label)
    }

    /// An error about a spot in the file: the `guidance` with its
    /// `{placeholders}` filled from `context`, the hint, and a snippet that
    /// shows file and line with `label` at `span`.
    #[must_use]
    pub fn error_at(
        &self,
        span: Range<usize>,
        guidance: &'static str,
        context: impl IntoIterator<Item = (&'static str, String)>,
        hint: impl Into<String>,
        label: &str,
    ) -> Box<PaceError> {
        let error = context.into_iter().fold(
            PaceError::new(self.kind, guidance),
            |error, (key, value)| error.attach_context(key, value),
        );
        self.point_at(error.attach_hint(hint), span, label)
    }

    /// `error`, pointed at `span` of the file with `label`.
    #[must_use]
    pub fn point_at(
        &self,
        error: Box<PaceError>,
        span: Range<usize>,
        label: &str,
    ) -> Box<PaceError> {
        error.attach_snippet(self.snippet(span, label))
    }

    /// An unknown key, with the closest of `known` or the list of them.
    #[must_use]
    pub fn unknown_key(&self, name: &str, span: Range<usize>, known: &[&str]) -> Box<PaceError> {
        let hint = closest_name(name, known.iter().copied()).map_or_else(
            || format!("pace knows these {}s here: {}", self.noun, known.join(", ")),
            |known| format!("did you mean `{known}`?"),
        );
        self.error_at(
            span,
            "`{key}` is not a {noun} pace knows",
            [("key", name.to_owned()), ("noun", self.noun.to_owned())],
            hint,
            "unknown key",
        )
        .attach_error_code(format!("{}::unknown-key", self.code))
    }

    /// Check the `version` of the file's table `table`, if it has one.
    /// Call it before the other keys: a newer file may hold keys this pace
    /// doesn't know.
    ///
    /// # Errors
    ///
    /// If it is not the version this pace reads; a newer one asks to update.
    pub fn check_version(&self, table: TomlTable<'_>) -> PaceResult<()> {
        let Some(value) = table.get("version") else {
            return Ok(());
        };
        let version = value.0.get_ref().as_integer().and_then(number);
        if version == Some(self.version) {
            return Ok(());
        }
        let hint = if version.is_some_and(|version| version > self.version) {
            "a newer pace wrote this file; update pace".to_owned()
        } else {
            format!("write `version = {}`", self.version)
        };
        let found =
            version.map_or_else(|| value.type_with_article(), |version| version.to_string());
        Err(self
            .error_at(
                value.span(),
                "this pace reads version {reads} of {what}, not {found}",
                [
                    ("reads", self.version.to_string()),
                    ("what", self.what.to_owned()),
                    ("found", found),
                ],
                hint,
                "this version",
            )
            .attach_error_code(format!("{}::version", self.code)))
    }

    /// The value as text.
    ///
    /// # Errors
    ///
    /// If it is no text; the error says what `name` takes (`expects`).
    pub fn string<'value>(
        &self,
        name: &str,
        value: TomlValue<'value>,
        expects: &str,
    ) -> PaceResult<&'value str> {
        value.0.get_ref().as_str().ok_or_else(|| {
            self.wrong_type(
                name,
                value,
                expects,
                format!("write it as text, for example `{name} = \"…\"`"),
                &value.type_with_article(),
            )
        })
    }

    /// The value as a whole number, in the base it is written in: `0x10`,
    /// `0o20`, `0b1_0000` and `16` are all 16.
    ///
    /// # Errors
    ///
    /// If it is no integer, or one too large; the error says what `name`
    /// takes (`expects`).
    pub fn integer(&self, name: &str, value: TomlValue<'_>, expects: &str) -> PaceResult<i64> {
        let integer = value.0.get_ref().as_integer();
        integer.and_then(number).ok_or_else(|| {
            let label = if integer.is_some() {
                "too large a number".to_owned()
            } else {
                value.type_with_article()
            };
            self.wrong_type(
                name,
                value,
                expects,
                format!("write it as a number, for example `{name} = 9`"),
                &label,
            )
        })
    }

    /// The text of a decimal number as written, such as `95` or `95.50`,
    /// for a reader that must not round it through a float. TOML has dropped
    /// the underscores between its digits.
    ///
    /// # Errors
    ///
    /// If it is no number, or an integer in another base than ten; the error
    /// says what `name` takes (`expects`).
    pub fn decimal_text<'value>(
        &self,
        name: &str,
        value: TomlValue<'value>,
        expects: &str,
    ) -> PaceResult<&'value str> {
        let text = match value.0.get_ref() {
            DeValue::Integer(integer) if integer.radix() == 10 => Some(integer.as_str()),
            DeValue::Float(float) => Some(float.as_str()),
            _ => None,
        };
        text.ok_or_else(|| {
            let label = if value.0.get_ref().is_integer() {
                "a number in another base".to_owned()
            } else {
                value.type_with_article()
            };
            self.wrong_type(
                name,
                value,
                expects,
                format!("write it as a decimal number, for example `{name} = 95.50`"),
                &label,
            )
        })
    }

    /// The value as `true` or `false`.
    ///
    /// # Errors
    ///
    /// If it is no boolean.
    pub fn boolean(&self, name: &str, value: TomlValue<'_>) -> PaceResult<bool> {
        value.0.get_ref().as_bool().ok_or_else(|| {
            self.wrong_type(
                name,
                value,
                "`true` or `false`",
                format!("write `{name} = true` or `{name} = false`, without quotes"),
                &value.type_with_article(),
            )
        })
    }

    /// The value as a day: a TOML local date such as `2026-09-01`, without
    /// quotes.
    ///
    /// # Errors
    ///
    /// If it is no local date: text, a date with a time, or a day that
    /// doesn't exist.
    pub fn date(&self, name: &str, value: TomlValue<'_>) -> PaceResult<Date> {
        let datetime = value.0.get_ref().as_datetime();
        let day = datetime
            .filter(|datetime| datetime.time.is_none() && datetime.offset.is_none())
            .and_then(|datetime| datetime.date)
            .and_then(|day| {
                let year = i16::try_from(day.year).ok()?;
                let month = i8::try_from(day.month).ok()?;
                let day = i8::try_from(day.day).ok()?;
                Date::new(year, month, day).ok()
            });
        day.ok_or_else(|| {
            let label = if datetime.is_some() {
                "a date with a time".to_owned()
            } else {
                value.type_with_article()
            };
            self.wrong_type(
                name,
                value,
                "a day such as 2026-09-01",
                format!("write the day without quotes, for example `{name} = 2026-09-01`"),
                &label,
            )
        })
    }

    /// A value of `name` that is not what it takes (`expects`).
    fn wrong_type(
        &self,
        name: &str,
        value: TomlValue<'_>,
        expects: &str,
        hint: String,
        label: &str,
    ) -> Box<PaceError> {
        self.error_at(
            value.span(),
            "`{key}` takes {expects}",
            [("key", name.to_owned()), ("expects", expects.to_owned())],
            hint,
            label,
        )
        .attach_error_code(format!("{}::invalid-value", self.code))
    }

    /// The value as a table.
    ///
    /// # Errors
    ///
    /// If it is no table.
    pub fn table<'value>(
        &self,
        name: &str,
        value: TomlValue<'value>,
    ) -> PaceResult<TomlTable<'value>> {
        value.0.get_ref().as_table().map(TomlTable).ok_or_else(|| {
            self.error_at(
                value.span(),
                "`{key}` is a table",
                [("key", name.to_owned())],
                format!("write it as `[{name}]` with its keys below"),
                &value.type_with_article(),
            )
            .attach_error_code(format!("{}::invalid-value", self.code))
        })
    }

    /// The tables of a list of tables, such as `[[acme.rates]]`, each with
    /// where it stands in the file.
    ///
    /// # Errors
    ///
    /// If it is no list, or a list that holds something else than tables.
    pub fn tables<'value>(
        &self,
        name: &str,
        value: TomlValue<'value>,
    ) -> PaceResult<Vec<(TomlTable<'value>, Range<usize>)>> {
        let items = value.0.get_ref().as_array();
        let tables = items.and_then(|items| {
            items
                .iter()
                .map(|item| Some((TomlTable(item.get_ref().as_table()?), item.span())))
                .collect::<Option<Vec<_>>>()
        });
        tables.ok_or_else(|| {
            self.error_at(
                value.span(),
                "`{key}` is a list of tables",
                [("key", name.to_owned())],
                format!("write each as `[[{name}]]` with its keys below"),
                &value.type_with_article(),
            )
            .attach_error_code(format!("{}::invalid-value", self.code))
        })
    }

    /// The line of a byte offset, counted from 1.
    #[must_use]
    pub fn line_of(&self, offset: usize) -> usize {
        self.text
            .get(..offset)
            .map_or(0, |before| before.matches('\n').count())
            + 1
    }
}

/// The number a TOML integer stands for, if it fits an `i64`.
fn number(integer: &DeInteger<'_>) -> Option<i64> {
    i64::from_str_radix(integer.as_str(), integer.radix()).ok()
}

fn with_article(noun: &str) -> String {
    let article = if noun.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    };
    format!("{article} {noun}")
}

/// How to go on when pace cannot read a TOML file: a file that is no UTF-8
/// needs another encoding, any other failure points at the permissions.
#[must_use]
pub fn read_hint(error: &std::io::Error) -> &'static str {
    if error.kind() == std::io::ErrorKind::InvalidData {
        "save the file as UTF-8"
    } else {
        "check that pace may read the file"
    }
}

/// The known name closest to `input`, if any is close enough (Jaro-Winkler
/// similarity above 0.8).
pub fn closest_name<'names>(
    input: &str,
    names: impl Iterator<Item = &'names str>,
) -> Option<&'names str> {
    names
        .map(|name| (strsim::jaro_winkler(input, name), name))
        .filter(|(score, _)| *score > 0.8)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, name)| name)
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn manifest(text: &str) -> TomlFile<'_> {
        TomlFile {
            path: Path::new("pace.toml"),
            text,
            what: "the project manifest",
            noun: "key",
            code: "pace::project",
            kind: ErrorKind::Project,
            version: 1,
        }
    }

    #[test]
    fn text_that_is_no_utf8_asks_for_utf8_and_other_read_errors_for_permission() {
        let encoding = std::io::Error::new(std::io::ErrorKind::InvalidData, "not UTF-8");
        let permission = std::io::Error::from(std::io::ErrorKind::PermissionDenied);

        assert_eq!(read_hint(&encoding), "save the file as UTF-8");
        assert_eq!(read_hint(&permission), "check that pace may read the file");
    }

    #[test]
    fn entries_come_in_file_order_with_their_spans() -> TestResult {
        let file = manifest("b = 1\na = \"x\"\n");
        let document = file.parse()?;

        let entries = document.table().entries();

        let names: Vec<&str> = entries.iter().map(|(key, _)| key.name()).collect();
        assert_eq!(names, ["b", "a"]);
        let (key, value) = entries.get(1).ok_or("two entries")?;
        assert_eq!(key.span(), 6..7);
        assert_eq!(value.span(), 10..13);
        assert_eq!(file.string("a", *value, "text")?, "x");
        Ok(())
    }

    #[test]
    fn tables_of_array_are_tables_and_other_values_have_none() -> TestResult {
        let file = manifest("[[event]]\nkind = 1\n[[event]]\nnote = 2\n");
        let document = file.parse()?;
        let table = document.table();

        let events = table.get("event").ok_or("the key is there")?.tables();
        let keys: Vec<&str> = events
            .iter()
            .flat_map(|event| event.entries())
            .map(|(key, _)| key.name())
            .collect();

        assert_eq!(keys, ["kind", "note"]);
        let kind = events.first().and_then(|event| event.get("kind"));
        assert!(kind.ok_or("kind is there")?.tables().is_empty());
        Ok(())
    }

    #[test]
    fn broken_toml_keeps_its_cause_and_the_kind_of_the_file() -> TestResult {
        let error = manifest("key = ")
            .parse()
            .err()
            .ok_or("broken TOML fails")?;

        assert_eq!(error.kind(), ErrorKind::Project);
        assert_eq!(error.context_value("file"), Some("pace.toml"));
        let source = std::error::Error::source(&*error).ok_or("the TOML error is the cause")?;
        assert!(
            source.to_string().contains("string values must be quoted"),
            "the cause is the TOML error: {source:?}"
        );
        Ok(())
    }

    #[test]
    fn error_at_fills_its_guidance_from_the_context() {
        let error = manifest("key = 1\n").error_at(
            6..7,
            "`{name}` takes text",
            [("name", "key".to_owned())],
            "write it as text",
            "an integer",
        );

        assert_eq!(error.to_string(), "`key` takes text");
        assert_eq!(error.context_value("name"), Some("key"));
        assert_eq!(error.kind(), ErrorKind::Project);
    }

    #[test]
    fn unknown_key_names_the_key_in_its_context() {
        let error = manifest("nmae = 1\n").unknown_key("nmae", 0..4, &["name"]);

        assert_eq!(error.to_string(), "`nmae` is not a key pace knows");
        assert_eq!(error.context_value("key"), Some("nmae"));
        assert_eq!(error.hint(), Some("did you mean `name`?"));
    }

    #[test]
    fn version_too_new_names_both_versions() -> TestResult {
        let file = manifest("version = 2\n");
        let document = file.parse()?;

        let error = file
            .check_version(document.table())
            .err()
            .ok_or("version 2 is refused")?;

        assert_eq!(
            error.to_string(),
            "this pace reads version 1 of the project manifest, not 2"
        );
        assert_eq!(error.context_value("found"), Some("2"));
        Ok(())
    }

    /// The label of the error's snippet.
    fn label(error: &PaceError) -> Option<String> {
        miette::Diagnostic::labels(error)?
            .next()?
            .label()
            .map(str::to_owned)
    }

    #[test]
    fn version_too_old_asks_to_write_version() -> TestResult {
        let file = manifest("version = 0\n");
        let document = file.parse()?;

        let error = file
            .check_version(document.table())
            .err()
            .ok_or("version 0 is refused")?;

        assert_eq!(
            error.to_string(),
            "this pace reads version 1 of the project manifest, not 0"
        );
        assert_eq!(error.hint(), Some("write `version = 1`"));
        Ok(())
    }

    #[test]
    fn version_as_text_names_its_type() -> TestResult {
        let file = manifest("version = \"1\"\n");
        let document = file.parse()?;

        let error = file
            .check_version(document.table())
            .err()
            .ok_or("a version as text is refused")?;

        assert_eq!(
            error.to_string(),
            "this pace reads version 1 of the project manifest, not a string"
        );
        assert_eq!(error.hint(), Some("write `version = 1`"));
        Ok(())
    }

    #[test]
    fn integer_beyond_i64_is_refused_as_too_large() -> TestResult {
        let file = manifest("minutes = 0xffff_ffff_ffff_ffff\n");
        let document = file.parse()?;
        let value = document.table().get("minutes").ok_or("the key is there")?;

        let error = file
            .integer("minutes", value, "a number of minutes")
            .err()
            .ok_or("too large a number is refused")?;

        assert_eq!(error.to_string(), "`minutes` takes a number of minutes");
        assert_eq!(label(&error).as_deref(), Some("too large a number"));
        assert_eq!(
            error.hint(),
            Some("write it as a number, for example `minutes = 9`")
        );
        Ok(())
    }

    #[test]
    fn table_written_as_value_says_how_to_write_it() -> TestResult {
        let file = manifest("billing = 1\n");
        let document = file.parse()?;
        let value = document.table().get("billing").ok_or("the key is there")?;

        let error = file
            .table("billing", value)
            .err()
            .ok_or("a number is no table")?;

        assert_eq!(error.to_string(), "`billing` is a table");
        assert_eq!(label(&error).as_deref(), Some("an integer"));
        assert_eq!(
            error.hint(),
            Some("write it as `[billing]` with its keys below")
        );
        assert!(error.is_code("pace::project::invalid-value"));
        Ok(())
    }

    #[test]
    fn decimal_text_keeps_written_digits_of_integer_and_float() -> TestResult {
        let file = manifest("whole = 95\nfraction = 95.50\n");
        let document = file.parse()?;
        let table = document.table();

        let whole = table.get("whole").ok_or("the key is there")?;
        let fraction = table.get("fraction").ok_or("the key is there")?;

        assert_eq!(file.decimal_text("whole", whole, "an amount")?, "95");
        assert_eq!(
            file.decimal_text("fraction", fraction, "an amount")?,
            "95.50"
        );
        Ok(())
    }

    #[test]
    fn decimal_text_refuses_hexadecimal_and_text() -> TestResult {
        let file = manifest("hex = 0x5f\ntext = \"95\"\n");
        let document = file.parse()?;
        let table = document.table();

        for name in ["hex", "text"] {
            let value = table.get(name).ok_or("the key is there")?;
            let error = file
                .decimal_text(name, value, "an amount")
                .err()
                .ok_or("no decimal number")?;
            assert_eq!(error.to_string(), format!("`{name}` takes an amount"));
            assert!(error.is_code("pace::project::invalid-value"));
        }
        Ok(())
    }

    #[test]
    fn boolean_reads_true_and_refuses_text() -> TestResult {
        let file = manifest("yes = true\nno = \"no\"\n");
        let document = file.parse()?;
        let table = document.table();

        let yes = table.get("yes").ok_or("the key is there")?;
        let no = table.get("no").ok_or("the key is there")?;

        assert!(file.boolean("yes", yes)?);
        let error = file.boolean("no", no).err().ok_or("text is no boolean")?;
        assert_eq!(error.to_string(), "`no` takes `true` or `false`");
        assert_eq!(label(&error).as_deref(), Some("a string"));
        Ok(())
    }

    #[test]
    fn date_reads_local_date_and_refuses_date_with_time() -> TestResult {
        let file = manifest("day = 2026-09-01\nwhen = 2026-09-01T09:00:00\n");
        let document = file.parse()?;
        let table = document.table();

        let day = table.get("day").ok_or("the key is there")?;
        let when = table.get("when").ok_or("the key is there")?;

        assert_eq!(file.date("day", day)?, jiff::civil::date(2026, 9, 1));
        let error = file.date("when", when).err().ok_or("a time is no day")?;
        assert_eq!(error.to_string(), "`when` takes a day such as 2026-09-01");
        assert_eq!(label(&error).as_deref(), Some("a date with a time"));
        Ok(())
    }

    #[test]
    fn tables_of_value_that_is_no_array_of_tables_says_how_to_write_it() -> TestResult {
        let file = manifest("rates = 5\n[[event]]\nkind = 1\n");
        let document = file.parse()?;
        let table = document.table();

        let events = table.get("event").ok_or("the key is there")?;
        let rates = table.get("rates").ok_or("the key is there")?;

        assert_eq!(file.tables("event", events)?.len(), 1);
        let error = file.tables("rates", rates).err().ok_or("5 is no list")?;
        assert_eq!(error.to_string(), "`rates` is a list of tables");
        assert_eq!(
            error.hint(),
            Some("write each as `[[rates]]` with its keys below")
        );
        Ok(())
    }
}
