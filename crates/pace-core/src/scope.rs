//! What a read of the event log must return: the entries of a time range,
//! the open ones, and those with an event recorded since a moment.

use jiff::Timestamp;
use pace_time::TimeRange;

/// What a read of the event log returns.
///
/// It returns every event of the entries whose time meets
/// [`range`](Self::range), of the open entries, and, with
/// [`recorded_since`](Self::recorded_since), of the entries with an event
/// recorded at or after that moment; and the undos of their batches. An
/// event is recorded at the wall time of its [`Hlc`](crate::Hlc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadScope {
    range: TimeRange,
    recorded_since: Option<Timestamp>,
}

impl ReadScope {
    /// Every event of every entry.
    #[must_use]
    pub const fn everything() -> Self {
        Self::of(TimeRange::everything())
    }

    /// The entries whose time meets `range`, and the open ones.
    #[must_use]
    pub const fn of(range: TimeRange) -> Self {
        Self {
            range,
            recorded_since: None,
        }
    }

    /// The scope, and the entries with an event recorded at or after
    /// `since`.
    #[must_use]
    pub fn with_recorded_since(self, since: Timestamp) -> Self {
        Self {
            recorded_since: Some(since),
            ..self
        }
    }

    /// The time whose entries the read returns.
    #[must_use]
    pub const fn range(&self) -> &TimeRange {
        &self.range
    }

    /// The moment since which the read also returns the entries with an
    /// event recorded.
    #[must_use]
    pub const fn recorded_since(&self) -> Option<Timestamp> {
        self.recorded_since
    }

    /// Whether the read returns every event: its range is all time.
    #[must_use]
    pub const fn is_everything(&self) -> bool {
        self.range.is_everything()
    }
}

impl From<TimeRange> for ReadScope {
    fn from(range: TimeRange) -> Self {
        Self::of(range)
    }
}
