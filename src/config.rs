//! Settings and the layers they come from.
//!
//! A value comes from, in rising order: pace's default, the global file
//! `pace.global.toml`, the environment (`PACE_` + the key in capitals, `-`
//! as `_`, `__` between tables) and the command line, once a setting has a
//! flag. A key pace doesn't know stops pace with the file, the line and the
//! closest known key; an unknown `PACE_*` variable only warns.

mod key;
mod values;

use std::{
    ffi::OsString,
    ops::Range,
    path::{Path, PathBuf},
};

use jiff::civil::Weekday;
use pace_core::{HoldSource, ProjectKey};
use pace_error::{ErrorKind, PaceError, PaceResult, Severity, closest_name};
use pace_store::Split;

use pace_store::toml_file::{TomlFile, TomlKey, TomlValue, read_hint};

pub(crate) use self::key::Key;
pub use self::values::TimeZoneSetting;
use self::{
    key::Kind,
    values::{
        Value, hold_source_name, parse_count, parse_hold_source, parse_time_zone, parse_weekday,
        weekday_name,
    },
};
use crate::{dirs::Located, origin::Origin};

/// The table that maps directories to projects.
pub(crate) const PROJECT_PATHS: &str = "project-paths";

/// The start of every variable pace reads.
const PREFIX: &str = "PACE_";

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
    /// `reports.time-zone`: the zone reports split days in.
    pub reports_time_zone: TimeZoneSetting,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            time_zone: TimeZoneSetting::System,
            week_start: Weekday::Monday,
            most_recent_count: 9,
            default_hold_source: HoldSource::Unknown,
            split: Split::Month,
            reports_time_zone: TimeZoneSetting::System,
        }
    }
}

impl Settings {
    fn set(&mut self, key: Key, value: &Value) -> PaceResult<()> {
        match key {
            Key::TimeZone => self.time_zone = parse_time_zone(&value.text())?,
            Key::WeekStart => self.week_start = parse_weekday(&value.text())?,
            Key::MostRecentCount => self.most_recent_count = parse_count(value)?,
            Key::HoldsDefaultSource => self.default_hold_source = parse_hold_source(&value.text())?,
            Key::StorageSplit => self.split = value.text().parse()?,
            Key::ReportsTimeZone => self.reports_time_zone = parse_time_zone(&value.text())?,
        }
        Ok(())
    }

    fn get(&self, key: Key) -> toml::Value {
        let text = match key {
            Key::TimeZone => self.time_zone.to_string(),
            Key::WeekStart => weekday_name(self.week_start).to_owned(),
            Key::MostRecentCount => {
                return toml::Value::Integer(
                    i64::try_from(self.most_recent_count).unwrap_or(i64::MAX),
                );
            }
            Key::HoldsDefaultSource => hold_source_name(self.default_hold_source).to_owned(),
            Key::StorageSplit => self.split.to_string(),
            Key::ReportsTimeZone => self.reports_time_zone.to_string(),
        };
        toml::Value::String(text)
    }
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
    /// The file it was written in.
    pub file: PathBuf,
    /// The line it was written on, counted from 1.
    pub line: usize,
}

impl ProjectPath {
    /// Where the mapping was written, as an origin.
    #[must_use]
    pub fn origin(&self) -> Origin {
        Origin::File {
            path: self.file.clone(),
            line: self.line,
        }
    }
}

/// The settings, where each value comes from, and warnings to show.
#[derive(Debug)]
pub struct Loaded {
    /// The effective settings.
    pub settings: Settings,
    origins: Vec<(Key, Origin)>,
    /// `[project-paths]`, in file order.
    pub project_paths: Vec<ProjectPath>,
    /// Problems that don't stop pace, such as an unknown `PACE_*` variable.
    pub warnings: Vec<PaceError>,
    /// The settings the global file sets, with their values as the file
    /// gives them, before the environment.
    file_values: Vec<(Key, toml::Value)>,
}

