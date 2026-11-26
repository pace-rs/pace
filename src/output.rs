//! How tracking commands describe what they did.

use jiff::ZonedDateTime;
use pace_core::{Entry, EntryId, EventKind, HoldSource, Outcome, PauseKind, State, short_ids};
use pace_time::format_duration;

/// A moment as the user reads it: `HH:MM` on the day of `now`, else with the
/// date; in the zone of `now`.
#[must_use]
pub fn clock(moment: &ZonedDateTime, now: &ZonedDateTime) -> String {
    let local = moment.with_time_zone(now.time_zone().clone());
    if local.date() == now.date() {
        local.strftime("%H:%M").to_string()
    } else {
        local.strftime("%Y-%m-%d %H:%M").to_string()
    }
}

/// The short ID of an entry, unique among the entries of `state`.
#[must_use]
pub fn short(state: &State, id: EntryId) -> String {
    let ids: Vec<EntryId> = state.entries().map(|entry| entry.id).collect();
    short_ids(&ids)
        .remove(&id)
        .unwrap_or_else(|| id.to_string())
}

/// The entry's short ID and its note, for a sentence.
#[must_use]
pub fn name(state: &State, entry: &Entry) -> String {
    let short = short(state, entry.id);
    entry
        .note
        .as_ref()
        .map_or_else(|| short.clone(), |note| format!("{short} \"{note}\""))
}

/// One line per event the command appended.
#[must_use]
pub fn describe(outcome: &Outcome) -> Vec<String> {
    let state = &outcome.state;
    let now = &outcome.now;
    outcome
        .recorded
        .iter()
        .filter_map(|recorded| {
            let at = &recorded.event.at;
            let line = match &recorded.event.kind {
                EventKind::EntryBegun { entry, .. } => {
                    let entry = state.entry(*entry)?;
                    format!("Began {} at {}.", name(state, entry), clock(at, now))
                }
                EventKind::EntryEnded { entry } => {
                    let entry = state.entry(*entry)?;
                    ended(state, entry, at, now)
                }
                EventKind::EntryHeld {
                    entry,
                    source,
                    reason,
                } => held(
                    state,
                    state.entry(*entry)?,
                    at,
                    *source,
                    reason.as_deref(),
                    now,
                ),
                EventKind::EntryPaused { entry, kind, .. } => {
                    let entry = state.entry(*entry)?;
                    format!(
                        "Paused {} at {} {}.",
                        short(state, entry.id),
                        clock(at, now),
                        pause_kind(*kind)
                    )
                }
                EventKind::EntryResumed { entry } => resumed(state, state.entry(*entry)?, at, now),
                EventKind::CatchNoted { entry, text } => {
                    format!("Caught \"{text}\" on {}.", short(state, *entry))
                }
                _ => return None,
            };
            Some(line)
        })
        .collect()
}

fn ended(state: &State, entry: &Entry, at: &ZonedDateTime, now: &ZonedDateTime) -> String {
    let (held, paused) = (entry.hold_time(now), entry.pause_time(now));
    let mut details = Vec::new();
    if !held.is_zero() || !paused.is_zero() {
        details.push(format!("{} in all", format_duration(entry.gross(now))));
    }
    if !held.is_zero() {
        details.push(format!("held {}", format_duration(held)));
    }
    if !paused.is_zero() {
        details.push(format!("paused {}", format_duration(paused)));
    }
    let details = if details.is_empty() {
        String::new()
    } else {
        format!(" ({})", details.join(", "))
    };
    format!(
        "Ended {} at {} after {}{details}.",
        name(state, entry),
        clock(at, now),
        format_duration(entry.net(now))
    )
}

fn held(
    state: &State,
    entry: &Entry,
    at: &ZonedDateTime,
    source: HoldSource,
    reason: Option<&str>,
    now: &ZonedDateTime,
) -> String {
    let hold = entry.holds.iter().find(|hold| hold.end.is_none());
    if let Some(hold) = hold.filter(|hold| hold.start < *at) {
        return format!(
            "{} is held since {}; the reason is now: {}.",
            short(state, entry.id),
            clock(&hold.start, now),
            reason.unwrap_or_default()
        );
    }
    let from = match source {
        HoldSource::Inner => ", interrupted from inside",
        HoldSource::Outer => ", interrupted from outside",
        HoldSource::Unknown => "",
    };
    let why = reason.map_or_else(String::new, |reason| format!(": {reason}"));
    format!(
        "Held {} at {}{from}{why}.",
        short(state, entry.id),
        clock(at, now)
    )
}

const fn pause_kind(kind: PauseKind) -> &'static str {
    match kind {
        PauseKind::Break => "for a break",
        PauseKind::Lunch => "for lunch",
        PauseKind::Pomodoro => "for a Pomodoro break",
        PauseKind::Recovery => "to recover",
    }
}

fn resumed(state: &State, entry: &Entry, at: &ZonedDateTime, now: &ZonedDateTime) -> String {
    let held = entry
        .holds
        .iter()
        .find(|hold| hold.end.as_ref() == Some(at))
        .map(|hold| (&hold.start, "held"));
    let paused = entry
        .pauses
        .iter()
        .find(|pause| pause.end.as_ref() == Some(at))
        .map(|pause| (&pause.start, "paused"));
    let after = held.or(paused).map_or_else(String::new, |(start, how)| {
        format!(" after {} {how}", format_duration(start.duration_until(at)))
    });
    format!(
        "Resumed {} at {}{after}.",
        short(state, entry.id),
        clock(at, now)
    )
}
