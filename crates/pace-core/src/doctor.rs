//! The data checks of `pace doctor`: the entries, the structure and the
//! billing, each a function without I/O on the state, the catalog and the
//! events.
//!
//! [`data_findings`] runs [`entry_findings`], [`structure_findings`] and
//! [`billing_findings`], in that order. Each problem becomes a
//! [`DataFinding`] that says what is wrong, the entry it is about and the
//! command that fixes it. The app reads every event and the catalog, and
//! adds the files and the line that a finding about a file points at.

mod conflicts;
mod entries;
mod rates;
mod structure;

use jiff::{ZonedDateTime, tz::TimeZone};

use crate::{Catalog, RecordedEvent, State};

pub use entries::entry_findings;
pub use rates::billing_findings;
pub use structure::structure_findings;

/// How bad a finding of `pace doctor` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingSeverity {
    /// Something pace can't work with; doctor exits with 1.
    Error,
    /// Something to look at; doctor still exits with 0.
    Warning,
}

impl FindingSeverity {
    /// The severity as the findings name it: `error` or `warning`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

/// The row of doctor's checks that a data finding belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataArea {
    /// The entries: overlaps, open entries, changes on two devices,
    /// projects without a record.
    Entries,
    /// The roles, clients and project records.
    Structure,
    /// Billed and billable entries against the rates.
    Billing,
}

impl DataArea {
    /// The area as the findings name it: `entries`, `structure` or
    /// `billing`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Entries => "entries",
            Self::Structure => "structure",
            Self::Billing => "billing",
        }
    }
}

/// One problem a data check found: what is wrong, the entry it is about,
/// and how to fix it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataFinding {
    /// How bad it is.
    pub severity: FindingSeverity,
    /// The row of doctor's checks it belongs to.
    pub area: DataArea,
    /// What is wrong, in one line.
    pub message: String,
    /// The short ID of the entry it is about, if it is about one.
    pub entry: Option<String>,
    /// The command or the step that fixes it.
    pub fix: Option<String>,
}

impl DataFinding {
    /// A warning in `area` that says `message` and names `fix`.
    const fn warning(area: DataArea, message: String, fix: String) -> Self {
        Self {
            severity: FindingSeverity::Warning,
            area,
            message,
            entry: None,
            fix: Some(fix),
        }
    }

    /// The finding about the entry with the short ID `entry`.
    fn about(self, entry: String) -> Self {
        Self {
            entry: Some(entry),
            ..self
        }
    }
}

/// What the data checks take.
#[derive(Debug, Clone, Copy)]
pub struct DoctorInput<'input> {
    /// The state of every event, its projects resolved against `catalog`
    /// ([`State::resolve_projects`]).
    pub state: &'input State,
    /// The roles, clients and project records.
    pub catalog: &'input Catalog,
    /// Every event of every device, as `state` was built from them.
    pub events: &'input [RecordedEvent],
    /// The reports' zone: an entry's day is its start day there, and the
    /// findings show times in it.
    pub zone: &'input TimeZone,
    /// Now, in `zone`.
    pub now: &'input ZonedDateTime,
}

/// Every finding of the data checks: the entries, the structure, then the
/// billing.
#[must_use]
pub fn data_findings(input: &DoctorInput<'_>) -> Vec<DataFinding> {
    let mut findings = entry_findings(input);
    findings.extend(structure_findings(input.catalog));
    findings.extend(billing_findings(input));
    findings
}
