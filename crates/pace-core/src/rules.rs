//! What each command turns into: the events of one batch, or an error that
//! says how to fix it.

use std::collections::BTreeSet;

use jiff::ZonedDateTime;
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_time::{clock, clock_against};
use uuid::Uuid;

use crate::{
    Batch, Change, Command, DeviceId, Entry, EntryChanges, EntryId, EntryVerb, Event, EventKind,
    HoldSource, Ids, State,
    command::{Adjust, Begin, Catch, End, Forget, Hold, Log, Pause, Resume},
};

mod checks;

const NOW_HINT: &str = "pace records what happened; `pace begin` starts an entry now";

const ENTRY_BEGAN: &str = "the entry began at {earlier}, after {at}";
const BREAK_BEGAN: &str = "the break began at {earlier}, after {at}";
const LAST_BREAK_ENDED: &str = "the last break ended at {earlier}, after {at}";

/// The command whose overlap [`Rules::refuse_overlap`] refuses.
#[derive(Debug, Clone, Copy)]
enum Verb {
    Begin,
    Log,
    Adjust,
}

/// The settings the rules depend on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    /// `holds.default-source`: the source of a hold without `--inner` or
    /// `--outer`.
    pub default_hold_source: HoldSource,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            default_hold_source: HoldSource::Unknown,
        }
    }
}

/// Decides commands against a state.
pub struct Rules<'a> {
    /// The current state.
    pub state: &'a State,
    /// Now, from the clock.
    pub now: &'a ZonedDateTime,
    /// The device the command runs on.
    pub device: DeviceId,
    /// Where new IDs come from.
    pub ids: &'a dyn Ids,
    /// The settings the rules depend on.
    pub policy: &'a Policy,
}

