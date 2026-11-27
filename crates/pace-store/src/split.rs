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
        f.write_str(match self {
            Self::Year => "year",
            Self::Month => "month",
            Self::Week => "week",
            Self::Day => "day",
        })
    }
}

impl FromStr for Split {
    type Err = Box<PaceError>;

    /// The split named `text`, in any case.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.to_lowercase().as_str() {
            "year" => Ok(Self::Year),
            "month" => Ok(Self::Month),
            "week" => Ok(Self::Week),
            "day" => Ok(Self::Day),
            _ => Err(
                PaceError::new(ErrorKind::InvalidInput, "`{split}` is not a split")
                    .attach_context("split", text)
                    .attach_hint("split by `year`, `month`, `week` or `day`"),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_split_reads_back_its_name() -> Result<(), Box<PaceError>> {
        for split in [Split::Year, Split::Month, Split::Week, Split::Day] {
            assert_eq!(split.to_string().parse::<Split>()?, split);
        }
        Ok(())
    }

    #[test]
    fn split_name_ignores_case() -> Result<(), Box<PaceError>> {
        assert_eq!("Week".parse::<Split>()?, Split::Week);
        Ok(())
    }
}
