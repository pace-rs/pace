//! The settings pace knows: their keys, their variables and the values
//! that fit.

use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult, closest_name};

/// A setting pace knows. A key in a table is written `table.key`.
///
/// Only [`Key::ALL`] builds a key, so a key left out of it is dead code,
/// which the workspace refuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Key {
    TimeZone,
    WeekStart,
    MostRecentCount,
    HoldsDefaultSource,
    StorageSplit,
    ReportsTimeZone,
    ReportsTemplates,
}

/// What a setting's value is in TOML.
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Text,
    Number,
}

impl Key {
    /// Every setting, in the order pace documents them.
    pub(crate) const ALL: [Self; 7] = [
        Self::TimeZone,
        Self::WeekStart,
        Self::MostRecentCount,
        Self::HoldsDefaultSource,
        Self::StorageSplit,
        Self::ReportsTimeZone,
        Self::ReportsTemplates,
    ];

    /// The setting written `name`, as `config get` and `config set` take
    /// it: `time-zone`, `holds.default-source`.
    ///
    /// # Errors
    ///
    /// If pace knows no setting `name`: the error names the closest key
    /// and exits with 2.
    pub(crate) fn named(name: &str) -> PaceResult<Self> {
        Self::ALL
            .into_iter()
            .find(|key| key.name() == name)
            .ok_or_else(|| {
                let hint = closest_name(name, Self::ALL.into_iter().map(Self::name)).map_or_else(
                    || "`pace config show` lists the settings".to_owned(),
                    |closest| format!("did you mean `{closest}`?"),
                );
                PaceError::new(
                    ErrorKind::InvalidInput,
                    "pace doesn't know the setting `{key}`",
                )
                .attach_context("key", name)
                .attach_hint(hint)
                .attach_exit_code(ExitCode::Usage)
            })
    }

    /// The key as written in the file.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::TimeZone => "time-zone",
            Self::WeekStart => "week-start",
            Self::MostRecentCount => "most-recent-count",
            Self::HoldsDefaultSource => "holds.default-source",
            Self::StorageSplit => "storage.split",
            Self::ReportsTimeZone => "reports.time-zone",
            Self::ReportsTemplates => "reports.templates",
        }
    }

    /// The environment variable that sets it.
    pub(crate) const fn variable(self) -> &'static str {
        match self {
            Self::TimeZone => "PACE_TIME_ZONE",
            Self::WeekStart => "PACE_WEEK_START",
            Self::MostRecentCount => "PACE_MOST_RECENT_COUNT",
            Self::HoldsDefaultSource => "PACE_HOLDS__DEFAULT_SOURCE",
            Self::StorageSplit => "PACE_STORAGE__SPLIT",
            Self::ReportsTimeZone => "PACE_REPORTS__TIME_ZONE",
            Self::ReportsTemplates => "PACE_REPORTS__TEMPLATES",
        }
    }

    /// The values that fit, for an error about the TOML type.
    pub(crate) const fn expects(self) -> &'static str {
        match self {
            Self::TimeZone | Self::ReportsTimeZone => {
                "a time zone name like `Europe/Berlin`, or `system`"
            }
            Self::WeekStart => "a day name like `monday`",
            Self::MostRecentCount => "a whole number from 1, like `9`",
            Self::HoldsDefaultSource => "`inner`, `outer` or `unknown`",
            Self::StorageSplit => "`year`, `month`, `week` or `day`",
            Self::ReportsTemplates => {
                "a full path, one that starts with `~/`, or empty for pace's own templates"
            }
        }
    }

    pub(crate) const fn kind(self) -> Kind {
        match self {
            Self::MostRecentCount => Kind::Number,
            Self::TimeZone
            | Self::WeekStart
            | Self::HoldsDefaultSource
            | Self::StorageSplit
            | Self::ReportsTimeZone
            | Self::ReportsTemplates => Kind::Text,
        }
    }
}
