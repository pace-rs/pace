//! Settings and the layers they come from.
//!
//! A value comes from, in rising order: pace's default, the global file
//! `pace.global.toml`, the environment (`PACE_` + the key in capitals, `-`
//! as `_`, `__` between tables) and the command line, once a setting has a
//! flag. A key pace doesn't know stops pace with the file, the line and the
//! closest known key; an unknown `PACE_*` variable only warns.

use std::{
    borrow::Cow,
    ffi::OsString,
    fmt,
    ops::Range,
    path::{Path, PathBuf},
};

use jiff::civil::Weekday;
use pace_core::{HoldSource, ProjectKey};
use pace_error::{ErrorKind, PaceError, PaceResult, Severity};
use pace_store::Split;
use toml::{
    Spanned,
    de::{DeString, DeValue},
};

use pace_store::toml_file::{TomlFile, closest, in_file_order};

use crate::{dirs::Located, origin::Origin};

/// A setting pace knows, and the variable that sets it. A key in a table
/// is written `table.key`.
struct Key {
    name: &'static str,
    variable: &'static str,
    expects: &'static str,
    kind: Kind,
}

/// What a setting's value is in TOML.
#[derive(Clone, Copy)]
enum Kind {
    Text,
    Number,
}

const KEYS: [Key; 5] = [
    Key {
        name: "time-zone",
        variable: "PACE_TIME_ZONE",
        expects: "a time zone name like `Europe/Berlin`, or `system`",
        kind: Kind::Text,
    },
    Key {
        name: "week-start",
        variable: "PACE_WEEK_START",
        expects: "a day name like `monday`",
        kind: Kind::Text,
    },
    Key {
        name: "most-recent-count",
        variable: "PACE_MOST_RECENT_COUNT",
        expects: "a whole number from 1, like `9`",
        kind: Kind::Number,
    },
    Key {
        name: "holds.default-source",
        variable: "PACE_HOLDS__DEFAULT_SOURCE",
        expects: "`inner`, `outer` or `unknown`",
        kind: Kind::Text,
    },
    Key {
        name: "storage.split",
        variable: "PACE_STORAGE__SPLIT",
        expects: "`year`, `month`, `week` or `day`",
        kind: Kind::Text,
    },
];

/// The table that maps directories to projects.
const PROJECT_PATHS: &str = "project-paths";

/// Environment variables that are not settings.
const OTHER_VARIABLES: [&str; 5] = [
    "PACE_CONFIG",
    "PACE_HOME",
    "PACE_MACHINE_DIR",
    "PACE_PROJECT",
    crate::clock::TEST_NOW,
];

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
    /// `most-recent-count`: how many recent entries `begin --again` offers.
    pub most_recent_count: usize,
    /// `holds.default-source`: the source of a hold without `--inner` or
    /// `--outer`.
    pub default_hold_source: HoldSource,
    /// `storage.split`: how a new device splits its event files.
    pub split: Split,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            time_zone: TimeZoneSetting::System,
            week_start: Weekday::Monday,
            most_recent_count: 9,
            default_hold_source: HoldSource::Unknown,
            split: Split::Month,
        }
    }
}

/// A setting's value as read: text, or a whole number from a file.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    Text(String),
    Number(i64),
}

impl Value {
    /// The value as text; a number in decimal digits.
    fn text(&self) -> Cow<'_, str> {
        match self {
            Self::Text(text) => Cow::Borrowed(text),
            Self::Number(number) => Cow::Owned(number.to_string()),
        }
    }
}

/// A value that doesn't fit its setting: `guidance` names it as `{value}`.
/// The caller adds where the value came from.
fn invalid(guidance: &'static str, value: &str, hint: impl Into<String>) -> Box<PaceError> {
    PaceError::new(ErrorKind::Configuration, guidance)
        .attach_context("value", value)
        .attach_hint(hint)
}

