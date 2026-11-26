//! How tracking commands describe what they did.

use jiff::ZonedDateTime;
use pace_core::{
    Change, Entry, EntryChanges, EntryId, EventKind, HoldSource, Outcome, PauseKind, ProjectRef,
    State, Tag, short_ids,
};
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

/// `1 event`, `2 events`.
#[must_use]
pub fn counted(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// The short ID of an entry, unique among the entries of `state`.
#[must_use]
pub fn short_id(state: &State, id: EntryId) -> String {
    let ids: Vec<EntryId> = state.entries().map(|entry| entry.id).collect();
    short_ids(&ids)
        .remove(&id)
        .unwrap_or_else(|| id.to_string())
}

/// The entry's short ID and its note, for a sentence.
#[must_use]
pub fn entry_name(state: &State, entry: &Entry) -> String {
    let short = short_id(state, entry.id);
    entry
        .note
        .as_ref()
        .map_or_else(|| short.clone(), |note| format!("{short} \"{note}\""))
}

/// The entry for a list or a picker: short ID, note and times.
#[must_use]
pub fn entry_label(state: &State, entry: &Entry, now: &ZonedDateTime) -> String {
    format!("{} ({})", entry_name(state, entry), times(entry, now))
}

/// `09:00–10:00`, or `since 09:00` for an open entry.
#[must_use]
pub fn times(entry: &Entry, now: &ZonedDateTime) -> String {
    entry.end.as_ref().map_or_else(
        || format!("since {}", clock(&entry.start, now)),
        |end| format!("{}–{}", clock(&entry.start, now), clock(end, now)),
    )
}

/// A project as the user knows it: its key, or its ID.
#[must_use]
pub fn project_name(project: &ProjectRef) -> String {
    match project {
        ProjectRef::Key(key) => key.to_string(),
        ProjectRef::Id(id) => id.to_string(),
    }
}

/// One line per event the command appended; for an undo, what it took
/// back.
#[must_use]
pub fn describe_outcome(outcome: &Outcome) -> Vec<String> {
    let state = &outcome.state;
    let before = &outcome.before;
    let now = &outcome.now;
    let mut lines: Vec<String> = outcome
        .recorded
        .iter()
        .filter_map(|recorded| {
            let at = &recorded.event.at;
            let line = match &recorded.event.kind {
                EventKind::EntryBegun { entry, .. } => {
                    let entry = state.entry(*entry)?;
                    format!("Began {} at {}.", entry_name(state, entry), clock(at, now))
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
                        short_id(state, entry.id),
                        clock(at, now),
                        pause_kind(*kind)
                    )
                }
                EventKind::EntryResumed { entry } => resumed(state, state.entry(*entry)?, at, now),
                EventKind::CatchNoted { entry, text } => {
                    format!("Caught \"{text}\" on {}.", short_id(state, *entry))
                }
                EventKind::EntryLogged { entry, .. } => {
                    let entry = state.entry(*entry)?;
                    format!(
                        "Logged {} from {} to {} ({}).",
                        entry_name(state, entry),
                        clock(&entry.start, now),
                        clock(entry.end.as_ref()?, now),
                        format_duration(entry.net(now))
                    )
                }
                EventKind::EntryAdjusted { entry, changes } => adjusted(
                    before.entry(*entry)?,
                    state,
                    state.entry(*entry)?,
                    changes,
                    now,
                ),
                EventKind::EntryForgotten { entry } => {
                    let entry = before.entry(*entry)?;
                    format!(
                        "Forgot {} ({}) for good. Its events are gone from this device; other \
                         devices remove theirs when they see this.",
                        short_id(before, entry.id),
                        times(entry, now)
                    )
                }
                _ => return None,
            };
            Some(line)
        })
        .collect();
    lines.extend(undid(outcome));
    lines
}

