//! Choosing the entry a command acts on: by its ID or short ID, `--last`,
//! or the open entry.

use jiff::{ToSpan, ZonedDateTime};
use pace_core::{Entry, EntryId, State};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_time::TimeRange;

use crate::{output, prompt, session::Session};

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
pub fn target(session: &Session, text: Option<&str>, last: bool) -> PaceResult<Option<EntryId>> {
    if text.is_some() || last {
        return chosen(session, text, last, "").map(|chosen| Some(chosen.entry.id));
    }
    if !prompt::interactive() {
        return Ok(None);
    }
    let state = session.state(&at_now(session)?)?;
    let open: Vec<&Entry> = state.open().collect();
    if open.len() < 2 {
        return Ok(None);
    }
    pick(&state, &open, "Which open entry?", &session.now()).map(|entry| Some(entry.id))
}

/// The entry `[ENTRY]` or `--last` names, else the open entry; `verb` is the
/// command, for the hint.
///
/// # Errors
///
/// If nothing fits, several open entries do without a terminal, or the
/// store fails.
pub fn chosen(session: &Session, text: Option<&str>, last: bool, verb: &str) -> PaceResult<Chosen> {
    if let Some(text) = text {
        return named(session, text);
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
        several if prompt::interactive() => {
            pick(&state, several, "Which open entry?", &session.now())?.clone()
        }
        several => {
            let now = session.now();
            let labels: Vec<String> = several
                .iter()
                .map(|entry| output::label(&state, entry, &now))
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
fn named(session: &Session, text: &str) -> PaceResult<Chosen> {
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
            several if prompt::interactive() => {
                let question = format!("`{text}` fits several entries. Which one?");
                pick(&state, several, &question, &session.now())?.clone()
            }
            several => {
                let now = session.now();
                let labels: Vec<String> = several
                    .iter()
                    .map(|entry| output::label(&state, entry, &now))
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

fn pick<'s>(
    state: &'s State,
    entries: &[&'s Entry],
    question: &str,
    now: &ZonedDateTime,
) -> PaceResult<&'s Entry> {
    let labels: Vec<String> = entries
        .iter()
        .map(|entry| output::label(state, entry, now))
        .collect();
    let choice = prompt::choose(question, &labels)?;
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
