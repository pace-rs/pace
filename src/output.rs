//! How tracking commands describe what they did.

use jiff::ZonedDateTime;
use pace_core::{
    Catalog, Change, Entry, EntryChanges, EntryId, EventKind, HoldSource, Outcome, PauseKind,
    ProjectRef, RecordedEvent, State, Tag,
};
use pace_error::{ErrorKind, PaceError, PaceResult};
use pace_time::{format_duration, format_moment};

/// `1 event`, `2 events`.
#[must_use]
pub fn counted(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// A project as the user knows it: its key, or its ID.
#[must_use]
pub fn project_name(project: &ProjectRef) -> String {
    match project {
        ProjectRef::Key(key) => key.to_string(),
        ProjectRef::Id(id) => id.to_string(),
    }
}

/// A project as the user knows it: the name of its record in `catalog`,
/// else its key or its ID.
#[must_use]
pub fn project_label(project: &ProjectRef, catalog: &Catalog) -> String {
    catalog
        .resolve(project)
        .map_or_else(|| project_name(project), |record| record.name.clone())
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
            state.entry_name(known(state, *entry)?),
            format_moment(at, now)
        ),
        EventKind::EntryEnded { entry } => ended(state, known(state, *entry)?, at, now),
        EventKind::EntryHeld {
            entry,
            source,
            reason,
        } => held(
            outcome,
            known(state, *entry)?,
            at,
            *source,
            reason.as_deref(),
        ),
        EventKind::EntryPaused {
            entry,
            kind,
            reason,
        } => paused(outcome, known(state, *entry)?, at, *kind, reason.as_deref()),
        EventKind::EntryResumed { entry } => resumed(state, known(state, *entry)?, at, now),
        EventKind::CatchNoted { entry, text } => {
            format!("Caught \"{text}\" on {}.", state.short_id(*entry))
        }
        EventKind::EntryLogged { entry, end, .. } => {
            let entry = known(state, *entry)?;
            format!(
                "Logged {} from {} to {} ({}).",
                state.entry_name(entry),
                format_moment(at, now),
                format_moment(end, now),
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
                before.short_id(entry.id),
                entry.clock_times(now)
            )
        }
        EventKind::BatchUndone { .. } => return Ok(None),
    };
    Ok(Some(line))
}

/// What an undo took back, one line per undone event. An event about an
/// entry that is gone for good since, forgotten or begun in an undone batch,
/// has nothing left to take back, so it has no line.
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
    let at = format_moment(&recorded.event.at, now);
    Ok(match &recorded.event.kind {
        EventKind::EntryBegun { entry, .. } => format!(
            "Took back the start of {} at {at}.",
            before.entry_name(known(before, *entry)?)
        ),
        EventKind::EntryEnded { entry } => format!(
            "Took back the end of {} at {at}; it is open again.",
            before.entry_name(known(before, *entry)?)
        ),
        EventKind::EntryHeld { entry, .. } => {
            format!("Took back the hold of {} at {at}.", before.short_id(*entry))
        }
        EventKind::EntryPaused { entry, .. } => {
            format!(
                "Took back the pause of {} at {at}.",
                before.short_id(*entry)
            )
        }
        EventKind::EntryResumed { entry } => {
            format!("Took back resuming {} at {at}.", before.short_id(*entry))
        }
        EventKind::CatchNoted { text, .. } => {
            format!("Took back the caught thought \"{text}\".")
        }
        EventKind::EntryLogged { entry, .. } => {
            let entry = known(before, *entry)?;
            format!(
                "Took back the logged entry {}, {}.",
                before.entry_name(entry),
                entry.clock_times(now)
            )
        }
        EventKind::EntryAdjusted { entry, .. } => format!(
            "Took back the adjustment of {}.",
            before.entry_name(known(before, *entry)?)
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
            format_moment(start, now),
            format_moment(&before.start, now)
        ));
    }
    if let Some(end) = &changes.end {
        let was = before
            .end
            .as_ref()
            .map_or_else(|| "open".to_owned(), |end| format_moment(end, now));
        parts.push(format!("end {} (was {was})", format_moment(end, now)));
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
        state.entry_name(after),
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
        state.entry_name(entry),
        format_moment(at, now),
        format_duration(entry.net(now))
    )
}

/// A new hold, or a new reason for the hold that was open before.
fn held(
    outcome: &Outcome,
    entry: &Entry,
    at: &ZonedDateTime,
    source: HoldSource,
    reason: Option<&str>,
) -> String {
    let (state, now) = (&outcome.state, &outcome.now);
    let open = outcome.before.entry(entry.id).and_then(Entry::open_hold);
    if let Some(hold) = open {
        return reason_is_now(outcome, entry, "held", &hold.start, reason);
    }
    let from = match source {
        HoldSource::Inner => ", interrupted from inside",
        HoldSource::Outer => ", interrupted from outside",
        HoldSource::Unknown => "",
    };
    let why = reason.map_or_else(String::new, |reason| format!(": {reason}"));
    format!(
        "Held {} at {}{from}{why}.",
        state.short_id(entry.id),
        format_moment(at, now)
    )
}