impl Settings {
    fn set(&mut self, key: &str, value: &Value) -> PaceResult<()> {
        if key == "most-recent-count" {
            self.most_recent_count = parse_count(value)?;
            return Ok(());
        }
        let value = value.text();
        let value = value.as_ref();
        match key {
            "time-zone" => self.time_zone = parse_time_zone(value)?,
            "week-start" => self.week_start = parse_weekday(value)?,
            "holds.default-source" => self.default_hold_source = parse_hold_source(value)?,
            "storage.split" => self.split = value.parse()?,
            _ => {
                return Err(PaceError::new(
                    ErrorKind::Internal,
                    "the setting `{key}` has no parser",
                )
                .attach_context("key", key)
                .ask_report());
            }
        }
        Ok(())
    }

    fn get(&self, key: &str) -> toml::Value {
        let text = match key {
            "time-zone" => self.time_zone.to_string(),
            "week-start" => weekday_name(self.week_start).to_owned(),
            "most-recent-count" => {
                return toml::Value::Integer(
                    i64::try_from(self.most_recent_count).unwrap_or(i64::MAX),
                );
            }
            "holds.default-source" => hold_source_name(self.default_hold_source).to_owned(),
            "storage.split" => self.split.to_string(),
            _ => String::new(),
        };
        toml::Value::String(text)
    }
}

fn parse_count(value: &Value) -> PaceResult<usize> {
    match value {
        Value::Number(number) => usize::try_from(*number).ok(),
        Value::Text(text) => text.parse::<usize>().ok(),
    }
    .filter(|count| *count > 0)
    .ok_or_else(|| {
        invalid(
            "`{value}` is not a whole number from 1",
            &value.text(),
            "write a number like `9`",
        )
    })
}

const HOLD_SOURCES: [(&str, HoldSource); 3] = [
    ("inner", HoldSource::Inner),
    ("outer", HoldSource::Outer),
    ("unknown", HoldSource::Unknown),
];

fn parse_hold_source(value: &str) -> PaceResult<HoldSource> {
    HOLD_SOURCES
        .iter()
        .find(|(name, _)| *name == value)
        .map(|(_, source)| *source)
        .ok_or_else(|| {
            invalid(
                "`{value}` is not a source of holds",
                value,
                "use `inner` (you broke off), `outer` (someone interrupted) or `unknown`",
            )
        })
}

fn hold_source_name(source: HoldSource) -> &'static str {
    HOLD_SOURCES
        .iter()
        .find(|(_, known)| *known == source)
        .map_or("unknown", |(name, _)| name)
}

fn parse_time_zone(value: &str) -> PaceResult<TimeZoneSetting> {
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
    let hint = closest(value, names.iter().map(String::as_str)).map_or_else(
        || {
            "use a name from the IANA time zone database, like `Europe/Berlin`, or `system`"
                .to_owned()
        },
        |name| format!("did you mean `{name}`?"),
    );
    Err(invalid(
        "`{value}` is not a time zone pace knows",
        value,
        hint,
    ))
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

fn parse_weekday(value: &str) -> PaceResult<Weekday> {
    let lowercase = value.to_lowercase();
    WEEKDAYS
        .iter()
        .find(|(name, _)| *name == lowercase)
        .map(|(_, day)| *day)
        .ok_or_else(|| {
            let hint = closest(&lowercase, WEEKDAYS.iter().map(|(name, _)| *name)).map_or_else(
                || "use a day name like `monday`".to_owned(),
                |name| format!("did you mean `{name}`?"),
            );
            invalid("`{value}` is not a day of the week", value, hint)
        })
}

fn weekday_name(day: Weekday) -> &'static str {
    WEEKDAYS
        .iter()
        .find(|(_, weekday)| *weekday == day)
        .map_or("monday", |(name, _)| name)
}

/// A directory mapped to a project in `[project-paths]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPath {
    /// The directory as written, for example `~/work/client-repo`.
    pub written: String,
    /// The directory, with `~` expanded.
    pub dir: PathBuf,
    /// The project.
    pub key: ProjectKey,
    /// The line it was written on.
    pub origin: Origin,
}