impl Loaded {
    /// Each setting as `(key, value, origin)`, in the order pace documents
    /// them; a key in a table is written `table.key`.
    #[must_use]
    pub fn entries(&self) -> Vec<(&'static str, toml::Value, &Origin)> {
        self.origins
            .iter()
            .map(|(key, origin)| (key.name(), self.settings.get(*key), origin))
            .collect()
    }

    /// The value of the setting `key` and where it comes from.
    pub(crate) fn entry(&self, key: Key) -> (toml::Value, Origin) {
        let origin = self
            .origins
            .iter()
            .find(|(known, _)| *known == key)
            .map_or(Origin::Default, |(_, origin)| origin.clone());
        (self.settings.get(key), origin)
    }

    /// The value the global file gives the setting `key`, if it sets it.
    pub(crate) fn file_value(&self, key: Key) -> Option<&toml::Value> {
        self.file_values
            .iter()
            .find(|(known, _)| *known == key)
            .map(|(_, value)| value)
    }

    /// The value the global file gives the setting `key`, else pace's
    /// default, before the environment: what `pace setup` keeps on Enter.
    pub(crate) fn file_or_default(&self, key: Key) -> toml::Value {
        self.file_value(key)
            .cloned()
            .unwrap_or_else(|| Settings::default().get(key))
    }
}

/// `text` as a value of the setting `key`, checked as the global file's
/// reader checks it, and written as pace writes it: `europe/vienna` is
/// `Europe/Vienna`, `Sunday` is `sunday`.
///
/// # Errors
///
/// If `text` doesn't fit the setting: the error's hint names what fits.
pub(crate) fn setting_value(key: Key, text: &str) -> PaceResult<toml::Value> {
    let mut settings = Settings::default();
    settings.set(key, &Value::Text(text.to_owned()))?;
    Ok(settings.get(key))
}

