//! What each command turns into: the events of one batch, or an error that
//! says how to fix it.

use std::collections::BTreeSet;

use jiff::ZonedDateTime;
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use uuid::Uuid;

use crate::{
    Batch, Change, Command, DeviceId, Entry, EntryChanges, EntryId, Event, EventKind, HoldSource,
    Ids, State,
    command::{Adjust, Begin, Catch, End, Forget, Hold, Log, Pause, Resume},
    short_ids,
};

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
        let mut events = Vec::new();
        let mut ending = Vec::new();

        match self.state.open().collect::<Vec<_>>().as_slice() {
            [] => {}
            [running] => {
                if running.start >= start {
                    return Err(PaceError::new(
                        ErrorKind::InvalidInput,
                        "the running entry began at {began}, after the new entry's start at {start}",
                    )
                    .attach_context("began", self.clock(&running.start))
                    .attach_context("start", self.clock(&start))
                    .attach_hint(format!(
                        "begin after {}, or move the running entry with `pace adjust --start`",
                        self.clock(&running.start)
                    )));
                }
                events.push(self.event(
                    &start,
                    &running.filed,
                    EventKind::EntryEnded { entry: running.id },
                )?);
                ending.push(running.id);
            }
            open => return Err(self.several_open(open)),
        }

        if !begin.allow_overlap {
            self.refuse_overlap(&start, self.now, &ending, "begin")?;
        }

        let entry = self.new_id()?;
        events.push(self.event(
            &start,
            &start,
            EventKind::EntryBegun {
                entry,
                project: begin.project.clone(),
                note: begin.note.clone(),
                tags: begin.tags.clone(),
            },
        )?);
        Ok(events)
    }

    fn hold(&self, hold: &Hold) -> PaceResult<Vec<Event>> {
        let entry = self.open_entry(None)?;
        let at = hold.at.clone().unwrap_or_else(|| self.now.clone());
        if let Some(open) = entry.holds.iter().find(|hold| hold.end.is_none()) {
            if hold.reason.is_none() {
                return Err(nothing_to_do(
                    format!("the entry is held since {}", self.clock(&open.start)),
                    "`pace resume` goes on with it",
                ));
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
                reason: hold.reason.clone(),
            },
        )?])
    }

    fn pause(&self, pause: &Pause) -> PaceResult<Vec<Event>> {
        let entry = self.open_entry(None)?;
        let at = pause.at.clone().unwrap_or_else(|| self.now.clone());
        if let Some(open) = entry.pauses.iter().find(|pause| pause.end.is_none()) {
            if pause.reason.is_none() {
                return Err(nothing_to_do(
                    format!("the entry is paused since {}", self.clock(&open.start)),
                    "`pace resume` goes on with it",
                ));
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
                reason: pause.reason.clone(),
            },
        )?])
    }

    fn resume(&self, resume: &Resume) -> PaceResult<Vec<Event>> {
        let entry = self.open_entry(resume.entry)?;
        let at = resume.at.clone().unwrap_or_else(|| self.now.clone());
        let Some(open_since) = open_break_start(entry) else {
            return Err(nothing_to_do(
                "the entry is running, not held or paused",
                "`pace hold` interrupts it, `pace pause` takes a break",
            ));
        };
        if at < *open_since {
            return Err(self.before(open_since, &at, "the break began"));
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
        let entry = self.open_entry(None)?;
        Ok(vec![self.event(
            self.now,
            &entry.filed,
            EventKind::CatchNoted {
                entry: entry.id,
                text: catch.text.clone(),
            },
        )?])
    }

    fn end(&self, end: &End) -> PaceResult<Vec<Event>> {
        let entry = self.open_entry(end.entry)?;
        let at = end.at.clone().unwrap_or_else(|| self.now.clone());
        if at < entry.start {
            return Err(self.before(&entry.start, &at, "the entry began"));
        }
        if let Some(open_since) = open_break_start(entry)
            && at < *open_since
        {
            return Err(self.before(open_since, &at, "the break began"));
        }
        Ok(vec![self.event(
            &at,
            &entry.filed,
            EventKind::EntryEnded { entry: entry.id },
        )?])
    }

    fn log(&self, log: &Log) -> PaceResult<Vec<Event>> {
        if log.end <= log.start {
            return Err(self.ends_before_start(&log.start, &log.end));
        }
        if !log.allow_overlap {
            self.refuse_overlap(&log.start, &log.end, &[], "log")?;
        }
        Ok(vec![self.event(
            &log.start,
            &log.start,
            EventKind::EntryLogged {
                entry: self.new_id()?,
                end: log.end.clone(),
                project: log.project.clone(),
                note: log.note.clone(),
                tags: log.tags.clone(),
            },
        )?])
    }

    fn adjust(&self, adjust: &Adjust) -> PaceResult<Vec<Event>> {
        let entry = match adjust.entry {
            Some(id) => self.known_entry(id)?,
            None => self.open_entry(None)?,
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
        if let Some(end) = &end
            && *end <= start
        {
            return Err(self.ends_before_start(&start, end));
        }
        self.check_breaks_inside(entry, &start, end.as_ref())?;

        let changes = changes(entry, adjust, &start, end.as_ref());
        if changes.is_empty() {
            return Err(nothing_to_do(
                "nothing to change",
                "give `--start`, `--end`, `--note`, `--project`, `--tag` or `--untag`",
            ));
        }
        if !adjust.allow_overlap && (changes.start.is_some() || changes.end.is_some()) {
            self.refuse_overlap(
                &start,
                end.as_ref().unwrap_or(self.now),
                &[entry.id],
                "adjust",
            )?;
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

    fn undo(&self) -> PaceResult<Vec<Event>> {
        let batch = self.state.last_undoable(self.device).ok_or_else(|| {
            nothing_to_do(
                "this device has nothing left to undo",
                "`pace undo` reverts the commands of this device, newest first",
            )
        })?;
        Ok(vec![self.event(
            self.now,
            self.now,
            EventKind::BatchUndone { batch },
        )?])
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

    /// The open entry `id`, or the only open entry.
    fn open_entry(&self, id: Option<EntryId>) -> PaceResult<&Entry> {
        if let Some(id) = id {
            let entry = self.known_entry(id)?;
            if let Some(end) = &entry.end {
                return Err(nothing_to_do(
                    format!("the entry ended at {}", self.clock(end)),
                    "`pace adjust --end` moves its end",
                ));
            }
            return Ok(entry);
        }
        match self.state.open().collect::<Vec<_>>().as_slice() {
            [] => Err(nothing_to_do(
                "nothing is running",
                "`pace begin` starts an entry",
            )),
            [entry] => Ok(entry),
            open => Err(self.several_open(open)),
        }
    }

    fn known_entry(&self, id: EntryId) -> PaceResult<&Entry> {
        self.state.entry(id).ok_or_else(|| {
            PaceError::new(ErrorKind::InvalidInput, "pace knows no entry {id}")
                .attach_context("id", id.to_string())
                .attach_hint("`pace now` shows the IDs of the open entries")
        })
    }

    fn several_open(&self, open: &[&Entry]) -> Box<PaceError> {
        let list: Vec<String> = open
            .iter()
            .map(|entry| {
                format!(
                    "{} since {}",
                    self.short(entry.id),
                    self.clock(&entry.start)
                )
            })
            .collect();
        PaceError::new(ErrorKind::InvalidInput, "{count} entries are open: {list}")
            .attach_context("count", open.len().to_string())
            .attach_context("list", list.join(", "))
            .attach_hint("end one of them with `pace end <id>`")
    }

    fn refuse_overlap(
        &self,
        start: &ZonedDateTime,
        end: &ZonedDateTime,
        except: &[EntryId],
        verb: &str,
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
                let end = entry
                    .end
                    .as_ref()
                    .map_or_else(|| "now".to_owned(), |end| self.clock(end));
                format!(
                    "{} ({}–{end})",
                    self.short(entry.id),
                    self.clock(&entry.start)
                )
            })
            .collect();
        let hint = if verb == "begin" {
            format!(
                "begin at {} or later, or allow it with `--allow-overlap`",
                self.clock(latest_end)
            )
        } else {
            format!("move it, or allow it with `pace {verb} --allow-overlap`")
        };
        Err(PaceError::new(
            ErrorKind::InvalidInput,
            "the new entry would overlap {list}",
        )
        .attach_context("list", list.join(", "))
        .attach_hint(hint))
    }

    /// A new hold or pause may not begin before the entry or before the end
    /// of an earlier one.
    fn check_break_start(&self, entry: &Entry, at: &ZonedDateTime) -> PaceResult<()> {
        if *at < entry.start {
            return Err(self.before(&entry.start, at, "the entry began"));
        }
        let last_break_end = entry
            .holds
            .iter()
            .filter_map(|hold| hold.end.as_ref())
            .chain(entry.pauses.iter().filter_map(|pause| pause.end.as_ref()))
            .max();
        if let Some(last) = last_break_end
            && at < last
        {
            return Err(self.before(last, at, "the last break ended"));
        }
        Ok(())
    }

    /// After an adjustment, holds and pauses still lie inside the entry.
    fn check_breaks_inside(
        &self,
        entry: &Entry,
        start: &ZonedDateTime,
        end: Option<&ZonedDateTime>,
    ) -> PaceResult<()> {
        let breaks = entry
            .holds
            .iter()
            .map(|hold| (&hold.start, hold.end.as_ref()))
            .chain(
                entry
                    .pauses
                    .iter()
                    .map(|pause| (&pause.start, pause.end.as_ref())),
            );
        for (break_start, break_end) in breaks {
            if break_start < start {
                return Err(PaceError::new(
                    ErrorKind::InvalidInput,
                    "the entry has a break at {at}, before the new start at {start}",
                )
                .attach_context("at", self.clock(break_start))
                .attach_context("start", self.clock(start))
                .attach_hint("pick a start before its first break"));
            }
            if let (Some(end), Some(break_end)) = (end, break_end)
                && break_end > end
            {
                return Err(PaceError::new(
                    ErrorKind::InvalidInput,
                    "the entry has a break until {at}, after the new end at {end}",
                )
                .attach_context("at", self.clock(break_end))
                .attach_context("end", self.clock(end))
                .attach_hint("pick an end after its last break"));
            }
        }
        Ok(())
    }

    fn before(&self, earlier: &ZonedDateTime, at: &ZonedDateTime, what: &str) -> Box<PaceError> {
        PaceError::new(ErrorKind::InvalidInput, "{what} at {earlier}, after {at}")
            .attach_context("what", what)
            .attach_context("earlier", self.clock(earlier))
            .attach_context("at", self.clock(at))
            .attach_hint(format!("give a time after {}", self.clock(earlier)))
    }

    fn ends_before_start(&self, start: &ZonedDateTime, end: &ZonedDateTime) -> Box<PaceError> {
        PaceError::new(
            ErrorKind::InvalidInput,
            "the entry would end at {end}, before it starts at {start}",
        )
        .attach_context("end", self.clock(end))
        .attach_context("start", self.clock(start))
        .attach_hint("give an end after the start")
    }

    /// A moment as the user reads it: `HH:MM` on the day of now, else with
    /// the date; in the zone of now.
    fn clock(&self, moment: &ZonedDateTime) -> String {
        let local = moment.with_time_zone(self.now.time_zone().clone());
        if local.date() == self.now.date() {
            local.strftime("%H:%M").to_string()
        } else {
            local.strftime("%Y-%m-%d %H:%M").to_string()
        }
    }

    fn short(&self, id: EntryId) -> String {
        let ids: Vec<EntryId> = self.state.entries().map(|entry| entry.id).collect();
        short_ids(&ids)
            .remove(&id)
            .unwrap_or_else(|| id.to_string())
    }
}

/// The start of the entry's open hold or pause, if any.
fn open_break_start(entry: &Entry) -> Option<&ZonedDateTime> {
    entry
        .holds
        .iter()
        .find(|hold| hold.end.is_none())
        .map(|hold| &hold.start)
        .or_else(|| {
            entry
                .pauses
                .iter()
                .find(|pause| pause.end.is_none())
                .map(|pause| &pause.start)
        })
}

/// The fields `adjust` really changes; a value equal to the current one is
/// no change.
fn changes(
    entry: &Entry,
    adjust: &Adjust,
    start: &ZonedDateTime,
    end: Option<&ZonedDateTime>,
) -> EntryChanges {
    let tags: BTreeSet<_> = entry
        .tags
        .union(&adjust.tag)
        .filter(|tag| !adjust.untag.contains(*tag))
        .cloned()
        .collect();
    EntryChanges {
        start: (*start != entry.start).then(|| start.clone()),
        end: end.filter(|end| entry.end.as_ref() != Some(*end)).cloned(),
        note: real_change(&adjust.note, entry.note.as_ref()),
        project: real_change(&adjust.project, entry.project.as_ref()),
        tags: (tags != entry.tags).then_some(tags),
    }
}

fn real_change<T: Clone + PartialEq>(change: &Change<T>, current: Option<&T>) -> Change<T> {
    match (change, current) {
        (Change::Set(value), Some(current)) if value == current => Change::Keep,
        (Change::Clear, None) | (Change::Keep, _) => Change::Keep,
        (change, _) => change.clone(),
    }
}

fn nothing_to_do(message: impl Into<String>, hint: impl Into<String>) -> Box<PaceError> {
    PaceError::new(ErrorKind::InvalidInput, message)
        .attach_hint(hint)
        .attach_exit_code(ExitCode::NothingToDo)
}
