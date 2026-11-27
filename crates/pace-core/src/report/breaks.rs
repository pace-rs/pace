//! The breaks of a report: holds by source with their most frequent
//! reasons, pauses by kind, and the catches.

use std::{cmp::Reverse, collections::BTreeMap};

use jiff::{SignedDuration, Timestamp, ZonedDateTime, tz::TimeZone};
use serde::Serialize;

use crate::{Entry, EntryId, HoldSource, PauseKind, report::split::Stretch};

/// How many reasons of holds a report names at most.
pub const TOP_REASONS: usize = 5;

/// The interruptions: every hold, by source, and the most frequent
/// reasons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HoldReport {
    /// How many holds began in the period.
    pub count: usize,
    /// Their time inside the period.
    pub time: SignedDuration,
    /// Inner, outer and unknown, in this order, each also without holds.
    pub by_source: Vec<HoldShare>,
    /// The most frequent reasons, at most [`TOP_REASONS`], the most
    /// frequent first; of two as frequent, by text.
    pub reasons: Vec<ReasonCount>,
}

/// The holds of one source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HoldShare {
    /// Where they came from.
    pub source: HoldSource,
    /// How many.
    pub count: usize,
    /// Their time inside the period.
    pub time: SignedDuration,
}

/// A reason given for holds, and how often.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReasonCount {
    /// The reason, as written.
    pub reason: String,
    /// How many holds gave it.
    pub count: usize,
}

/// The pauses of one kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PauseShare {
    /// The kind of break.
    pub kind: PauseKind,
    /// How many began in the period.
    pub count: usize,
    /// Their time inside the period.
    pub time: SignedDuration,
}

/// A thought caught in the period.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReportCatch {
    /// The entry it was caught in.
    pub entry: EntryId,
    /// When, in the report's zone.
    pub at: ZonedDateTime,
    /// The thought.
    pub text: String,
}

const SOURCES: [HoldSource; 3] = [HoldSource::Inner, HoldSource::Outer, HoldSource::Unknown];
const KINDS: [PauseKind; 4] = [
    PauseKind::Break,
    PauseKind::Lunch,
    PauseKind::Pomodoro,
    PauseKind::Recovery,
];

/// The sums of the breaks, entry by entry.
#[derive(Debug)]
pub(crate) struct BreakTally {
    holds: Vec<HoldShare>,
    reasons: BTreeMap<String, usize>,
    pauses: Vec<PauseShare>,
    catches: Vec<ReportCatch>,
}

impl Default for BreakTally {
    fn default() -> Self {
        Self {
            holds: SOURCES
                .map(|source| HoldShare {
                    source,
                    count: 0,
                    time: SignedDuration::ZERO,
                })
                .to_vec(),
            reasons: BTreeMap::new(),
            pauses: KINDS
                .map(|kind| PauseShare {
                    kind,
                    count: 0,
                    time: SignedDuration::ZERO,
                })
                .to_vec(),
            catches: Vec::new(),
        }
    }
}

impl BreakTally {
    /// Add the holds, pauses and catches of `entry` that began in
    /// `period`; their time counts as far as it lies inside the entry and
    /// the period.
    pub(crate) fn add(
        &mut self,
        entry: &Entry,
        period: Stretch,
        now: &ZonedDateTime,
        zone: &TimeZone,
    ) {
        let inside = Inside::of(entry, period, now);
        for hold in &entry.holds {
            let Some(time) = inside.time(&hold.start, hold.end.as_ref()) else {
                continue;
            };
            if let Some(share) = self
                .holds
                .iter_mut()
                .find(|share| share.source == hold.source)
            {
                share.count += 1;
                share.time += time;
            }
            if let Some(reason) = &hold.reason {
                *self.reasons.entry(reason.clone()).or_default() += 1;
            }
        }
        for pause in &entry.pauses {
            let Some(time) = inside.time(&pause.start, pause.end.as_ref()) else {
                continue;
            };
            if let Some(share) = self
                .pauses
                .iter_mut()
                .find(|share| share.kind == pause.kind)
            {
                share.count += 1;
                share.time += time;
            }
        }
        self.catches.extend(
            entry
                .catches
                .iter()
                .filter(|caught| period.holds(caught.at.timestamp()))
                .map(|caught| ReportCatch {
                    entry: entry.id,
                    at: caught.at.with_time_zone(zone.clone()),
                    text: caught.text.clone(),
                }),
        );
    }

    /// The holds, the pauses of the kinds that occur, and the catches by
    /// time.
    pub(crate) fn finish(mut self) -> (HoldReport, Vec<PauseShare>, Vec<ReportCatch>) {
        let mut reasons: Vec<ReasonCount> = self
            .reasons
            .into_iter()
            .map(|(reason, count)| ReasonCount { reason, count })
            .collect();
        // the map gives the texts in order, and the sort keeps it
        reasons.sort_by_key(|reason| Reverse(reason.count));
        reasons.truncate(TOP_REASONS);
        let holds = HoldReport {
            count: self.holds.iter().map(|share| share.count).sum(),
            time: self.holds.iter().map(|share| share.time).sum(),
            by_source: self.holds,
            reasons,
        };
        self.pauses.retain(|share| share.count > 0);
        self.catches
            .sort_by(|a, b| a.at.cmp(&b.at).then(a.entry.cmp(&b.entry)));
        (holds, self.pauses, self.catches)
    }
}

/// Where an entry's breaks count: inside the entry, up to its end or now,
/// and inside the period.
#[derive(Debug, Clone, Copy)]
struct Inside {
    entry_start: Timestamp,
    entry_end: Timestamp,
    period: Stretch,
}

impl Inside {
    fn of(entry: &Entry, period: Stretch, now: &ZonedDateTime) -> Self {
        Self {
            entry_start: entry.start.timestamp(),
            entry_end: entry.until(now).timestamp(),
            period,
        }
    }

    /// The time of a break from `start` to `end`, or to the entry's end
    /// while open, if it began in the period; `None` if it began outside.
    fn time(&self, start: &ZonedDateTime, end: Option<&ZonedDateTime>) -> Option<SignedDuration> {
        let start = start.timestamp().max(self.entry_start);
        if !self.period.holds(start) {
            return None;
        }
        let end = end
            .map_or(self.entry_end, ZonedDateTime::timestamp)
            .min(self.entry_end);
        Some(
            Stretch::new(start, end)
                .and_then(|stretch| stretch.within(self.period))
                .map_or(SignedDuration::ZERO, Stretch::length),
        )
    }
}