/// Load the settings: defaults, then the global file, then the environment.
///
/// `home` expands a leading `~` in `[project-paths]`. Without a
/// `config_file`, pace reads no file: `config set` creates a file that
/// isn't there yet, even one that `--config` or `PACE_CONFIG` names.
///
/// # Errors
///
/// If the global file was given but is missing, cannot be read, is no
/// valid TOML, has an unknown key or a value that doesn't fit; or if a
/// variable has a value that doesn't fit.
pub fn load_settings(
    config_file: Option<&Located>,
    home: Option<&Path>,
    env: impl Fn(&str) -> Option<OsString>,
    variables: impl Iterator<Item = OsString>,
) -> PaceResult<Loaded> {
    let mut settings = Settings::default();
    let mut origins: Vec<_> = Key::ALL.map(|key| (key, Origin::Default)).to_vec();
    let mut project_paths = Vec::new();
    let mut from_file = Vec::new();

    if let Some((config_file, text)) = read_config_file(config_file)? {
        let file = TomlFile {
            path: &config_file.path,
            text: &text,
            what: "the global config file",
            noun: "setting",
            code: "pace::config",
            kind: ErrorKind::Configuration,
            version: FILE_VERSION,
        };
        for (key, value, span) in file_values(&file, home, &mut project_paths)? {
            settings.set(key, &value).map_err(|error| {
                file.point_at(error, span.clone(), "this value")
                    .attach_error_code("pace::config::invalid-value")
            })?;
            from_file.push((key, settings.get(key)));
            set_origin(
                &mut origins,
                key,
                Origin::File {
                    path: config_file.path.clone(),
                    line: file.line_of(span.start),
                },
            );
        }
    }

    for key in Key::ALL {
        let Some(value) = env(key.variable()).filter(|value| !value.is_empty()) else {
            continue;
        };
        let value = Value::Text(value.to_string_lossy().into_owned());
        settings
            .set(key, &value)
            .map_err(|error| from_variable(error, key.variable()))?;
        set_origin(&mut origins, key, Origin::Env(key.variable()));
    }

    Ok(Loaded {
        settings,
        origins,
        project_paths,
        warnings: unknown_variables(variables),
        file_values: from_file,
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

/// The global file and its text; `None` without a file, or when the file
/// pace looks for by default isn't there.
fn read_config_file(config_file: Option<&Located>) -> PaceResult<Option<(&Located, String)>> {
    let Some(config_file) = config_file else {
        return Ok(None);
    };
    match std::fs::read_to_string(&config_file.path) {
        Ok(text) => Ok(Some((config_file, text))),
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
        Err(error) => {
            let hint = read_hint(&error);
            Err(PaceError::with_source(
                ErrorKind::Configuration,
                "cannot read the config file {path}",
                error,
            )
            .attach_context("path", config_file.path.display().to_string())
            .attach_hint(hint))
        }
    }
}

fn set_origin(origins: &mut [(Key, Origin)], key: Key, origin: Origin) {
    if let Some(entry) = origins.iter_mut().find(|(known, _)| *known == key) {
        entry.1 = origin;
    }
}

/// The scalar settings of the file as `(key, value, span)`; the entries of
/// `[project-paths]` go into `project_paths`.
fn file_values(
    file: &TomlFile<'_>,
    home: Option<&Path>,
    project_paths: &mut Vec<ProjectPath>,
) -> PaceResult<Vec<(Key, Value, Range<usize>)>> {
    let document = file.parse()?;
    let mut known: Vec<&str> = Vec::new();
    for key in Key::ALL {
        let name = key.name();
        let top = name.split_once('.').map_or(name, |(table, _)| table);
        if !known.contains(&top) {
            known.push(top);
        }
    }
    known.extend([PROJECT_PATHS, "version"]);

    let table = document.table();
    file.check_version(table)?;
    let mut values = Vec::new();
    for (key, value) in table.entries() {
        let name = key.name();
        if name == "version" {
            // checked above
        } else if name == PROJECT_PATHS {
            for (dir, project) in file.table(name, value)?.entries() {
                project_paths.push(project_path(file, home, dir, project)?);
            }
        } else if let Some(setting) = Key::ALL.into_iter().find(|setting| setting.name() == name) {
            values.push((setting, scalar(file, name, setting, value)?, value.span()));
        } else if let Some(in_table) = table_keys(name) {
            for (key, value) in file.table(name, value)?.entries() {
                let inner = key.name();
                let Some(setting) = Key::ALL.into_iter().find(|setting| {
                    setting
                        .name()
                        .strip_prefix(name)
                        .and_then(|rest| rest.strip_prefix('.'))
                        == Some(inner)
                }) else {
                    return Err(file.unknown_key(inner, key.span(), &in_table));
                };
                values.push((setting, scalar(file, inner, setting, value)?, value.span()));
            }
        } else {
            return Err(file.unknown_key(name, key.span(), &known));
        }
    }
    Ok(values)
}

/// The keys of the settings table `table`, if pace knows the table.
fn table_keys(table: &str) -> Option<Vec<&'static str>> {
    let keys: Vec<&str> = Key::ALL
        .into_iter()
        .filter_map(|key| key.name().strip_prefix(table)?.strip_prefix('.'))
        .collect();
    (!keys.is_empty()).then_some(keys)
}

/// A setting's value, checked for its TOML type; `name` is the key as
/// written.
fn scalar(
    file: &TomlFile<'_>,
    name: &str,
    setting: Key,
    value: TomlValue<'_>,
) -> PaceResult<Value> {
    Ok(match setting.kind() {
        Kind::Text => Value::Text(file.string(name, value, setting.expects())?.to_owned()),
        Kind::Number => Value::Number(file.integer(name, value, setting.expects())?),
    })
}

fn project_path(
    file: &TomlFile<'_>,
    home: Option<&Path>,
    dir: TomlKey<'_>,
    project: TomlValue<'_>,
) -> PaceResult<ProjectPath> {
    let written = dir.name();
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
        file: file.path.to_path_buf(),
        line: file.line_of(dir.span().start),
    })
}

