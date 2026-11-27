//! The settings pace knows: their keys, their variables and the values
//! that fit.

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
}

/// What a setting's value is in TOML.
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Text,
    Number,
}

impl Key {
    /// Every setting, in the order pace documents them.
    pub(crate) const ALL: [Self; 5] = [
        Self::TimeZone,
        Self::WeekStart,
        Self::MostRecentCount,
        Self::HoldsDefaultSource,
        Self::StorageSplit,
    ];

    /// The key as written in the file.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::TimeZone => "time-zone",
            Self::WeekStart => "week-start",
            Self::MostRecentCount => "most-recent-count",
            Self::HoldsDefaultSource => "holds.default-source",
            Self::StorageSplit => "storage.split",
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
        }
    }

    /// The values that fit, for an error about the TOML type.
    pub(crate) const fn expects(self) -> &'static str {
        match self {
            Self::TimeZone => "a time zone name like `Europe/Berlin`, or `system`",
            Self::WeekStart => "a day name like `monday`",
            Self::MostRecentCount => "a whole number from 1, like `9`",
            Self::HoldsDefaultSource => "`inner`, `outer` or `unknown`",
            Self::StorageSplit => "`year`, `month`, `week` or `day`",
        }
    }

    pub(crate) const fn kind(self) -> Kind {
        match self {
            Self::MostRecentCount => Kind::Number,
            Self::TimeZone | Self::WeekStart | Self::HoldsDefaultSource | Self::StorageSplit => {
                Kind::Text
            }
        }
    }
}
