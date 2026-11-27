//! Choosing the entry a command acts on: by its ID or short ID, `--last`,
//! or the open entry.

use jiff::{ToSpan, ZonedDateTime};
use pace_core::{Entry, EntryId, EntryVerb, State};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_time::TimeRange;

use crate::prompt::Ask;
use crate::session::Session;

/// How far back pace looks for an entry first, in days, before it reads
/// everything: recent entries are the likely ones, and reading less is
/// faster.
const LOOK_BACK_DAYS: [i64; 2] = [1, 31];

/// An entry, and the state pace found it in.
#[derive(Debug)]
pub struct Chosen {
    /// The state that holds the entry.
    pub state: State,
    /// The entry.
    pub entry: Entry,
}

/// How the user names the entry a command acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target<'t> {
    /// `[ENTRY]`: its ID or short ID.
    Id(&'t str),
    /// `--last`: the entry that ended last.
    Last,
    /// Neither: the open entry.
    Open,
}

impl<'t> Target<'t> {
    /// The target of `[ENTRY]` and `--last`, which clap keeps apart.
    #[must_use]
    pub const fn new(entry: Option<&'t str>, last: bool) -> Self {
        match (entry, last) {
            (Some(text), _) => Self::Id(text),
            (None, true) => Self::Last,
            (None, false) => Self::Open,
        }
    }

    /// The target of `[ENTRY]` for a command without `--last`.
    #[must_use]
    pub const fn id_or_open(entry: Option<&'t str>) -> Self {
        Self::new(entry, false)
    }
}

/// The entry `target` names. The open entry is `None`, which leaves the
/// choice to the core; with several open entries and a terminal, the user
/// picks one.
///
/// # Errors
///
/// If the text names no entry, or the store fails.
pub fn target_entry(
    session: &Session,
    asker: &dyn Ask,
    target: Target<'_>,
) -> PaceResult<Option<EntryId>> {
    let chosen = match target {
        Target::Id(text) => named(session, asker, text)?,
        Target::Last => last_ended(session)?,
        Target::Open if asker.can_ask() => {
            let state = session.state(&at_now(session)?)?;
            return Ok(picked_open(&state, &session.now(), asker)?.map(|entry| entry.id));
        }
        Target::Open => return Ok(None),
    };
    Ok(Some(chosen.entry.id))
}

/// The entry `target` names, else the open entry; `verb` is the command,
/// for the hint when none or several are open.
///
/// # Errors
///
/// If nothing fits, several open entries do without a terminal, or the
/// store fails.
pub fn chosen_entry(
    session: &Session,
    asker: &dyn Ask,
    target: Target<'_>,
    verb: EntryVerb,
) -> PaceResult<Chosen> {
    match target {
        Target::Id(text) => return named(session, asker, text),
        Target::Last => return last_ended(session),
        Target::Open => {}
    }
    let state = session.state(&at_now(session)?)?;
    let entry = open_entry(&state, verb, &session.now(), asker)?.clone();
    Ok(Chosen { state, entry })
}

/// The open entry; of several, the one the user picks on a terminal.
fn open_entry<'s>(
    state: &'s State,
    verb: EntryVerb,
    now: &ZonedDateTime,
    asker: &dyn Ask,
) -> PaceResult<&'s Entry> {
    if asker.can_ask()
        && let Some(entry) = picked_open(state, now, asker)?
    {
        return Ok(entry);
    }
    state.only_open(verb, now)
}

/// The open entry the user picks, if several are open.
fn picked_open<'s>(
    state: &'s State,
    now: &ZonedDateTime,
    asker: &dyn Ask,
) -> PaceResult<Option<&'s Entry>> {
    let open: Vec<&Entry> = state.open().collect();
    if open.len() < 2 {
        return Ok(None);
    }
    pick(state, &open, "Which open entry?", now, asker).map(Some)
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
        let fitting = fitting(&state, full, &suffix);
        if let Some(entry) = one_named(&state, &fitting, text, &session.now(), asker)? {
            return Ok(Chosen { state, entry });
        }
    }
    Err(
        PaceError::new(ErrorKind::InvalidInput, "pace knows no entry {text}")
            .attach_context("text", text)
            .attach_hint("`pace now` shows the IDs of the open entries"),
    )
}

/// The entries of `state` with the ID `full`, or whose ID ends with
/// `suffix`.
fn fitting<'s>(state: &'s State, full: Option<EntryId>, suffix: &str) -> Vec<&'s Entry> {
    state
        .entries()
        .filter(|entry| {
            full.map_or_else(
                || entry.id.uuid().simple().to_string().ends_with(suffix),
                |id| entry.id == id,
            )
        })
        .collect()
}