fn unknown_variables(variables: impl Iterator<Item = OsString>) -> Vec<PaceError> {
    let known: Vec<&str> = Key::ALL
        .into_iter()
        .map(Key::variable)
        .chain(OTHER_VARIABLES)
        .collect();
    let mut unknown: Vec<String> = variables
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| name.starts_with(PREFIX) && !known.contains(&name.as_str()))
        .collect();
    unknown.sort();
    unknown
        .into_iter()
        .map(|name| {
            // Scored without the shared `PACE_`, which would make every
            // pair of names look close.
            let without_prefix = |name: &'static str| name.strip_prefix(PREFIX).unwrap_or(name);
            let hint = closest_name(
                name.strip_prefix(PREFIX).unwrap_or(&name),
                known.iter().copied().map(without_prefix),
            )
            .map_or_else(
                || "pace ignores it".to_owned(),
                |known| format!("did you mean {PREFIX}{known}? pace ignores {name}"),
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

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn every_setting_reads_back_what_it_shows() -> TestResult {
        let settings = Settings {
            time_zone: parse_time_zone("Asia/Tokyo")?,
            week_start: Weekday::Sunday,
            most_recent_count: 3,
            default_hold_source: HoldSource::Outer,
            split: Split::Week,
            reports_time_zone: parse_time_zone("Europe/Lisbon")?,
        };
        let mut read_back = Settings::default();

        for key in Key::ALL {
            let shown = match settings.get(key) {
                toml::Value::Integer(number) => Value::Number(number),
                toml::Value::String(text) => Value::Text(text),
                other => return Err(format!("{} shows {other}", key.name()).into()),
            };
            read_back.set(key, &shown)?;
        }

        assert_eq!(read_back, settings);
        Ok(())
    }

    #[test]
    fn unknown_variable_far_from_every_known_one_gets_no_suggestion() {
        let warnings = unknown_variables(["PACE_DEBUG".into()].into_iter());

        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].hint(), Some("pace ignores it"));
    }

    #[test]
    fn unknown_variable_close_to_known_one_suggests_it() {
        let warnings = unknown_variables(["PACE_TIMEZONE".into()].into_iter());

        assert_eq!(
            warnings[0].hint(),
            Some("did you mean PACE_TIME_ZONE? pace ignores PACE_TIMEZONE")
        );
    }

    #[test]
    fn file_or_default_takes_file_before_default_and_ignores_environment() -> TestResult {
        let dir = pace_store::testing::scratch_dir()?;
        let path = dir.path().join("pace.global.toml");
        std::fs::write(&path, "week-start = \"sunday\"\n")?;
        let config_file = Located {
            path,
            origin: Origin::Default,
        };
        let env = |name: &str| (name == "PACE_WEEK_START").then(|| "friday".into());

        let loaded = load_settings(Some(&config_file), None, env, std::iter::empty())?;

        assert_eq!(loaded.settings.week_start, Weekday::Friday);
        assert_eq!(
            loaded.file_or_default(Key::WeekStart),
            toml::Value::String("sunday".to_owned())
        );
        assert_eq!(
            loaded.file_or_default(Key::TimeZone),
            toml::Value::String("system".to_owned())
        );
        Ok(())
    }

    #[test]
    fn project_path_at_home_without_home_directory_is_refused() -> TestResult {
        let dir = pace_store::testing::scratch_dir()?;
        let path = dir.path().join("pace.global.toml");
        std::fs::write(&path, "[project-paths]\n\"~/repo\" = \"client-repo\"\n")?;
        let config_file = Located {
            path,
            origin: Origin::Default,
        };

        let error = load_settings(Some(&config_file), None, |_| None, std::iter::empty()).err();

        assert_eq!(
            error.as_ref().map(ToString::to_string).as_deref(),
            Some("pace cannot find your home directory to expand `~`")
        );
        assert_eq!(
            error.as_ref().and_then(|error| error.hint()),
            Some("write the full path")
        );
        Ok(())
    }
}
