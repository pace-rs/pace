//! How a device's event log is split into files: by year, month, week or
//! day.

use std::{fmt, str::FromStr};

use jiff::ZonedDateTime;
use pace_error::{ErrorKind, PaceError};
use serde::{Deserialize, Serialize};

/// How a device's event log is split into files: one file per period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Split {
    /// `events/<device>/2026.toml`
    Year,
    /// `events/<device>/2026/2026-11.toml`, the default.
    #[default]
    Month,
    /// `events/<device>/2026/2026-W48.toml`, by ISO week.
    Week,
    /// `events/<device>/2026/11/2026-11-26.toml`
    Day,
}

const NAMES: [(&str, Split); 4] = [
    ("year", Split::Year),
    ("month", Split::Month),
    ("week", Split::Week),
    ("day", Split::Day),
];

impl Split {
    /// The file, relative to the device's directory, for events filed under
    /// `about`, by its date in the zone it was recorded in.
    #[must_use]
    pub fn file(self, about: &ZonedDateTime) -> String {
        let date = about.date();
        let (year, month, day) = (date.year(), date.month(), date.day());
        match self {
            Self::Year => format!("{year}.toml"),
            Self::Month => format!("{year}/{year}-{month:02}.toml"),
            Self::Week => {
                let week = date.iso_week_date();
                let week_year = week.year();
                format!("{week_year}/{week_year}-W{:02}.toml", week.week())
            }
            Self::Day => format!("{year}/{month:02}/{year}-{month:02}-{day:02}.toml"),
        }
    }
}

impl fmt::Display for Split {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = NAMES
            .iter()
            .find(|(_, split)| split == self)
            .map_or("month", |(name, _)| name);
        f.write_str(name)
    }
}

impl FromStr for Split {
    type Err = Box<PaceError>;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        NAMES
            .iter()
            .find(|(name, _)| *name == text)
            .map(|(_, split)| *split)
            .ok_or_else(|| {
                PaceError::new(ErrorKind::InvalidInput, "`{split}` is not a split")
                    .attach_context("split", text)
                    .attach_hint("split by `year`, `month`, `week` or `day`")
            })
    }
}
