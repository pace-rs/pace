//! Settings and the layers they come from.
//!
//! A value comes from, in rising order: pace's default, the global file
//! `pace.global.toml`, the environment (`PACE_` + the key in capitals, `-`
//! as `_`, `__` between tables) and the command line, once a setting has a
//! flag. A key pace doesn't know stops pace with the file, the line and the
//! closest known key; an unknown `PACE_*` variable only warns.

use std::{
    ffi::OsString,
    fmt,
    ops::Range,
    path::{Path, PathBuf},
};

use jiff::civil::Weekday;
use pace_core::ProjectKey;
use pace_error::{ErrorKind, PaceError, PaceResult, Severity};
use toml::{
    Spanned,
    de::{DeString, DeValue},
};

use crate::{
    dirs::Located,
    origin::Origin,
    toml_file::{TomlFile, in_file_order},
};

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

/// The table that maps directories to projects.
const PROJECT_PATHS: &str = "project-paths";

/// Environment variables that are not settings.
const OTHER_VARIABLES: [&str; 4] = [
    "PACE_CONFIG",
    "PACE_HOME",
    "PACE_MACHINE_DIR",
    "PACE_PROJECT",
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
/// `home` expands a leading `~` in `[project-paths]`.
///
/// # Errors
///
/// If the global file was given but is missing, cannot be read, is no
/// valid TOML, has an unknown key or a value that doesn't fit; or if a
/// variable has a value that doesn't fit.
pub fn load(
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
            version: FILE_VERSION,
        };
        for (name, value, span) in file_values(&file, home, &mut project_paths)? {
            settings.set(name, &value).map_err(|invalid| {
                file.error_at(span.clone(), invalid.message, invalid.hint, "this value")
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
        let value = value.to_string_lossy();
        settings.set(key.name, &value).map_err(|invalid| {
            PaceError::new(ErrorKind::Configuration, "{variable}: {message}")
                .attach_context("variable", key.variable)
                .attach_context("message", invalid.message)
                .attach_hint(invalid.hint)
                .attach_error_code("pace::config::invalid-value")
        })?;
        set_origin(&mut origins, key.name, Origin::Env(key.variable));
    }

    Ok(Loaded {
        settings,
        origins,
        project_paths,
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
) -> PaceResult<Vec<(&'static str, String, Range<usize>)>> {
    let table = file.parse()?;
    let known: Vec<&str> = KEYS
        .iter()
        .map(|key| key.name)
        .chain([PROJECT_PATHS, "version"])
        .collect();

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
            let text = file.string(name, value, setting.expects)?;
            values.push((setting.name, text.to_owned(), value.span()));
        } else {
            return Err(file.unknown_key(name, key.span(), &known));
        }
    }
    Ok(values)
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
            file.error_at(
                project.span(),
                error.to_string(),
                error.hint().unwrap_or_default(),
                "this key",
            )
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
                format!("`{written}` is not an absolute path"),
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
