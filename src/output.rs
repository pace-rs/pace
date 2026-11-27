//! How tracking commands describe what they did.

use jiff::ZonedDateTime;
use pace_core::{
    Change, Entry, EntryChanges, EntryId, EventKind, HoldSource, Outcome, PauseKind, ProjectRef,
    RecordedEvent, State, Tag, short_ids,
};
use pace_error::{ErrorKind, PaceError, PaceResult};
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
///
/// # Errors
///
/// If an event names an entry the outcome lacks, or the command appended
/// events and no line says so: both are bugs.
pub fn describe_outcome(outcome: &Outcome) -> PaceResult<Vec<String>> {
    let mut lines = Vec::new();
    for recorded in &outcome.recorded {
        lines.extend(recorded_line(outcome, recorded)?);
    }
    lines.extend(undid(outcome)?);
    if lines.is_empty() && !outcome.recorded.is_empty() {
        return Err(PaceError::new(
            ErrorKind::Internal,
            "pace recorded {count} and has no line to say so",
        )
        .attach_context("count", counted(outcome.recorded.len(), "event"))
        .ask_report());
    }
    Ok(lines)
}

/// The line for one appended event; an undo's lines come from what it took
/// back, in [`undid`].
fn recorded_line(outcome: &Outcome, recorded: &RecordedEvent) -> PaceResult<Option<String>> {
    let (state, before, now) = (&outcome.state, &outcome.before, &outcome.now);
    let at = &recorded.event.at;
    let line = match &recorded.event.kind {
        EventKind::EntryBegun { entry, .. } => format!(
            "Began {} at {}.",
            entry_name(state, known(state, *entry)?),
            clock(at, now)
        ),
        EventKind::EntryEnded { entry } => ended(state, known(state, *entry)?, at, now),
        EventKind::EntryHeld {
            entry,
            source,
            reason,
        } => held(
            state,
            known(state, *entry)?,
            at,
            *source,
            reason.as_deref(),
            now,
        ),
        EventKind::EntryPaused { entry, kind, .. } => format!(
            "Paused {} at {} {}.",
            short_id(state, known(state, *entry)?.id),
            clock(at, now),
            pause_kind(*kind)
        ),
        EventKind::EntryResumed { entry } => resumed(state, known(state, *entry)?, at, now),
        EventKind::CatchNoted { entry, text } => {
            format!("Caught \"{text}\" on {}.", short_id(state, *entry))
        }
        EventKind::EntryLogged { entry, end, .. } => {
            let entry = known(state, *entry)?;
            format!(
                "Logged {} from {} to {} ({}).",
                entry_name(state, entry),
                clock(at, now),
                clock(end, now),
                format_duration(entry.net(now))
            )
        }
        EventKind::EntryAdjusted { entry, changes } => adjusted(
            known(before, *entry)?,
            state,
            known(state, *entry)?,
            changes,
            now,
        ),
        EventKind::EntryForgotten { entry } => {
            let entry = known(before, *entry)?;
            format!(
                "Forgot {} ({}) for good. Its events are gone from this device; other \
                 devices remove theirs when they see this.",
                short_id(before, entry.id),
                times(entry, now)
            )
        }
        EventKind::BatchUndone { .. } => return Ok(None),
    };
    Ok(Some(line))
}

/// What an undo took back, one line per undone event. An event about an
/// entry forgotten since has nothing left to take back, so it has no line.
fn undid(outcome: &Outcome) -> PaceResult<Vec<String>> {
    let before = &outcome.before;
    outcome
        .undone
        .iter()
        .filter(|recorded| {
            recorded
                .event
                .kind
                .entry()
                .is_none_or(|entry| before.entry(entry).is_some())
        })
        .map(|recorded| took_back(before, recorded, &outcome.now))
        .collect()
}

/// The line for one undone event.
fn took_back(before: &State, recorded: &RecordedEvent, now: &ZonedDateTime) -> PaceResult<String> {
    let at = clock(&recorded.event.at, now);
    Ok(match &recorded.event.kind {
        EventKind::EntryBegun { entry, .. } => format!(
            "Took back the start of {} at {at}.",
            entry_name(before, known(before, *entry)?)
        ),
        EventKind::EntryEnded { entry } => format!(
            "Took back the end of {} at {at}; it is open again.",
            entry_name(before, known(before, *entry)?)
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
            let entry = known(before, *entry)?;
            format!(
                "Took back the logged entry {}, {}.",
                entry_name(before, entry),
                times(entry, now)
            )
        }
        EventKind::EntryAdjusted { entry, .. } => format!(
            "Took back the adjustment of {}.",
            entry_name(before, known(before, *entry)?)
        ),
        // The state never offers an undo or a forget to undo.
        EventKind::EntryForgotten { .. } | EventKind::BatchUndone { .. } => {
            return Err(PaceError::new(
                ErrorKind::Internal,
                "pace undid a batch with the event {event}, which can't be undone",
            )
            .attach_context("event", recorded.event.id.to_string())
            .ask_report());
        }
    })
}

