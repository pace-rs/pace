//! The split of an entry's time: the stretches it was worked, outside its
//! holds and pauses, cut to the period and at midnight in the report's
//! zone.

use jiff::{SignedDuration, Timestamp, ZonedDateTime, civil::Date, tz::TimeZone};

use crate::Entry;

/// A stretch of time, from its start up to, not including, its end; never
/// empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Stretch {
    pub(crate) start: Timestamp,
    pub(crate) end: Timestamp,
}

impl Stretch {
    /// The stretch from `start` to `end`; `None` when it would be empty.
    pub(crate) fn new(start: Timestamp, end: Timestamp) -> Option<Self> {
        (start < end).then_some(Self { start, end })
    }

    /// The part of this stretch that lies inside `other`.
    pub(crate) fn within(self, other: Self) -> Option<Self> {
        Self::new(self.start.max(other.start), self.end.min(other.end))
    }

    /// Whether `moment` lies inside the stretch.
    pub(crate) fn contains(self, moment: Timestamp) -> bool {
        self.start <= moment && moment < self.end
    }

    /// How long it lasts.
    pub(crate) fn length(self) -> SignedDuration {
        self.start.duration_until(self.end)
    }
}

/// The stretches `entry` was worked: from its start to its end, or to `now`
/// while it is open, without its holds and pauses. Breaks that overlap,
/// merged from two devices, count once.
pub(crate) fn worked(entry: &Entry, now: &ZonedDateTime) -> Vec<Stretch> {
    let Some(whole) = Stretch::new(entry.start.timestamp(), entry.until(now).timestamp()) else {
        return Vec::new();
    };
    let mut breaks: Vec<Stretch> = entry
        .holds
        .iter()
        .map(|hold| (&hold.start, hold.end.as_ref()))
        .chain(
            entry
                .pauses
                .iter()
                .map(|pause| (&pause.start, pause.end.as_ref())),
        )
        .filter_map(|(start, end)| {
            let end = end.map_or(whole.end, ZonedDateTime::timestamp);
            Stretch::new(start.timestamp(), end).and_then(|stretch| stretch.within(whole))
        })
        .collect();
    breaks.sort_by_key(|stretch| stretch.start);

    let mut stretches = Vec::new();
    let mut from = whole.start;
    for gap in breaks {
        stretches.extend(Stretch::new(from, gap.start));
        from = from.max(gap.end);
    }
    stretches.extend(Stretch::new(from, whole.end));
    stretches
}

/// The days `stretch` touches in `zone`, each with the part of the stretch
/// that lies on it. A day lasts from midnight to midnight, so a day of a
/// clock change holds 23 or 25 hours.
pub(crate) fn per_day(stretch: Stretch, zone: &TimeZone) -> Vec<(Date, SignedDuration)> {
    let mut parts = Vec::new();
    let mut from = stretch.start;
    while from < stretch.end {
        let day = from.to_zoned_datetime(zone.clone()).date();
        // past jiff's last day, the rest of the stretch stays on it
        let next = next_midnight(day, zone)
            .filter(|next| *next > from)
            .map_or(stretch.end, |next| next.min(stretch.end));
        parts.push((day, from.duration_until(next)));
        from = next;
    }
    parts
}

/// The first moment of the day after `day` in `zone`.
fn next_midnight(day: Date, zone: &TimeZone) -> Option<Timestamp> {
    let next = day.tomorrow().ok()?;
    next.to_zoned_datetime(zone.clone())
        .ok()
        .map(|midnight| midnight.timestamp())
}