/// The one of the `fitting` entries that `text` names: none, the only
/// one, or the one the user picks of several. Without a terminal, several
/// are an error that lists them.
fn one_named(
    state: &State,
    fitting: &[&Entry],
    text: &str,
    now: &ZonedDateTime,
    asker: &dyn Ask,
) -> PaceResult<Option<Entry>> {
    match fitting {
        [] => Ok(None),
        [entry] => Ok(Some((*entry).clone())),
        several if asker.can_ask() => {
            let question = format!("`{text}` fits several entries. Which one?");
            Ok(Some(pick(state, several, &question, now, asker)?.clone()))
        }
        several => {
            let labels: Vec<String> = several
                .iter()
                .map(|entry| state.entry_label(entry, now))
                .collect();
            Err(PaceError::new(
                ErrorKind::InvalidInput,
                "`{text}` fits {count} entries: {list}",
            )
            .attach_context("text", text)
            .attach_context("count", several.len().to_string())
            .attach_context("list", labels.join("; "))
            .attach_hint("type more of the ID"))
        }
    }
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
        .map(|entry| state.entry_label(entry, now))
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
                state.entry_label(open[0], &at("10:00")),
                state.entry_label(open[1], &at("10:00")),
            ]]
        );
    }

    /// Entries begun at 09:00 and 09:30 whose IDs both end in `1234`.
    fn two_ending_alike() -> State {
        State::from_events(&[
            begun(0x1234, "09:00", "Write offer"),
            begun(0x11234, "09:30", "Call client"),
        ])
    }

    #[test]
    fn short_id_that_fits_several_entries_lists_them_without_terminal() {
        let state = two_ending_alike();
        let fitting = fitting(&state, None, "1234");

        let error = one_named(
            &state,
            &fitting,
            "1234",
            &at("10:00"),
            &Scripted::without_terminal(),
        )
        .unwrap_err();

        let labels: Vec<String> = fitting
            .iter()
            .map(|entry| state.entry_label(entry, &at("10:00")))
            .collect();
        assert_eq!(
            error.to_string(),
            format!("`1234` fits 2 entries: {}", labels.join("; "))
        );
        assert_eq!(error.hint(), Some("type more of the ID"));
    }

    #[test]
    fn short_id_that_fits_several_entries_lets_user_pick_on_terminal() {
        let state = two_ending_alike();
        let fitting = fitting(&state, None, "1234");
        let asker = Scripted::choosing([1]);

        let entry = one_named(&state, &fitting, "1234", &at("10:00"), &asker)
            .unwrap()
            .unwrap();

        assert_eq!(entry.id, fitting[1].id);
        assert_eq!(
            asker.questions(),
            ["`1234` fits several entries. Which one?"]
        );
    }

    #[test]
    fn full_id_fits_only_its_entry() {
        let state = two_ending_alike();
        let full: EntryId = id(0xa, 0x1234);

        let fitting = fitting(&state, Some(full), &full.to_string());

        assert_eq!(fitting.len(), 1);
        assert_eq!(fitting[0].id, full);
    }

    #[test]
    fn short_id_that_fits_nothing_names_no_entry() {
        let state = two_ending_alike();

        let fitting = fitting(&state, None, "beef");

        assert!(
            one_named(
                &state,
                &fitting,
                "beef",
                &at("10:00"),
                &Scripted::without_terminal()
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn several_open_entries_let_user_pick_on_terminal() {
        let state = two_ending_alike();
        let asker = Scripted::choosing([0]);

        let entry = open_entry(&state, EntryVerb::End, &at("10:00"), &asker).unwrap();

        assert_eq!(entry.id, id::<EntryId>(0xa, 0x1234));
        assert_eq!(asker.questions(), ["Which open entry?"]);
    }

    #[test]
    fn one_open_entry_is_taken_without_asking() {
        let state = State::from_events(&[begun(1, "09:00", "Write offer")]);
        let asker = Scripted::choosing([]);

        let entry = open_entry(&state, EntryVerb::End, &at("10:00"), &asker).unwrap();

        assert_eq!(entry.id, id::<EntryId>(0xa, 1));
        assert!(picked_open(&state, &at("10:00"), &asker).unwrap().is_none());
        assert_eq!(asker.questions(), Vec::<String>::new());
    }

    #[test]
    fn picker_without_answer_in_range_is_internal_error() {
        let state = two_ending_alike();
        let open: Vec<&Entry> = state.open().collect();

        let error = pick(
            &state,
            &open,
            "Which open entry?",
            &at("10:00"),
            &Scripted::choosing([2]),
        )
        .unwrap_err();

        assert_eq!(error.kind(), ErrorKind::Internal);
        assert_eq!(error.to_string(), "the picker returned no entry");
    }
}
