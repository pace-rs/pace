//! The open entry a command acts on, and how pace names entries in its
//! texts: the short ID, the note and the times.

use jiff::ZonedDateTime;
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_time::clock;

use crate::{Entry, State};

/// A command that acts on one entry, the open one unless the user names
/// it; it says how a hint names the entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryVerb {
    /// `pace begin`, which ends the running entry.
    Begin,
    /// `pace hold`.
    Hold,
    /// `pace pause`.
    Pause,
    /// `pace resume [ENTRY]`.
    Resume,
    /// `pace catch`.
    Catch,
    /// `pace end [ENTRY]`.
    End,
    /// `pace adjust [ENTRY] [--last]`.
    Adjust,
    /// `pace edit [ENTRY] [--last]`.
    Edit,
    /// `pace forget [ENTRY] [--last]`.
    Forget,
}

/// How a command lets the user name its entry.
enum Naming {
    /// It acts on the open entry only.
    OpenOnly,
    /// It takes the entry's ID.
    ById,
    /// It takes the entry's ID or `--last`.
    ByIdOrLast,
}

impl EntryVerb {
    /// The command as the user types it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Begin => "begin",
            Self::Hold => "hold",
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Catch => "catch",
            Self::End => "end",
            Self::Adjust => "adjust",
            Self::Edit => "edit",
            Self::Forget => "forget",
        }
    }

    const fn naming(self) -> Naming {
        match self {
            Self::Begin | Self::Hold | Self::Pause | Self::Catch => Naming::OpenOnly,
            Self::Resume | Self::End => Naming::ById,
            Self::Adjust | Self::Edit | Self::Forget => Naming::ByIdOrLast,
        }
    }
}

impl State {
    /// The open entry, if one is open; several open entries are an error
    /// whose hint says how `verb` names one of them.
    ///
    /// # Errors
    ///
    /// If several entries are open.
    pub fn sole_open(&self, verb: EntryVerb, now: &ZonedDateTime) -> PaceResult<Option<&Entry>> {
        match self.open().collect::<Vec<_>>().as_slice() {
            [] => Ok(None),
            [entry] => Ok(Some(entry)),
            several => Err(self.several_open(several, verb, now)),
        }
    }

    /// The open entry; none or several are an error whose hint says how
    /// `verb` names the entry.
    ///
    /// # Errors
    ///
    /// If no entry is open, with [`ExitCode::NothingToDo`], or several are.
    pub fn only_open(&self, verb: EntryVerb, now: &ZonedDateTime) -> PaceResult<&Entry> {
        self.sole_open(verb, now)?
            .ok_or_else(|| nothing_running(verb))
    }

    fn several_open(
        &self,
        open: &[&Entry],
        verb: EntryVerb,
        now: &ZonedDateTime,
    ) -> Box<PaceError> {
        let labels: Vec<String> = open
            .iter()
            .map(|entry| self.entry_label(entry, now))
            .collect();
        let hint = match verb.naming() {
            Naming::OpenOnly => "end one of them with `pace end <id>`".to_owned(),
            Naming::ById | Naming::ByIdOrLast => {
                format!("name one with `pace {} <id>`", verb.name())
            }
        };
        PaceError::new(ErrorKind::InvalidInput, "several entries are open: {list}")
            .attach_context("list", labels.join("; "))
            .attach_hint(hint)
    }

    /// The entry's short ID and its note, for a sentence:
    /// `ad5e "Write offer"`.
    #[must_use]
    pub fn entry_name(&self, entry: &Entry) -> String {
        let short = self.short_id(entry.id);
        entry
            .note
            .as_ref()
            .map_or_else(|| short.clone(), |note| format!("{short} \"{note}\""))
    }

    /// The entry for a list or a picker, with its times:
    /// `ad5e "Write offer" (since 09:00)`.
    #[must_use]
    pub fn entry_label(&self, entry: &Entry, now: &ZonedDateTime) -> String {
        format!("{} ({})", self.entry_name(entry), entry.clock_times(now))
    }
}

impl Entry {
    /// When the entry ran, against `now`: `09:00–10:00`, or `since 09:00`
    /// while it is open.
    #[must_use]
    pub fn clock_times(&self, now: &ZonedDateTime) -> String {
        self.end.as_ref().map_or_else(
            || format!("since {}", clock(&self.start, now)),
            |end| format!("{}–{}", clock(&self.start, now), clock(end, now)),
        )
    }
}

fn nothing_running(verb: EntryVerb) -> Box<PaceError> {
    let hint = match verb.naming() {
        Naming::OpenOnly | Naming::ById => "`pace begin` starts an entry".to_owned(),
        Naming::ByIdOrLast => format!(
            "name the entry with `pace {verb} <id>`, or take the last one with `pace {verb} --last`",
            verb = verb.name()
        ),
    };
    PaceError::new(ErrorKind::InvalidInput, "nothing is running")
        .attach_hint(hint)
        .attach_exit_code(ExitCode::NothingToDo)
}