/// The entry an event of this command names; without it, the outcome is
/// wrong.
fn known(state: &State, id: EntryId) -> PaceResult<&Entry> {
    state.entry(id).ok_or_else(|| {
        PaceError::new(
            ErrorKind::Internal,
            "pace recorded an event about the entry {entry}, which its state lacks",
        )
        .attach_context("entry", id.to_string())
        .ask_report()
    })
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use pace_core::{BatchId, Event, Hlc};

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn id<T: std::str::FromStr>(prefix: u16, number: u32) -> Result<T, T::Err> {
        format!("0193a8d0-{prefix:04x}-7000-8000-{number:012x}").parse()
    }

    fn nine() -> Result<ZonedDateTime, jiff::Error> {
        "2026-11-26T09:00[Europe/Berlin]".parse()
    }

    /// `kind` as event `number` of batch `batch`, at 09:00.
    fn recorded(
        batch: u32,
        number: u32,
        kind: EventKind,
    ) -> Result<RecordedEvent, Box<dyn std::error::Error>> {
        let at = nine()?;
        Ok(RecordedEvent {
            device: id(0xd, 1)?,
            batch: id(0xb, batch)?,
            hlc: Hlc::new(at.timestamp(), number),
            event: Event {
                id: id(0xe, number)?,
                at: at.clone(),
                about: at,
                kind,
            },
        })
    }

    fn begun(entry: EntryId) -> EventKind {
        EventKind::EntryBegun {
            entry,
            project: None,
            note: Some("Write offer".to_owned()),
            tags: BTreeSet::new(),
        }
    }

    fn outcome(
        before: &[RecordedEvent],
        recorded: Vec<RecordedEvent>,
        undone: Vec<RecordedEvent>,
    ) -> Result<Outcome, jiff::Error> {
        let mut all = before.to_vec();
        all.extend(recorded.iter().cloned());
        Ok(Outcome {
            recorded,
            before: State::from_events(before),
            state: State::from_events(&all),
            undone,
            now: nine()?,
        })
    }

    #[test]
    fn recorded_event_about_missing_entry_is_internal_error() -> TestResult {
        let missing: EntryId = id(0xa, 9)?;
        let outcome = outcome(
            &[],
            vec![recorded(1, 1, EventKind::EntryEnded { entry: missing })?],
            Vec::new(),
        )?;

        let error = describe_outcome(&outcome).err().ok_or("an error")?;

        assert_eq!(error.kind(), ErrorKind::Internal);
        Ok(())
    }

    #[test]
    fn undo_without_line_is_internal_error() -> TestResult {
        let batch: BatchId = id(0xb, 1)?;
        let outcome = outcome(
            &[],
            vec![recorded(2, 1, EventKind::BatchUndone { batch })?],
            Vec::new(),
        )?;

        let error = describe_outcome(&outcome).err().ok_or("an error")?;

        assert_eq!(error.kind(), ErrorKind::Internal);
        Ok(())
    }

    #[test]
    fn undo_takes_back_each_kind_of_event_in_its_own_words() -> TestResult {
        let (entry, logged): (EntryId, EntryId) = (id(0xa, 1)?, id(0xa, 3)?);
        let kinds = [
            begun(entry),
            EventKind::EntryHeld {
                entry,
                source: HoldSource::Outer,
                reason: None,
            },
            EventKind::EntryPaused {
                entry,
                kind: PauseKind::Break,
                reason: None,
            },
            EventKind::EntryResumed { entry },
            EventKind::CatchNoted {
                entry,
                text: "call Anna".to_owned(),
            },
            EventKind::EntryLogged {
                entry: logged,
                end: "2026-11-26T10:00[Europe/Berlin]".parse()?,
                project: None,
                note: Some("Review".to_owned()),
                tags: BTreeSet::new(),
            },
            EventKind::EntryAdjusted {
                entry,
                changes: EntryChanges::default(),
            },
            EventKind::EntryEnded { entry },
        ];
        let undone = (1..)
            .zip(kinds)
            .map(|(number, kind)| recorded(1, number, kind))
            .collect::<Result<Vec<_>, _>>()?;
        let batch = undone[0].batch;
        let undo = recorded(2, 20, EventKind::BatchUndone { batch })?;
        let outcome = outcome(&undone, vec![undo], undone.clone())?;

        let lines = describe_outcome(&outcome)?;

        assert_eq!(
            lines,
            [
                "Took back the start of 0001 \"Write offer\" at 09:00.",
                "Took back the hold of 0001 at 09:00.",
                "Took back the pause of 0001 at 09:00.",
                "Took back resuming 0001 at 09:00.",
                "Took back the caught thought \"call Anna\".",
                "Took back the logged entry 0003 \"Review\", 09:00–10:00.",
                "Took back the adjustment of 0001 \"Write offer\".",
                "Took back the end of 0001 \"Write offer\" at 09:00; it is open again.",
            ]
        );
        Ok(())
    }

    #[test]
    fn undone_forget_is_internal_error() -> TestResult {
        let entry: EntryId = id(0xa, 1)?;
        let began = recorded(1, 1, begun(entry))?;
        let forgot = recorded(2, 2, EventKind::EntryForgotten { entry })?;
        let undo = recorded(
            3,
            3,
            EventKind::BatchUndone {
                batch: forgot.batch,
            },
        )?;
        let outcome = outcome(&[began], vec![undo], vec![forgot])?;

        let error = describe_outcome(&outcome).err().ok_or("an error")?;

        assert_eq!(error.kind(), ErrorKind::Internal);
        Ok(())
    }

    #[test]
    fn undone_event_about_forgotten_entry_has_no_line() -> TestResult {
        let (kept, gone): (EntryId, EntryId) = (id(0xa, 1)?, id(0xa, 2)?);
        let began = recorded(1, 1, begun(kept))?;
        let ended = recorded(2, 2, EventKind::EntryEnded { entry: kept })?;
        let undo = recorded(3, 4, EventKind::BatchUndone { batch: ended.batch })?;
        let switched = vec![ended.clone(), recorded(2, 3, begun(gone))?];
        let outcome = outcome(&[began, ended], vec![undo], switched)?;

        let lines = describe_outcome(&outcome)?;

        assert_eq!(
            lines,
            ["Took back the end of 0001 \"Write offer\" at 09:00; it is open again."]
        );
        Ok(())
    }
}
