//! Half-open ranges of time, `[start, end)`, whose bounds may be open.

use jiff::ZonedDateTime;
use pace_error::{ErrorKind, PaceError, PaceResult};

/// A half-open range of time `[start, end)`. A missing bound is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeRange {
    start: Option<ZonedDateTime>,
    end: Option<ZonedDateTime>,
}

impl TimeRange {
    /// A range from `start` (inclusive) to `end` (exclusive).
    ///
    /// # Errors
    ///
    /// If `end` lies before `start`.
    pub fn new(start: Option<ZonedDateTime>, end: Option<ZonedDateTime>) -> PaceResult<Self> {
        if let (Some(start), Some(end)) = (&start, &end)
            && end < start
        {
            return Err(PaceError::new(
                ErrorKind::InvalidInput,
                "the time range ends at {end}, before it starts at {start}",
            )
            .attach_hint("swap the start and the end")
            .attach_context("start", start.to_string())
            .attach_context("end", end.to_string()));
        }
        Ok(Self { start, end })
    }

    /// The range of all time, open at both ends.
    #[must_use]
    pub const fn everything() -> Self {
        Self {
            start: None,
            end: None,
        }
    }

    /// Whether the range is open at both ends.
    #[must_use]
    pub const fn is_everything(&self) -> bool {
        self.start.is_none() && self.end.is_none()
    }

    /// The first moment in the range; `None` if the range is open at the start.
    #[must_use]
    pub const fn start(&self) -> Option<&ZonedDateTime> {
        self.start.as_ref()
    }

    /// The first moment after the range; `None` if the range is open at the end.
    #[must_use]
    pub const fn end(&self) -> Option<&ZonedDateTime> {
        self.end.as_ref()
    }

    /// Whether `moment` lies in the range.
    #[must_use]
    pub fn contains(&self, moment: &ZonedDateTime) -> bool {
        self.start.as_ref().is_none_or(|start| start <= moment)
            && self.end.as_ref().is_none_or(|end| moment < end)
    }
}
