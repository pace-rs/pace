//! pace's TOML files, read with spans so every error can point at its line.
//!
//! Config files, project manifests and event files share this reader.

use std::{ops::Range, path::Path};

use pace_error::{ErrorKind, PaceError, PaceResult, Snippet};
use toml::{
    Spanned,
    de::{DeString, DeTable, DeValue},
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
    /// The `version` this pace reads.
    pub version: i64,
}

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
    /// If it is no valid TOML; the snippet points at the spot.
    pub fn parse(&self) -> PaceResult<Spanned<DeTable<'a>>> {
        DeTable::parse(self.text).map_err(|error| {
            PaceError::new(ErrorKind::Configuration, "{file} is not valid TOML")
                .attach_context("file", self.path.display().to_string())
                .attach_hint("fix the TOML at the marked spot")
                .attach_error_code(format!("{}::syntax", self.code))
                .attach_snippet(self.snippet(error.span().unwrap_or(0..0), error.message().trim()))
        })
    }

    fn snippet(&self, span: Range<usize>, label: &str) -> Snippet {
        Snippet::new(self.path.display().to_string(), self.text, span, label)
    }

    /// An error about a spot in the file; the snippet shows file and line.
    #[must_use]
    pub fn error_at(
        &self,
        span: Range<usize>,
        message: impl Into<String>,
        hint: impl Into<String>,
        label: &str,
    ) -> Box<PaceError> {
        PaceError::new(ErrorKind::Configuration, message)
            .attach_hint(hint)
            .attach_snippet(self.snippet(span, label))
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
            format!("`{name}` is not a {} pace knows", self.noun),
            hint,
            "unknown key",
        )
        .attach_error_code(format!("{}::unknown-key", self.code))
    }

    /// Check the file's `version`.
    ///
    /// # Errors
    ///
    /// If it is not the version this pace reads; a newer one asks to update.
    pub fn check_version(&self, value: &Spanned<DeValue<'_>>) -> PaceResult<()> {
        let version = value
            .get_ref()
            .as_integer()
            .and_then(|integer| integer.as_str().parse::<i64>().ok());
        if version == Some(self.version) {
            return Ok(());
        }
        let hint = if version.is_some_and(|version| version > self.version) {
            "a newer pace wrote this file; update pace".to_owned()
        } else {
            format!("write `version = {}`", self.version)
        };
        Err(self
            .error_at(
                value.span(),
                format!("this pace reads version {} of {}", self.version, self.what),
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
            self.error_at(
                value.span(),
                format!("`{name}` takes {expects}"),
                format!("write it as text, for example `{name} = \"…\"`"),
                &with_article(value.get_ref().type_str()),
            )
            .attach_error_code(format!("{}::invalid-value", self.code))
        })
    }

    /// The value as a whole number, as written.
    ///
    /// # Errors
    ///
    /// If it is no integer; the error says what `name` takes (`expects`).
    pub fn integer<'v>(
        &self,
        name: &str,
        value: &'v Spanned<DeValue<'_>>,
        expects: &str,
    ) -> PaceResult<&'v str> {
        value
            .get_ref()
            .as_integer()
            .map(toml::de::DeInteger::as_str)
            .ok_or_else(|| {
                self.error_at(
                    value.span(),
                    format!("`{name}` takes {expects}"),
                    format!("write it as a number, for example `{name} = 9`"),
                    &with_article(value.get_ref().type_str()),
                )
                .attach_error_code(format!("{}::invalid-value", self.code))
            })
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
                format!("`{name}` is a table"),
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

fn with_article(noun: &str) -> String {
    let article = if noun.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    };
    format!("{article} {noun}")
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
