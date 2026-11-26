//! Settings and the layers they come from.
//!
//! A value comes from, in rising order: pace's default, the global file
//! `pace.global.toml`, the environment (`PACE_` + the key in capitals, `-`
//! as `_`, `__` between tables) and the command line, once a setting has a
//! flag. A key pace doesn't know stops pace with the file, the line and the
//! closest known key; an unknown `PACE_*` variable only warns.

use std::{ffi::OsString, fmt, ops::Range, path::Path};

use jiff::civil::Weekday;
use pace_error::{ErrorKind, PaceError, PaceResult, Severity, Snippet};
use toml::de::{DeTable, DeValue};

use crate::{dirs::Located, origin::Origin};

/// A setting pace knows, and the variable that sets it.
struct Key {
    name: &'static str,
    variable: &'static str,
    expects: &'static str,
}

const KEYS: [Key; 2] = [
    Key {
        name: "time-zone",
        variable: "PACE_TIME_ZONE",
        expects: "a time zone name like `Europe/Berlin`, or `system`",
    },
    Key {
        name: "week-start",
        variable: "PACE_WEEK_START",
        expects: "a day name like `monday`",
    },
];

/// Environment variables that are not settings.
const OTHER_VARIABLES: [&str; 3] = ["PACE_CONFIG", "PACE_HOME", "PACE_MACHINE_DIR"];

/// The version of `pace.global.toml` this pace reads and writes.
const FILE_VERSION: i64 = 1;

/// The time zone pace records and reports in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeZoneSetting {
    /// The zone of the operating system.
    System,
    /// A zone of the IANA time zone database.
    Named(String),
}

impl fmt::Display for TimeZoneSetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::System => f.write_str("system"),
            Self::Named(name) => f.write_str(name),
        }
    }
}

/// The effective settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// `time-zone`: the zone `--at` times are read in and reports use.
    pub time_zone: TimeZoneSetting,
    /// `week-start`: the first day of the week, for `this-week` and
    /// `last-week`.
    pub week_start: Weekday,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            time_zone: TimeZoneSetting::System,
            week_start: Weekday::Monday,
        }
    }
}

/// Why a value doesn't fit a key; becomes a [`PaceError`] with the place
/// the value came from.
struct Invalid {
    message: String,
    hint: String,
}

impl Settings {
    fn set(&mut self, key: &str, value: &str) -> Result<(), Invalid> {
        match key {
            "time-zone" => self.time_zone = parse_time_zone(value)?,
            "week-start" => self.week_start = parse_weekday(value)?,
            _ => {
                return Err(Invalid {
                    message: format!("`{key}` has no parser"),
                    hint: "this is a bug in pace".to_owned(),
                });
            }
        }
        Ok(())
    }

    fn get(&self, key: &str) -> String {
        match key {
            "time-zone" => self.time_zone.to_string(),
            "week-start" => weekday_name(self.week_start).to_owned(),
            _ => String::new(),
        }
    }
}

fn parse_time_zone(value: &str) -> Result<TimeZoneSetting, Invalid> {
    if value == "system" {
        return Ok(TimeZoneSetting::System);
    }
    if jiff::tz::db().get(value).is_ok() {
        return Ok(TimeZoneSetting::Named(value.to_owned()));
    }
    let names: Vec<String> = jiff::tz::db()
        .available()
        .map(|name| name.as_str().to_owned())
        .collect();
    Err(Invalid {
        message: format!("`{value}` is not a time zone pace knows"),
        hint: closest(value, names.iter().map(String::as_str)).map_or_else(
            || {
                "use a name from the IANA time zone database, like `Europe/Berlin`, or \
                 `system`"
                    .to_owned()
            },
            |name| format!("did you mean `{name}`?"),
        ),
    })
}

const WEEKDAYS: [(&str, Weekday); 7] = [
    ("monday", Weekday::Monday),
    ("tuesday", Weekday::Tuesday),
    ("wednesday", Weekday::Wednesday),
    ("thursday", Weekday::Thursday),
    ("friday", Weekday::Friday),
    ("saturday", Weekday::Saturday),
    ("sunday", Weekday::Sunday),
];

fn parse_weekday(value: &str) -> Result<Weekday, Invalid> {
    let lowercase = value.to_lowercase();
    WEEKDAYS
        .iter()
        .find(|(name, _)| *name == lowercase)
        .map(|(_, day)| *day)
        .ok_or_else(|| Invalid {
            message: format!("`{value}` is not a day of the week"),
            hint: closest(&lowercase, WEEKDAYS.iter().map(|(name, _)| *name)).map_or_else(
                || "use a day name like `monday`".to_owned(),
                |name| format!("did you mean `{name}`?"),
            ),
        })
}

