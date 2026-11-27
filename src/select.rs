//! Choosing the entry a command acts on: by its ID or short ID, `--last`,
//! or the open entry.

use jiff::{ToSpan, ZonedDateTime};
use pace_core::{Entry, EntryId, State};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_time::TimeRange;

use crate::output::entry_label;
use crate::prompt::Ask;
use crate::session::Session;

/// How far back pace looks for an entry first, in days, before it reads
/// everything: recent entries are the likely ones, and reading less is
/// faster.
const LOOK_BACK_DAYS: [i64; 2] = [1, 31];

/// An entry, and the state pace found it in.
pub struct Chosen {
    /// The state that holds the entry.
    pub state: State,
    /// The entry.
    pub entry: Entry,
}

/// The entry `[ENTRY]` or `--last` names. Without either, `None` leaves
/// the choice to the core (the open entry); with several open entries and a
/// terminal, the user picks one.
///
/// # Errors
///
/// If the text names no entry, or the store fails.
pub fn target_entry(
    session: &Session,
    asker: &dyn Ask,
    text: Option<&str>,
    last: bool,
) -> PaceResult<Option<EntryId>> {
    if text.is_some() || last {
        return chosen_entry(session, asker, text, last, "").map(|chosen| Some(chosen.entry.id));
    }
    if !asker.can_ask() {
        return Ok(None);
    }
    let state = session.state(&at_now(session)?)?;
    let open: Vec<&Entry> = state.open().collect();
    if open.len() < 2 {
        return Ok(None);
    }
    pick(&state, &open, "Which open entry?", &session.now(), asker).map(|entry| Some(entry.id))
}

/// The entry `[ENTRY]` or `--last` names, else the open entry; `verb` is the
/// command, for the hint.
///
/// # Errors
///
/// If nothing fits, several open entries do without a terminal, or the
/// store fails.
pub fn chosen_entry(
    session: &Session,
    asker: &dyn Ask,
    text: Option<&str>,
    last: bool,
    verb: &str,
) -> PaceResult<Chosen> {
    if let Some(text) = text {
        return named(session, asker, text);
    }
    if last {
        return last_ended(session);
    }
    let state = session.state(&at_now(session)?)?;
    let open: Vec<&Entry> = state.open().collect();
    let entry = match open.as_slice() {
        [] => {
            return Err(PaceError::new(ErrorKind::InvalidInput, "nothing is running")
                .attach_hint(format!(
                    "name the entry with `pace {verb} <id>`, or take the last one with `pace {verb} --last`"
                ))
                .attach_exit_code(ExitCode::NothingToDo));
        }
        [entry] => (*entry).clone(),
        several if asker.can_ask() => {
            pick(&state, several, "Which open entry?", &session.now(), asker)?.clone()
        }
        several => {
            let now = session.now();
            let labels: Vec<String> = several
                .iter()
                .map(|entry| entry_label(&state, entry, &now))
                .collect();
            return Err(PaceError::new(
                ErrorKind::InvalidInput,
                "{count} entries are open: {list}",
            )
            .attach_context("count", several.len().to_string())
            .attach_context("list", labels.join("; "))
            .attach_hint(format!("name one with `pace {verb} <id>`")));
        }
    };
    Ok(Chosen { state, entry })
}

/// The entry with the ID `text`, or the one whose ID ends with `text`: at
/// least four hex characters. Recent entries come first.
fn named(session: &Session, asker: &dyn Ask, text: &str) -> PaceResult<Chosen> {
    let full = text.parse::<EntryId>().ok();
    let suffix = text.to_lowercase();
    if full.is_none() && (suffix.len() < 4 || !suffix.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return Err(
            PaceError::new(ErrorKind::InvalidInput, "`{text}` is no entry ID")
                .attach_context("text", text)
                .attach_hint("use the short ID pace shows, at least four characters")
                .attach_exit_code(ExitCode::Usage),
        );
    }
    for range in look_back(session) {
        let state = session.state(&range?)?;
        let fitting: Vec<&Entry> = state
            .entries()
            .filter(|entry| {
                full.map_or_else(
                    || entry.id.uuid().simple().to_string().ends_with(&suffix),
                    |id| entry.id == id,
                )
            })
            .collect();
        let entry = match fitting.as_slice() {
            [] => continue,
            [entry] => (*entry).clone(),
            several if asker.can_ask() => {
                let question = format!("`{text}` fits several entries. Which one?");
                pick(&state, several, &question, &session.now(), asker)?.clone()
            }
            several => {
                let now = session.now();
                let labels: Vec<String> = several
                    .iter()
                    .map(|entry| entry_label(&state, entry, &now))
                    .collect();
                return Err(PaceError::new(
                    ErrorKind::InvalidInput,
                    "`{text}` fits {count} entries: {list}",
                )
                .attach_context("text", text)
                .attach_context("count", several.len().to_string())
                .attach_context("list", labels.join("; "))
                .attach_hint("type more of the ID"));
            }
        };
        return Ok(Chosen { state, entry });
    }
    Err(
        PaceError::new(ErrorKind::InvalidInput, "pace knows no entry {text}")
            .attach_context("text", text)
            .attach_hint("`pace now` shows the IDs of the open entries"),
    )
}