impl Rules<'_> {
    /// The events of one batch for `command`.
    ///
    /// # Errors
    ///
    /// If the command doesn't fit the state, with a hint how to fix it;
    /// "nothing to do" cases exit with [`ExitCode::NothingToDo`].
    pub fn decide(&self, command: &Command) -> PaceResult<Batch> {
        let events = match command {
            Command::Begin(begin) => self.begin(begin)?,
            Command::Hold(hold) => self.hold(hold)?,
            Command::Pause(pause) => self.pause(pause)?,
            Command::Resume(resume) => self.resume(resume)?,
            Command::Catch(catch) => self.catch(catch)?,
            Command::End(end) => self.end(end)?,
            Command::Log(log) => self.log(log)?,
            Command::Adjust(adjust) => self.adjust(adjust)?,
            Command::Undo => self.undo()?,
            Command::Forget(forget) => self.forget(forget)?,
        };
        Ok(Batch {
            id: self.new_id()?,
            events,
        })
    }

    fn begin(&self, begin: &Begin) -> PaceResult<Vec<Event>> {
        let start = begin.at.clone().unwrap_or_else(|| self.now.clone());
        self.refuse_future("start", &start, "leave out `--at` to begin now")?;
        let mut events = Vec::new();
        let mut ending = Vec::new();

        if let Some(running) = self.state.sole_open(EntryVerb::Begin, self.now)? {
            self.check_switch_after_start(running, &start)?;
            self.check_switch_after_breaks(running, &start)?;
            events.push(self.event(
                &start,
                &running.filed,
                EventKind::EntryEnded { entry: running.id },
            )?);
            ending.push(running.id);
        }

        if !begin.allow_overlap {
            self.refuse_overlap(&start, self.now, &ending, Verb::Begin)?;
        }

        let entry = self.new_id()?;
        events.push(self.event(
            &start,
            &start,
            EventKind::EntryBegun {
                entry,
                project: begin.project.clone(),
                note: begin.note.clone().map(String::from),
                tags: begin.tags.clone(),
            },
        )?);
        Ok(events)
    }

    fn hold(&self, hold: &Hold) -> PaceResult<Vec<Event>> {
        let entry = self.open_entry(None, EntryVerb::Hold)?;
        let at = hold.at.clone().unwrap_or_else(|| self.now.clone());
        self.refuse_future("be held", &at, "leave out `--at` to hold it now")?;
        if let Some(open) = entry.open_hold() {
            if hold.reason.is_none() {
                return Err(nothing_to_do(
                    "the entry is held since {since}",
                    "`pace resume` goes on with it",
                )
                .attach_context("since", self.clock(&open.start)));
            }
        } else {
            self.check_break_start(entry, &at)?;
        }
        Ok(vec![self.event(
            &at,
            &entry.filed,
            EventKind::EntryHeld {
                entry: entry.id,
                source: hold.source.unwrap_or(self.policy.default_hold_source),
                reason: hold.reason.clone().map(String::from),
            },
        )?])
    }

    fn pause(&self, pause: &Pause) -> PaceResult<Vec<Event>> {
        let entry = self.open_entry(None, EntryVerb::Pause)?;
        let at = pause.at.clone().unwrap_or_else(|| self.now.clone());
        self.refuse_future("be paused", &at, "leave out `--at` to pause it now")?;
        if let Some(open) = entry.open_pause() {
            if pause.reason.is_none() {
                return Err(nothing_to_do(
                    "the entry is paused since {since}",
                    "`pace resume` goes on with it",
                )
                .attach_context("since", self.clock(&open.start)));
            }
        } else {
            self.check_break_start(entry, &at)?;
        }
        Ok(vec![self.event(
            &at,
            &entry.filed,
            EventKind::EntryPaused {
                entry: entry.id,
                kind: pause.kind,
                reason: pause.reason.clone().map(String::from),
            },
        )?])
    }

    fn resume(&self, resume: &Resume) -> PaceResult<Vec<Event>> {
        let entry = self.open_entry(resume.entry, EntryVerb::Resume)?;
        let at = resume.at.clone().unwrap_or_else(|| self.now.clone());
        self.refuse_future("resume", &at, "leave out `--at` to resume it now")?;
        let Some(open_since) = entry.open_break_start() else {
            return Err(nothing_to_do(
                "the entry is running, not held or paused",
                "`pace hold` interrupts it, `pace pause` takes a break",
            ));
        };
        if at < *open_since {
            return Err(self.after(BREAK_BEGAN, open_since, &at));
        }
        Ok(vec![self.event(
            &at,
            &entry.filed,
            EventKind::EntryResumed { entry: entry.id },
        )?])
    }

    fn catch(&self, catch: &Catch) -> PaceResult<Vec<Event>> {
        if self.state.open().next().is_none() {
            return Err(PaceError::new(
                ErrorKind::InvalidInput,
                "nothing is running to note the thought on",
            )
            .attach_hint(
                "`pace begin` starts an entry; notes without an entry come with the inbox",
            ));
        }
        let entry = self.open_entry(None, EntryVerb::Catch)?;
        Ok(vec![self.event(
            self.now,
            &entry.filed,
            EventKind::CatchNoted {
                entry: entry.id,
                text: catch.text.to_string(),
            },
        )?])
    }

    fn end(&self, end: &End) -> PaceResult<Vec<Event>> {
        let entry = self.open_entry(end.entry, EntryVerb::End)?;
        let at = end.at.clone().unwrap_or_else(|| self.now.clone());
        self.refuse_future("end", &at, "leave out `--at` to end it now")?;
        if at < entry.start {
            return Err(self.after(ENTRY_BEGAN, &entry.start, &at));
        }
        self.check_length(&entry.start, &at)?;
        self.check_end_after_breaks(entry, &at)?;
        Ok(vec![self.event(
            &at,
            &entry.filed,
            EventKind::EntryEnded { entry: entry.id },
        )?])
    }

    fn log(&self, log: &Log) -> PaceResult<Vec<Event>> {
        self.check_length(&log.start, &log.end)?;
        self.refuse_future("end", &log.end, NOW_HINT)?;
        if !log.allow_overlap {
            self.refuse_overlap(&log.start, &log.end, &[], Verb::Log)?;
        }
        Ok(vec![self.event(
            &log.start,
            &log.start,
            EventKind::EntryLogged {
                entry: self.new_id()?,
                end: log.end.clone(),
                project: log.project.clone(),
                note: log.note.clone().map(String::from),
                tags: log.tags.clone(),
            },
        )?])
    }

    fn adjust(&self, adjust: &Adjust) -> PaceResult<Vec<Event>> {
        let entry = match adjust.entry {
            Some(id) => self.known_entry(id)?,
            None => self.open_entry(None, EntryVerb::Adjust)?,
        };
        if let Some(end) = &adjust.end
            && entry.is_open()
        {
            return Err(
                PaceError::new(ErrorKind::InvalidInput, "the entry is still open")
                    .attach_hint(format!("end it with `pace end --at {}`", self.clock(end))),
            );
        }

        let start = adjust.start.clone().unwrap_or_else(|| entry.start.clone());
        let end = adjust.end.clone().or_else(|| entry.end.clone());
        let changes = adjust.changes(entry);
        if changes.is_empty() {
            return Err(nothing_to_do(
                "nothing to change",
                "give `--start`, `--end`, `--note`, `--project`, `--tag` or `--untag`",
            ));
        }
        if changes.start.is_some() || changes.end.is_some() {
            self.check_moved_times(entry, &changes, &start, end.as_ref())?;
            if !adjust.allow_overlap {
                self.refuse_overlap(
                    &start,
                    end.as_ref().unwrap_or(self.now),
                    &[entry.id],
                    Verb::Adjust,
                )?;
            }
        }
        Ok(vec![self.event(
            self.now,
            &entry.filed,
            EventKind::EntryAdjusted {
                entry: entry.id,
                changes,
            },
        )?])
    }

    /// One `batch.undone` per moment the batch's events are filed under, so
    /// a store that reads a period sees the undo with the batch.
    fn undo(&self) -> PaceResult<Vec<Event>> {
        let target = self.state.last_undoable(self.device).ok_or_else(|| {
            nothing_to_do(
                "this device has nothing left to undo",
                "`pace undo` reverts the commands of this device, newest first",
            )
        })?;
        target
            .abouts
            .iter()
            .map(|about| {
                self.event(
                    self.now,
                    about,
                    EventKind::BatchUndone {
                        batch: target.batch,
                    },
                )
            })
            .collect()
    }

    fn forget(&self, forget: &Forget) -> PaceResult<Vec<Event>> {
        let entry = self.known_entry(forget.entry)?;
        Ok(vec![self.event(
            self.now,
            &entry.filed,
            EventKind::EntryForgotten { entry: entry.id },
        )?])
    }

    // Helpers

    fn new_id<T: TryFrom<Uuid, Error = Box<PaceError>>>(&self) -> PaceResult<T> {
        T::try_from(self.ids.next_id())
    }

    fn event(
        &self,
        at: &ZonedDateTime,
        about: &ZonedDateTime,
        kind: EventKind,
    ) -> PaceResult<Event> {
        Ok(Event {
            id: self.new_id()?,
            at: at.clone(),
            about: about.clone(),
            kind,
        })
    }

    /// The open entry `id`, or the only open entry; `verb` names the
    /// command for the hint when none or several are open.
    fn open_entry(&self, id: Option<EntryId>, verb: EntryVerb) -> PaceResult<&Entry> {
        let Some(id) = id else {
            return self.state.only_open(verb, self.now);
        };
        let entry = self.known_entry(id)?;
        if let Some(end) = &entry.end {
            return Err(nothing_to_do(
                "the entry ended at {end}",
                "`pace adjust --end` moves its end",
            )
            .attach_context("end", self.clock(end)));
        }
        Ok(entry)
    }

    fn known_entry(&self, id: EntryId) -> PaceResult<&Entry> {
        self.state.entry(id).ok_or_else(|| {
            PaceError::new(ErrorKind::InvalidInput, "pace knows no entry {id}")
                .attach_context("id", id.to_string())
                .attach_hint("`pace now` shows the IDs of the open entries")
        })
    }

    fn refuse_overlap(
        &self,
        start: &ZonedDateTime,
        end: &ZonedDateTime,
        except: &[EntryId],
        verb: Verb,
    ) -> PaceResult<()> {
        let overlapping: Vec<&Entry> = self
            .state
            .overlapping(start, end, except, self.now)
            .collect();
        let Some(latest_end) = overlapping
            .iter()
            .map(|entry| entry.end.as_ref().unwrap_or(self.now))
            .max()
        else {
            return Ok(());
        };
        let list: Vec<String> = overlapping
            .iter()
            .map(|entry| {
                let until = entry.end.as_ref().map_or_else(
                    || "now".to_owned(),
                    |until| self.clock_against(until, start),
                );
                format!(
                    "{} ({}–{until})",
                    self.state.short_id(entry.id),
                    self.clock_against(&entry.start, end)
                )
            })
            .collect();
        let (guidance, hint) = match verb {
            Verb::Begin => (
                "the new entry would overlap {list}",
                format!(
                    "begin at {} or later, or allow it with `--allow-overlap`",
                    self.clock_against(latest_end, start)
                ),
            ),
            Verb::Log => (
                "the new entry would overlap {list}",
                "move it, or allow it with `pace log --allow-overlap`".to_owned(),
            ),
            Verb::Adjust => (
                "the entry would overlap {list}",
                "move it, or allow it with `pace adjust --allow-overlap`".to_owned(),
            ),
        };
        Err(PaceError::new(ErrorKind::InvalidInput, guidance)
            .attach_context("list", list.join(", "))
            .attach_hint(hint))
    }

    /// pace records what happened, so nothing starts or ends after now.
    fn refuse_future(&self, what: &str, moment: &ZonedDateTime, hint: &str) -> PaceResult<()> {
        if moment <= self.now {
            return Ok(());
        }
        Err(PaceError::new(
            ErrorKind::InvalidInput,
            "the entry would {what} at {moment}, in the future",
        )
        .attach_context("what", what)
        .attach_context("moment", self.clock_against(moment, self.now))
        .attach_hint(hint))
    }

    /// A moment as the user reads it, against now: [`clock`].
    fn clock(&self, moment: &ZonedDateTime) -> String {
        clock(moment, self.now)
    }

    /// `moment` told apart from `other`: [`clock_against`].
    fn clock_against(&self, moment: &ZonedDateTime, other: &ZonedDateTime) -> String {
        clock_against(moment, other, self.now)
    }
}

