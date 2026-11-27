//! Reading and showing the values of settings: time zones, days of the
//! week, counts, sources of holds and directories.

use std::{
    borrow::Cow,
    fmt,
    path::{Path, PathBuf},
};

use jiff::{civil::Weekday, tz::TimeZone};
use pace_core::HoldSource;
use pace_error::{ErrorKind, PaceError, PaceResult, closest_name};

/// The time zone pace records and reports in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeZoneSetting {
    /// The zone of the operating system, looked up when pace needs it.
    System,
    /// A zone of the IANA time zone database; it always has its IANA name.
    Named(TimeZone),
}

impl fmt::Display for TimeZoneSetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::System => f.write_str("system"),
            // `parse_time_zone` takes only zones with a name; a zone
            // without one is what jiff calls `Etc/Unknown`.
            Self::Named(zone) => f.write_str(zone.iana_name().unwrap_or("Etc/Unknown")),
        }
    }
}

/// A setting's value as read: text, or a whole number from a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    /// Text, from a file or a variable.
    Text(String),
    /// A whole number from a file.
    Number(i64),
}

impl Value {
    /// The value as text; a number in decimal digits.
    pub(crate) fn text(&self) -> Cow<'_, str> {
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

pub(crate) fn parse_count(value: &Value) -> PaceResult<usize> {
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

pub(crate) fn parse_hold_source(value: &str) -> PaceResult<HoldSource> {
    match value.to_lowercase().as_str() {
        "inner" => Ok(HoldSource::Inner),
        "outer" => Ok(HoldSource::Outer),
        "unknown" => Ok(HoldSource::Unknown),
        _ => Err(invalid(
            "`{value}` is not a source of holds",
            value,
            "use `inner` (you broke off), `outer` (someone interrupted) or `unknown`",
        )),
    }
}

pub(crate) const fn hold_source_name(source: HoldSource) -> &'static str {
    match source {
        HoldSource::Inner => "inner",
        HoldSource::Outer => "outer",
        HoldSource::Unknown => "unknown",
    }
}

pub(crate) fn parse_time_zone(value: &str) -> PaceResult<TimeZoneSetting> {
    if value.eq_ignore_ascii_case("system") {
        return Ok(TimeZoneSetting::System);
    }
    if let Some(zone) = jiff::tz::db()
        .get(value)
        .ok()
        .filter(|zone| zone.iana_name().is_some())
    {
        return Ok(TimeZoneSetting::Named(zone));
    }
    let names: Vec<String> = jiff::tz::db()
        .available()
        .map(|name| name.as_str().to_owned())
        .collect();
    let hint = closest_name(value, names.iter().map(String::as_str)).map_or_else(
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

/// The directory of `reports.templates`: `None` for empty text, else a
/// full path or one that starts with `~`, which
/// [`expand_home`] expands.
pub(crate) fn parse_templates_dir(value: &str) -> PaceResult<Option<PathBuf>> {
    if value.is_empty() {
        return Ok(None);
    }
    if Path::new(value).is_absolute() || home_relative(value).is_some() {
        return Ok(Some(PathBuf::from(value)));
    }
    Err(invalid(
        "`{value}` is not a full path",
        value,
        "write the full path, or start it with `~/`",
    ))
}

/// `dir` with a leading `~` replaced by `home`.
///
/// # Errors
///
/// If `dir` starts with `~` and pace knows no home directory.
pub(crate) fn expand_home(dir: PathBuf, home: Option<&Path>) -> PaceResult<PathBuf> {
    let Some(rest) = dir.to_str().and_then(home_relative) else {
        return Ok(dir);
    };
    home.map(|home| home.join(rest)).ok_or_else(|| {
        invalid(
            "pace cannot find your home directory to expand `~` in `{value}`",
            &dir.display().to_string(),
            "write the full path",
        )
    })
}

/// The rest of `path` after a leading `~/`, `~\` or a lone `~`.
pub(crate) fn home_relative(path: &str) -> Option<&str> {
    path.strip_prefix("~/")
        .or_else(|| path.strip_prefix("~\\"))
        .or_else(|| (path == "~").then_some(""))
}

const WEEKDAYS: [Weekday; 7] = [
    Weekday::Monday,
    Weekday::Tuesday,
    Weekday::Wednesday,
    Weekday::Thursday,
    Weekday::Friday,
    Weekday::Saturday,
    Weekday::Sunday,
];

pub(crate) fn parse_weekday(value: &str) -> PaceResult<Weekday> {
    let lowercase = value.to_lowercase();
    WEEKDAYS
        .into_iter()
        .find(|day| weekday_name(*day) == lowercase)
        .ok_or_else(|| {
            let hint = closest_name(&lowercase, WEEKDAYS.into_iter().map(weekday_name))
                .map_or_else(
                    || "use a day name like `monday`".to_owned(),
                    |name| format!("did you mean `{name}`?"),
                );
            invalid("`{value}` is not a day of the week", value, hint)
        })
}

pub(crate) const fn weekday_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Monday => "monday",
        Weekday::Tuesday => "tuesday",
        Weekday::Wednesday => "wednesday",
        Weekday::Thursday => "thursday",
        Weekday::Friday => "friday",
        Weekday::Saturday => "saturday",
        Weekday::Sunday => "sunday",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn time_zone_setting_holds_zone_under_its_canonical_name() -> TestResult {
        let setting = parse_time_zone("america/NEW_YORK")?;

        assert_eq!(
            setting,
            TimeZoneSetting::Named(TimeZone::get("America/New_York")?)
        );
        assert_eq!(setting.to_string(), "America/New_York");
        Ok(())
    }

    #[test]
    fn system_keyword_of_time_zone_ignores_case() -> TestResult {
        assert_eq!(parse_time_zone("System")?, TimeZoneSetting::System);
        assert_eq!(parse_time_zone("SYSTEM")?, TimeZoneSetting::System);
        Ok(())
    }

    #[test]
    fn unknown_zone_of_jiff_is_no_time_zone_setting() {
        assert!(parse_time_zone("Etc/Unknown").is_err());
    }

    #[test]
    fn hold_source_reads_each_source_and_shows_it_back() -> TestResult {
        for (written, source) in [
            ("inner", HoldSource::Inner),
            ("outer", HoldSource::Outer),
            ("unknown", HoldSource::Unknown),
        ] {
            assert_eq!(parse_hold_source(written)?, source);
            assert_eq!(hold_source_name(source), written);
        }
        Ok(())
    }

    #[test]
    fn hold_source_refuses_other_word_and_names_the_three() {
        let error = parse_hold_source("sideways").err();

        assert_eq!(
            error.as_ref().map(ToString::to_string).as_deref(),
            Some("`sideways` is not a source of holds")
        );
        assert_eq!(
            error.as_ref().and_then(|error| error.hint()),
            Some("use `inner` (you broke off), `outer` (someone interrupted) or `unknown`")
        );
    }

    #[test]
    fn time_zone_far_from_every_name_points_to_the_database() {
        let error = parse_time_zone("Qqqqqqqqqqqqqqqq").err();

        assert_eq!(
            error.as_ref().map(ToString::to_string).as_deref(),
            Some("`Qqqqqqqqqqqqqqqq` is not a time zone pace knows")
        );
        assert_eq!(
            error.as_ref().and_then(|error| error.hint()),
            Some("use a name from the IANA time zone database, like `Europe/Berlin`, or `system`")
        );
    }

    #[test]
    fn weekday_close_to_day_name_suggests_it() {
        let error = parse_weekday("Mondy").err();

        assert_eq!(
            error.as_ref().map(ToString::to_string).as_deref(),
            Some("`Mondy` is not a day of the week")
        );
        assert_eq!(
            error.as_ref().and_then(|error| error.hint()),
            Some("did you mean `monday`?")
        );
    }

    #[test]
    fn templates_dir_is_none_when_empty_and_takes_full_or_home_path() -> TestResult {
        let full = std::env::temp_dir().join("templates");

        assert_eq!(parse_templates_dir("")?, None);
        assert_eq!(
            parse_templates_dir(&full.display().to_string())?,
            Some(full.clone())
        );
        assert_eq!(
            parse_templates_dir("~/templates")?,
            Some(PathBuf::from("~/templates"))
        );
        Ok(())
    }

    #[test]
    fn relative_templates_dir_is_refused() {
        let error = parse_templates_dir("templates").err();

        assert_eq!(
            error.as_ref().map(ToString::to_string).as_deref(),
            Some("`templates` is not a full path")
        );
        assert_eq!(
            error.as_ref().and_then(|error| error.hint()),
            Some("write the full path, or start it with `~/`")
        );
    }

    #[test]
    fn home_expands_leading_tilde_only() -> TestResult {
        let home = std::env::temp_dir().join("home");
        let full = std::env::temp_dir().join("templates");

        assert_eq!(
            expand_home(PathBuf::from("~/templates"), Some(&home))?,
            home.join("templates")
        );
        assert_eq!(expand_home(PathBuf::from("~"), Some(&home))?, home);
        assert_eq!(expand_home(full.clone(), None)?, full);
        assert!(expand_home(PathBuf::from("~/templates"), None).is_err());
        Ok(())
    }
}