fn weekday_name(day: Weekday) -> &'static str {
    WEEKDAYS
        .iter()
        .find(|(_, weekday)| *weekday == day)
        .map_or("monday", |(name, _)| name)
}

/// The known name closest to `input`, if any is close enough.
pub(crate) fn closest<'a>(input: &str, names: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    names
        .map(|name| (strsim::jaro_winkler(input, name), name))
        .filter(|(score, _)| *score > 0.8)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, name)| name)
}

/// The settings, where each value comes from, and warnings to show.
#[derive(Debug)]
pub struct Loaded {
    /// The effective settings.
    pub settings: Settings,
    origins: Vec<(&'static str, Origin)>,
    /// Problems that don't stop pace, such as an unknown `PACE_*` variable.
    pub warnings: Vec<PaceError>,
}

impl Loaded {
    /// Each setting as `(key, value, origin)`, in the order pace documents
    /// them.
    #[must_use]
    pub fn entries(&self) -> Vec<(&'static str, String, &Origin)> {
        self.origins
            .iter()
            .map(|(key, origin)| (*key, self.settings.get(key), origin))
            .collect()
    }
}

/// Load the settings: defaults, then the global file, then the environment.
///
/// # Errors
///
/// If the global file was given but is missing, cannot be read, is no
/// valid TOML, has an unknown key or a value that doesn't fit; or if a
/// variable has a value that doesn't fit.
pub fn load(
    config_file: &Located,
    env: impl Fn(&str) -> Option<OsString>,
    variables: impl Iterator<Item = OsString>,
) -> PaceResult<Loaded> {
    let mut settings = Settings::default();
    let mut origins: Vec<_> = KEYS.iter().map(|key| (key.name, Origin::Default)).collect();
    let mut set_origin = |name: &str, origin: Origin| {
        if let Some(entry) = origins.iter_mut().find(|(key, _)| *key == name) {
            entry.1 = origin;
        }
    };

    if let Some(text) = read_config_file(config_file)? {
        for value in file_values(&config_file.path, &text)? {
            settings.set(value.key, &value.text).map_err(|invalid| {
                error_at(
                    &config_file.path,
                    &text,
                    value.span.clone(),
                    invalid,
                    "this value",
                )
                .attach_error_code("pace::config::invalid-value")
            })?;
            set_origin(
                value.key,
                Origin::File {
                    path: config_file.path.clone(),
                    line: line_of(&text, value.span.start),
                },
            );
        }
    }

    for key in &KEYS {
        let Some(value) = env(key.variable).filter(|value| !value.is_empty()) else {
            continue;
        };
        let value = value.to_string_lossy();
        settings.set(key.name, &value).map_err(|invalid| {
            PaceError::new(ErrorKind::Configuration, "{variable}: {message}")
                .attach_context("variable", key.variable)
                .attach_context("message", invalid.message)
                .attach_hint(invalid.hint)
                .attach_error_code("pace::config::invalid-value")
        })?;
        set_origin(key.name, Origin::Env(key.variable));
    }

    Ok(Loaded {
        settings,
        origins,
        warnings: unknown_variables(variables),
    })
}

fn read_config_file(config_file: &Located) -> PaceResult<Option<String>> {
    match std::fs::read_to_string(&config_file.path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => match &config_file.origin {
            Origin::Flag(name) | Origin::Env(name) => Err(PaceError::new(
                ErrorKind::Configuration,
                "the config file {path} does not exist",
            )
            .attach_context("path", config_file.path.display().to_string())
            .attach_hint(format!(
                "create it, or drop {name} to use the default location"
            ))
            .attach_error_code("pace::config::missing-file")),
            _ => Ok(None),
        },
        Err(error) => Err(PaceError::with_source(
            ErrorKind::Configuration,
            "cannot read the config file {path}",
            error,
        )
        .attach_context("path", config_file.path.display().to_string())
        .attach_hint("check that pace may read the file")),
    }
}

/// A setting found in a file.
struct FileValue {
    key: &'static str,
    text: String,
    span: Range<usize>,
}

fn file_values(path: &Path, text: &str) -> PaceResult<Vec<FileValue>> {
    let table = DeTable::parse(text).map_err(|error| {
        PaceError::new(ErrorKind::Configuration, "{file} is not valid TOML")
            .attach_context("file", path.display().to_string())
            .attach_hint("fix the TOML at the marked spot")
            .attach_error_code("pace::config::syntax")
            .attach_snippet(Snippet::new(
                path.display().to_string(),
                text,
                error.span().unwrap_or(0..0),
                error.message().trim(),
            ))
    })?;

    let mut entries: Vec<_> = table.get_ref().iter().collect();
    entries.sort_by_key(|(key, _)| key.span().start);

    let mut values = Vec::new();
    for (key, value) in entries {
        let name = key.get_ref().as_ref();
        if name == "version" {
            check_version(path, text, value.get_ref(), value.span())?;
            continue;
        }
        let Some(known) = KEYS.iter().find(|known| known.name == name) else {
            return Err(unknown_key(path, text, name, key.span()));
        };
        let Some(string) = value.get_ref().as_str() else {
            return Err(error_at(
                path,
                text,
                value.span(),
                Invalid {
                    message: format!("`{name}` takes {}", known.expects),
                    hint: format!("write it as text, for example `{name} = \"…\"`"),
                },
                &with_article(value.get_ref().type_str()),
            )
            .attach_error_code("pace::config::invalid-value"));
        };
        values.push(FileValue {
            key: known.name,
            text: string.to_owned(),
            span: value.span(),
        });
    }
    Ok(values)
}

fn check_version(
    path: &Path,
    text: &str,
    value: &DeValue<'_>,
    span: Range<usize>,
) -> PaceResult<()> {
    let version = value
        .as_integer()
        .and_then(|integer| integer.as_str().parse::<i64>().ok());
    if version == Some(FILE_VERSION) {
        return Ok(());
    }
    Err(error_at(
        path,
        text,
        span,
        Invalid {
            message: format!("this pace reads version {FILE_VERSION} of the global config file"),
            hint: if version.is_some_and(|version| version > FILE_VERSION) {
                "a newer pace wrote this file; update pace".to_owned()
            } else {
                format!("write `version = {FILE_VERSION}`")
            },
        },
        "this version",
    )
    .attach_error_code("pace::config::version"))
}

fn unknown_key(path: &Path, text: &str, name: &str, span: Range<usize>) -> Box<PaceError> {
    let names = KEYS.iter().map(|key| key.name).chain(["version"]);
    let hint = closest(name, names).map_or_else(
        || {
            format!(
                "pace knows these settings: {}",
                KEYS.iter()
                    .map(|key| key.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
        |known| format!("did you mean `{known}`?"),
    );
    error_at(
        path,
        text,
        span,
        Invalid {
            message: format!("`{name}` is not a setting pace knows"),
            hint,
        },
        "unknown key",
    )
    .attach_error_code("pace::config::unknown-key")
}

/// An error about a spot in a file; the snippet shows the file and the line.
fn error_at(
    path: &Path,
    text: &str,
    span: Range<usize>,
    invalid: Invalid,
    label: &str,
) -> Box<PaceError> {
    PaceError::new(ErrorKind::Configuration, invalid.message)
        .attach_hint(invalid.hint)
        .attach_snippet(Snippet::new(path.display().to_string(), text, span, label))
}

fn with_article(noun: &str) -> String {
    let article = if noun.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    };
    format!("{article} {noun}")
}

fn line_of(text: &str, offset: usize) -> usize {
    text.get(..offset)
        .map_or(0, |before| before.matches('\n').count())
        + 1
}

fn unknown_variables(variables: impl Iterator<Item = OsString>) -> Vec<PaceError> {
    let known: Vec<&str> = KEYS
        .iter()
        .map(|key| key.variable)
        .chain(OTHER_VARIABLES)
        .collect();
    let mut unknown: Vec<String> = variables
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| name.starts_with("PACE_") && !known.contains(&name.as_str()))
        .collect();
    unknown.sort();
    unknown
        .into_iter()
        .map(|name| {
            let hint = closest(&name, known.iter().copied()).map_or_else(
                || "pace ignores it".to_owned(),
                |known| format!("did you mean {known}? pace ignores {name}"),
            );
            *PaceError::new(
                ErrorKind::Configuration,
                "the variable {name} is not one pace knows",
            )
            .attach_context("name", name)
            .attach_hint(hint)
            .attach_severity(Severity::Warning)
        })
        .collect()
}