impl Adjust {
    /// The fields this adjustment really changes in `entry`: a value equal
    /// to the entry's own is no change.
    #[must_use]
    pub fn changes(&self, entry: &Entry) -> EntryChanges {
        let tags: BTreeSet<_> = entry
            .tags
            .union(&self.tag)
            .filter(|tag| !self.untag.contains(*tag))
            .cloned()
            .collect();
        EntryChanges {
            start: self.start.clone().filter(|start| *start != entry.start),
            end: self
                .end
                .clone()
                .filter(|end| entry.end.as_ref() != Some(end)),
            note: real_change(&self.note.clone().map(String::from), entry.note.as_ref()),
            project: real_change(&self.project, entry.project.as_ref()),
            tags: (tags != entry.tags).then_some(tags),
        }
    }
}

fn real_change<T: Clone + PartialEq>(change: &Change<T>, current: Option<&T>) -> Change<T> {
    match (change, current) {
        (Change::Set(value), Some(current)) if value == current => Change::Keep,
        (Change::Clear, None) | (Change::Keep, _) => Change::Keep,
        (change, _) => change.clone(),
    }
}

fn nothing_to_do(guidance: &'static str, hint: &'static str) -> Box<PaceError> {
    PaceError::new(ErrorKind::InvalidInput, guidance)
        .attach_hint(hint)
        .attach_exit_code(ExitCode::NothingToDo)
}
