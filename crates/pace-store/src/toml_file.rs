//! pace's TOML files, read with spans so every error can point at its line.
//!
//! Config files, project manifests and event files share this reader.

use std::{
    fmt::{self, Display},
    ops::Range,
    path::Path,
};

use pace_error::{ErrorKind, PaceError, PaceResult, Snippet};
use toml::{
    Spanned,
    de::{DeInteger, DeString, DeTable, DeValue},
};

/// A TOML file and how to talk about it in errors.
pub struct TomlFile<'a> {
    /// The file, for messages and snippets.
    pub path: &'a Path,
    /// Its content.
    pub text: &'a str,
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
pub struct TomlCause(pub toml::de::Error);

impl Display for TomlCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.message().trim())
    }
}

impl std::error::Error for TomlCause {}

/// The entries of a table in the order they appear in the file.
pub type Entries<'t, 'i> = Vec<(&'t Spanned<DeString<'i>>, &'t Spanned<DeValue<'i>>)>;

/// The entries of `table`, by their position in the file.
#[must_use]
pub fn in_file_order<'t, 'i>(table: &'t DeTable<'i>) -> Entries<'t, 'i> {
    let mut entries: Entries<'t, 'i> = table.iter().collect();
    entries.sort_by_key(|(key, _)| key.span().start);
    entries
}

impl<'a> TomlFile<'a> {
    /// The file as a table with spans.
    ///
    /// # Errors
    ///
    /// If it is no valid TOML; the snippet points at the spot, and the TOML
    /// error is the cause.
    pub fn parse(&self) -> PaceResult<Spanned<DeTable<'a>>> {
        DeTable::parse(self.text).map_err(|error| {
            let span = error.span().unwrap_or(0..0);
            let label = error.message().trim().to_owned();
            PaceError::with_source(
                self.kind,
                "the file {file} is not valid TOML",
                TomlCause(error),
            )
            .attach_context("file", self.path.display().to_string())
            .attach_hint("fix the TOML at the marked spot")
            .attach_error_code(format!("{}::syntax", self.code))
            .attach_snippet(self.snippet(span, &label))
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
        let hint = closest(name, known.iter().copied()).map_or_else(
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
    pub fn check_version(&self, table: &DeTable<'_>) -> PaceResult<()> {
        let Some(value) = table.get("version") else {
            return Ok(());
        };
        let version = value.get_ref().as_integer().and_then(number);
        if version == Some(self.version) {
            return Ok(());
        }
        let hint = if version.is_some_and(|version| version > self.version) {
            "a newer pace wrote this file; update pace".to_owned()
        } else {
            format!("write `version = {}`", self.version)
        };
        let found = version.map_or_else(
            || with_article(value.get_ref().type_str()),
            |version| version.to_string(),
        );
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
    pub fn string<'v>(
        &self,
        name: &str,
        value: &'v Spanned<DeValue<'_>>,
        expects: &str,
    ) -> PaceResult<&'v str> {
        value.get_ref().as_str().ok_or_else(|| {
            self.wrong_type(
                name,
                value,
                expects,
                format!("write it as text, for example `{name} = \"…\"`"),
                &with_article(value.get_ref().type_str()),
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
    pub fn integer(
        &self,
        name: &str,
        value: &Spanned<DeValue<'_>>,
        expects: &str,
    ) -> PaceResult<i64> {
        let integer = value.get_ref().as_integer();
        integer.and_then(number).ok_or_else(|| {
            let label = if integer.is_some() {
                "too large a number".to_owned()
            } else {
                with_article(value.get_ref().type_str())
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

    /// A value of `name` that is not what it takes (`expects`).
    fn wrong_type(
        &self,
        name: &str,
        value: &Spanned<DeValue<'_>>,
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
    pub fn table<'v, 'i>(
        &self,
        name: &str,
        value: &'v Spanned<DeValue<'i>>,
    ) -> PaceResult<&'v DeTable<'i>> {
        value.get_ref().as_table().ok_or_else(|| {
            self.error_at(
                value.span(),
                "`{key}` is a table",
                [("key", name.to_owned())],
                format!("write it as `[{name}]` with its keys below"),
                &with_article(value.get_ref().type_str()),
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
pub fn closest<'a>(input: &str, names: impl Iterator<Item = &'a str>) -> Option<&'a str> {
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
        let table = file.parse()?;

        let error = file
            .check_version(table.get_ref())
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
        let table = file.parse()?;

        let error = file
            .check_version(table.get_ref())
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
        let table = file.parse()?;

        let error = file
            .check_version(table.get_ref())
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
        let table = file.parse()?;
        let value = table.get_ref().get("minutes").ok_or("the key is there")?;

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
        let table = file.parse()?;
        let value = table.get_ref().get("billing").ok_or("the key is there")?;

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
}