/// What an undo took back, one line per undone event.
fn undid(outcome: &Outcome) -> Vec<String> {
    let before = &outcome.before;
    let now = &outcome.now;
    outcome
        .undone
        .iter()
        .filter_map(|recorded| {
            let at = clock(&recorded.event.at, now);
            let line = match &recorded.event.kind {
                EventKind::EntryBegun { entry, .. } => format!(
                    "Took back the start of {} at {at}.",
                    entry_name(before, before.entry(*entry)?)
                ),
                EventKind::EntryEnded { entry } => format!(
                    "Took back the end of {} at {at}; it is open again.",
                    entry_name(before, before.entry(*entry)?)
                ),
                EventKind::EntryHeld { entry, .. } => {
                    format!(
                        "Took back the hold of {} at {at}.",
                        short_id(before, *entry)
                    )
                }
                EventKind::EntryPaused { entry, .. } => {
                    format!(
                        "Took back the pause of {} at {at}.",
                        short_id(before, *entry)
                    )
                }
                EventKind::EntryResumed { entry } => {
                    format!("Took back resuming {} at {at}.", short_id(before, *entry))
                }
                EventKind::CatchNoted { text, .. } => {
                    format!("Took back the caught thought \"{text}\".")
                }
                EventKind::EntryLogged { entry, .. } => {
                    let entry = before.entry(*entry)?;
                    format!(
                        "Took back the logged entry {}, {}.",
                        entry_name(before, entry),
                        times(entry, now)
                    )
                }
                EventKind::EntryAdjusted { entry, .. } => format!(
                    "Took back the adjustment of {}.",
                    entry_name(before, before.entry(*entry)?)
                ),
                _ => return None,
            };
            Some(line)
        })
        .collect()
}

fn adjusted(
    before: &Entry,
    state: &State,
    after: &Entry,
    changes: &EntryChanges,
    now: &ZonedDateTime,
) -> String {
    let mut parts = Vec::new();
    if let Some(start) = &changes.start {
        parts.push(format!(
            "start {} (was {})",
            clock(start, now),
            clock(&before.start, now)
        ));
    }
    if let Some(end) = &changes.end {
        let was = before
            .end
            .as_ref()
            .map_or_else(|| "open".to_owned(), |end| clock(end, now));
        parts.push(format!("end {} (was {was})", clock(end, now)));
    }
    let quoted = |text: &String| format!("\"{text}\"");
    if let Some(part) = changed("note", &changes.note, before.note.as_ref(), quoted) {
        parts.push(part);
    }
    if let Some(part) = changed(
        "project",
        &changes.project,
        before.project.as_ref(),
        project_name,
    ) {
        parts.push(part);
    }
    if let Some(tags) = &changes.tags {
        let added: Vec<&str> = tags.difference(&before.tags).map(Tag::as_str).collect();
        let removed: Vec<&str> = before.tags.difference(tags).map(Tag::as_str).collect();
        if !added.is_empty() {
            parts.push(format!("tagged {}", added.join(", ")));
        }
        if !removed.is_empty() {
            parts.push(format!("untagged {}", removed.join(", ")));
        }
    }
    let net = if changes.start.is_some() || changes.end.is_some() {
        format!("; {} net now", format_duration(after.net(now)))
    } else {
        String::new()
    };
    format!(
        "Adjusted {}: {}{net}.",
        entry_name(state, after),
        parts.join(", ")
    )
}

/// `note "new" (was "old")`, `note removed (was "old")`, `note "new" (had none)`.
fn changed<T>(
    field: &str,
    change: &Change<T>,
    was: Option<&T>,
    show: impl Fn(&T) -> String,
) -> Option<String> {
    let was = was.map_or_else(|| "had none".to_owned(), |was| format!("was {}", show(was)));
    match change {
        Change::Keep => None,
        Change::Set(value) => Some(format!("{field} {} ({was})", show(value))),
        Change::Clear => Some(format!("{field} removed ({was})")),
    }
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
        entry_name(state, entry),
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
            short_id(state, entry.id),
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
        short_id(state, entry.id),
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
        short_id(state, entry.id),
        clock(at, now)
    )
}