/// The entry that ended last.
fn last_ended(session: &Session) -> PaceResult<Chosen> {
    for range in look_back(session) {
        let state = session.state(&range?)?;
        let last = state
            .entries()
            .filter(|entry| entry.end.is_some())
            .max_by(|first, second| first.end.cmp(&second.end))
            .cloned();
        if let Some(entry) = last {
            return Ok(Chosen { state, entry });
        }
    }
    Err(
        PaceError::new(ErrorKind::InvalidInput, "no entry has ended yet")
            .attach_hint("`pace begin` starts one")
            .attach_exit_code(ExitCode::NothingToDo),
    )
}

/// Let the user pick one of `entries` by its label.
fn pick<'s>(
    state: &'s State,
    entries: &[&'s Entry],
    question: &str,
    now: &ZonedDateTime,
    asker: &dyn Ask,
) -> PaceResult<&'s Entry> {
    let labels: Vec<String> = entries
        .iter()
        .map(|entry| entry_label(state, entry, now))
        .collect();
    let choice = asker.choose(question, &labels)?;
    entries.get(choice).copied().ok_or_else(|| {
        PaceError::new(ErrorKind::Internal, "the picker returned no entry").ask_report()
    })
}

/// The open entries and those around now.
fn at_now(session: &Session) -> PaceResult<TimeRange> {
    let now = session.now();
    TimeRange::new(Some(now.clone()), Some(now))
}

/// The time spans to look through, growing: the last day, month, then all.
fn look_back(session: &Session) -> impl Iterator<Item = PaceResult<TimeRange>> {
    let now = session.now();
    LOOK_BACK_DAYS
        .into_iter()
        .map(move |days| days_back(&now, days))
        .chain(std::iter::once(TimeRange::new(None, None)))
}

fn days_back(now: &ZonedDateTime, days: i64) -> PaceResult<TimeRange> {
    TimeRange::new(now.checked_sub(days.days()).ok(), Some(now.clone()))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use pace_core::{Event, EventKind, Hlc, RecordedEvent};

    use super::*;
    use crate::prompt::Scripted;

    fn id<T: std::str::FromStr>(prefix: u16, number: u32) -> T
    where
        T::Err: std::fmt::Debug,
    {
        format!("0193a8d0-{prefix:04x}-7000-8000-{number:012x}")
            .parse()
            .unwrap()
    }

    fn at(time: &str) -> ZonedDateTime {
        format!("2026-11-26T{time}[Europe/Berlin]").parse().unwrap()
    }

    /// Entry `number` begun at `time` with `note`.
    fn begun(number: u32, time: &str, note: &str) -> RecordedEvent {
        RecordedEvent {
            device: id(0xd, 1),
            batch: id(0xb, number),
            hlc: Hlc::new(at(time).timestamp(), number),
            event: Event {
                id: id(0xe, number),
                at: at(time),
                about: at(time),
                kind: EventKind::EntryBegun {
                    entry: id(0xa, number),
                    project: None,
                    note: Some(note.to_owned()),
                    tags: BTreeSet::new(),
                },
            },
        }
    }

    #[test]
    fn picker_returns_entry_user_chose() {
        let state = State::from_events(&[
            begun(1, "09:00", "Write offer"),
            begun(2, "09:30", "Call client"),
        ]);
        let open: Vec<&Entry> = state.open().collect();
        let asker = Scripted::choosing([1]);

        let entry = pick(&state, &open, "Which open entry?", &at("10:00"), &asker).unwrap();

        assert_eq!(entry.id, open[1].id);
        assert_eq!(asker.questions(), ["Which open entry?"]);
        assert_eq!(
            asker.offered(),
            [vec![
                entry_label(&state, open[0], &at("10:00")),
                entry_label(&state, open[1], &at("10:00")),
            ]]
        );
    }
}