/// A new pause, or a new reason for the pause that was open before.
fn paused(
    outcome: &Outcome,
    entry: &Entry,
    at: &ZonedDateTime,
    kind: PauseKind,
    reason: Option<&str>,
) -> String {
    let open = outcome.before.entry(entry.id).and_then(Entry::open_pause);
    if let Some(pause) = open {
        return reason_is_now(outcome, entry, "paused", &pause.start, reason);
    }
    format!(
        "Paused {} at {} {}.",
        outcome.state.short_id(entry.id),
        format_moment(at, &outcome.now),
        pause_kind(kind)
    )
}

/// `ad5e is held since 10:00; the reason is now: phone call.`
fn reason_is_now(
    outcome: &Outcome,
    entry: &Entry,
    how: &str,
    since: &ZonedDateTime,
    reason: Option<&str>,
) -> String {
    format!(
        "{} is {how} since {}; the reason is now: {}.",
        outcome.state.short_id(entry.id),
        format_moment(since, &outcome.now),
        reason.unwrap_or_default()
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
        state.short_id(entry.id),
        format_moment(at, now)
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
            role: None,
            determined_by: None,
            billable: None,
            allow_overlap: false,
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
            warnings: Vec::new(),
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
                role: None,
                determined_by: None,
                billable: None,
                allow_overlap: false,
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

    /// The lines for `kind`, recorded after entry 1 was begun.
    fn lines_after_begin(kind: EventKind) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let entry: EntryId = id(0xa, 1)?;
        let began = recorded(1, 1, begun(entry))?;
        let outcome = outcome(&[began], vec![recorded(2, 2, kind)?], Vec::new())?;
        Ok(describe_outcome(&outcome)?)
    }

    #[test]
    fn hold_says_where_interruption_came_from() -> TestResult {
        let entry: EntryId = id(0xa, 1)?;
        let held = |source| EventKind::EntryHeld {
            entry,
            source,
            reason: None,
        };

        assert_eq!(
            lines_after_begin(held(HoldSource::Inner))?,
            ["Held 0001 at 09:00, interrupted from inside."]
        );
        assert_eq!(
            lines_after_begin(held(HoldSource::Outer))?,
            ["Held 0001 at 09:00, interrupted from outside."]
        );
        assert_eq!(
            lines_after_begin(held(HoldSource::Unknown))?,
            ["Held 0001 at 09:00."]
        );
        Ok(())
    }

    #[test]
    fn pause_says_its_kind() -> TestResult {
        let entry: EntryId = id(0xa, 1)?;
        let mut lines = Vec::new();
        for kind in [
            PauseKind::Break,
            PauseKind::Lunch,
            PauseKind::Pomodoro,
            PauseKind::Recovery,
        ] {
            lines.extend(lines_after_begin(EventKind::EntryPaused {
                entry,
                kind,
                reason: None,
            })?);
        }

        assert_eq!(
            lines,
            [
                "Paused 0001 at 09:00 for a break.",
                "Paused 0001 at 09:00 for lunch.",
                "Paused 0001 at 09:00 for a Pomodoro break.",
                "Paused 0001 at 09:00 to recover.",
            ]
        );
        Ok(())
    }

    #[test]
    fn adjustment_names_removed_note_new_project_and_removed_tag() -> TestResult {
        let entry: EntryId = id(0xa, 1)?;
        let began = recorded(
            1,
            1,
            EventKind::EntryBegun {
                entry,
                project: None,
                note: Some("Write offer".to_owned()),
                tags: BTreeSet::from(["draft".parse()?]),
                role: None,
                determined_by: None,
                billable: None,
                allow_overlap: false,
            },
        )?;
        let changes = EntryChanges {
            note: Change::Clear,
            project: Change::Set(ProjectRef::Key("acme".parse()?)),
            tags: Some(BTreeSet::new()),
            ..EntryChanges::default()
        };
        let adjusted = recorded(2, 2, EventKind::EntryAdjusted { entry, changes })?;
        let outcome = outcome(&[began], vec![adjusted], Vec::new())?;

        assert_eq!(
            describe_outcome(&outcome)?,
            [
                "Adjusted 0001: note removed (was \"Write offer\"), project acme (had none), \
                 untagged draft."
            ]
        );
        Ok(())
    }
}