/// The settings, where each value comes from, and warnings to show.
#[derive(Debug)]
pub struct Loaded {
    /// The effective settings.
    pub settings: Settings,
    origins: Vec<(&'static str, Origin)>,
    /// `[project-paths]`, in file order.
    pub project_paths: Vec<ProjectPath>,
    /// Problems that don't stop pace, such as an unknown `PACE_*` variable.
    pub warnings: Vec<PaceError>,
}

impl Loaded {
    /// Each setting as `(key, value, origin)`, in the order pace documents
    /// them; a key in a table is written `table.key`.
    #[must_use]
    pub fn entries(&self) -> Vec<(&'static str, toml::Value, &Origin)> {
        self.origins
            .iter()
            .map(|(key, origin)| (*key, self.settings.get(key), origin))
            .collect()
    }
}

/// Load the settings: defaults, then the global file, then the environment.
/// `home` expands a leading `~` in `[project-paths]`.
///
/// # Errors
///
/// If the global file was given but is missing, cannot be read, is no
/// valid TOML, has an unknown key or a value that doesn't fit; or if a
/// variable has a value that doesn't fit.
pub fn load_settings(
    config_file: &Located,
    home: Option<&Path>,
    env: impl Fn(&str) -> Option<OsString>,
    variables: impl Iterator<Item = OsString>,
) -> PaceResult<Loaded> {
    let mut settings = Settings::default();
    let mut origins: Vec<_> = KEYS.iter().map(|key| (key.name, Origin::Default)).collect();
    let mut project_paths = Vec::new();

    if let Some(text) = read_config_file(config_file)? {
        let file = TomlFile {
            path: &config_file.path,
            text: &text,
            what: "the global config file",
            noun: "setting",
            code: "pace::config",
            kind: ErrorKind::Configuration,
            version: FILE_VERSION,
        };
        for (name, value, span) in file_values(&file, home, &mut project_paths)? {
            settings.set(name, &value).map_err(|error| {
                file.point_at(error, span.clone(), "this value")
                    .attach_error_code("pace::config::invalid-value")
            })?;
            set_origin(
                &mut origins,
                name,
                Origin::File {
                    path: config_file.path.clone(),
                    line: file.line_of(span.start),
                },
            );
        }
    }

    for key in &KEYS {
        let Some(value) = env(key.variable).filter(|value| !value.is_empty()) else {
            continue;
        };
        let value = Value::Text(value.to_string_lossy().into_owned());
        settings
            .set(key.name, &value)
            .map_err(|error| from_variable(error, key.variable))?;
        set_origin(&mut origins, key.name, Origin::Env(key.variable));
    }

    Ok(Loaded {
        settings,
        origins,
        project_paths,
        warnings: unknown_variables(variables),
    })
}

/// `error`, about the value of the variable `variable`, with a first line
/// that names the variable.
fn from_variable(error: Box<PaceError>, variable: &str) -> Box<PaceError> {
    error
        .prepend_guidance_line("the setting given by {variable} is not valid")
        .attach_context("variable", variable)
        .attach_error_code("pace::config::invalid-value")
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

fn set_origin(origins: &mut [(&'static str, Origin)], name: &str, origin: Origin) {
    if let Some(entry) = origins.iter_mut().find(|(key, _)| *key == name) {
        entry.1 = origin;
    }
}

/// The scalar settings of the file as `(key, value, span)`; the entries of
/// `[project-paths]` go into `project_paths`.
fn file_values(
    file: &TomlFile<'_>,
    home: Option<&Path>,
    project_paths: &mut Vec<ProjectPath>,
) -> PaceResult<Vec<(&'static str, Value, Range<usize>)>> {
    let table = file.parse()?;
    let mut known: Vec<&str> = Vec::new();
    for key in &KEYS {
        let top = key
            .name
            .split_once('.')
            .map_or(key.name, |(table, _)| table);
        if !known.contains(&top) {
            known.push(top);
        }
    }
    known.extend([PROJECT_PATHS, "version"]);

    let mut values = Vec::new();
    for (key, value) in in_file_order(table.get_ref()) {
        let name = key.get_ref().as_ref();
        if name == "version" {
            file.check_version(value)?;
        } else if name == PROJECT_PATHS {
            for (dir, project) in in_file_order(file.table(name, value)?) {
                project_paths.push(project_path(file, home, dir, project)?);
            }
        } else if let Some(setting) = KEYS.iter().find(|setting| setting.name == name) {
            values.push((
                setting.name,
                scalar(file, name, setting, value)?,
                value.span(),
            ));
        } else if let Some(in_table) = table_keys(name) {
            for (key, value) in in_file_order(file.table(name, value)?) {
                let inner = key.get_ref().as_ref();
                let Some(setting) = KEYS.iter().find(|setting| {
                    setting
                        .name
                        .strip_prefix(name)
                        .and_then(|rest| rest.strip_prefix('.'))
                        == Some(inner)
                }) else {
                    return Err(file.unknown_key(inner, key.span(), &in_table));
                };
                values.push((
                    setting.name,
                    scalar(file, inner, setting, value)?,
                    value.span(),
                ));
            }
        } else {
            return Err(file.unknown_key(name, key.span(), &known));
        }
    }
    Ok(values)
}

/// The keys of the settings table `table`, if pace knows the table.
fn table_keys(table: &str) -> Option<Vec<&'static str>> {
    let keys: Vec<&str> = KEYS
        .iter()
        .filter_map(|key| key.name.strip_prefix(table)?.strip_prefix('.'))
        .collect();
    (!keys.is_empty()).then_some(keys)
}

/// A setting's value, checked for its TOML type; `name` is the key as
/// written.
fn scalar(
    file: &TomlFile<'_>,
    name: &str,
    setting: &Key,
    value: &Spanned<DeValue<'_>>,
) -> PaceResult<Value> {
    Ok(match setting.kind {
        Kind::Text => Value::Text(file.string(name, value, setting.expects)?.to_owned()),
        Kind::Number => Value::Number(file.integer(name, value, setting.expects)?),
    })
}

fn project_path(
    file: &TomlFile<'_>,
    home: Option<&Path>,
    dir: &Spanned<DeString<'_>>,
    project: &Spanned<DeValue<'_>>,
) -> PaceResult<ProjectPath> {
    let written = dir.get_ref().as_ref();
    let key = file
        .string(written, project, "a project key like `acme-webshop`")?
        .parse::<ProjectKey>()
        .map_err(|error| {
            file.point_at(error, project.span(), "this key")
                .attach_error_code("pace::config::invalid-value")
        })?;

    let relative_to_home = written
        .strip_prefix("~/")
        .or_else(|| written.strip_prefix("~\\"))
        .or_else(|| (written == "~").then_some(""));
    let expanded = match (relative_to_home, home) {
        (Some(rest), Some(home)) => home.join(rest),
        (Some(_), None) => {
            return Err(file.error_at(
                dir.span(),
                "pace cannot find your home directory to expand `~`",
                [],
                "write the full path",
                "this path",
            ));
        }
        (None, _) => PathBuf::from(written),
    };
    if !expanded.is_absolute() {
        return Err(file
            .error_at(
                dir.span(),
                "`{path}` is not an absolute path",
                [("path", written.to_owned())],
                "write the full path, or start it with `~/`",
                "relative path",
            )
            .attach_error_code("pace::config::invalid-value"));
    }

    Ok(ProjectPath {
        written: written.to_owned(),
        dir: expanded,
        key,
        origin: Origin::File {
            path: file.path.to_path_buf(),
            line: file.line_of(dir.span().start),
        },
    })
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
