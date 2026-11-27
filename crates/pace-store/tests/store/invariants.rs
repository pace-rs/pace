//! The entry's invariants (restart.md 3.5): whatever commands one device
//! runs, no entry ends before it starts or inside a break, and its holds and
//! pauses lie inside it, one open at most, without overlapping each other.

use std::{collections::BTreeSet, sync::Arc};

use jiff::{SignedDuration, ZonedDateTime};
use pace_core::{
    Catalog, Clock, Command, DeterminedBy, DeviceId, Entry, EntryId, Policy, RoleId, State, Tag,
    command::{Adjust, Begin, End, Forget, Hold, Log, Pause, Resume},
    testing::SequentialIds,
    testing::TestClock,
};
use pace_store::{Tracker, testing::MemoryStore};
use pace_time::TimeRange;
use proptest::{prelude::*, sample::Index, test_runner::TestCaseError};

use crate::fixed_seed;

/// What a step does, before it knows the time and the entries.
#[derive(Debug, Clone)]
enum Verb {
    Begin {
        allow_overlap: bool,
        fields: Fields,
    },
    Hold {
        reason: bool,
    },
    Pause {
        reason: bool,
    },
    Resume,
    End {
        entry: Option<Index>,
    },
    Log {
        minutes: i64,
        allow_overlap: bool,
        fields: Fields,
    },
    Adjust {
        entry: Option<Index>,
        start: Option<i64>,
        end: Option<i64>,
    },
    Undo,
    Forget {
        entry: Index,
    },
}

/// The entry fields a begin or a log gives besides its times: a role or
/// none, who determined it, and billable or not.
#[derive(Debug, Clone, Default)]
struct Fields {
    role: bool,
    determined_by: Option<DeterminedBy>,
    billable: Option<bool>,
}

impl Fields {
    /// The role every entry with a role gets.
    const ROLE: &str = "0193a8d0-000f-7000-8000-000000000001";

    fn role(&self) -> Result<Option<RoleId>, TestCaseError> {
        self.role
            .then(|| Self::ROLE.parse::<RoleId>())
            .transpose()
            .map_err(fail)
    }
}

fn fields() -> impl Strategy<Value = Fields> {
    (
        any::<bool>(),
        prop::option::of(prop_oneof![
            Just(DeterminedBy::Me),
            Just(DeterminedBy::Others)
        ]),
        any::<Option<bool>>(),
    )
        .prop_map(|(role, determined_by, billable)| Fields {
            role,
            determined_by,
            billable,
        })
}

/// One command: the clock moves on by `wait` minutes, and the command
/// names a time `back` minutes before now, or now.
#[derive(Debug, Clone)]
struct Step {
    wait: i64,
    back: Option<i64>,
    verb: Verb,
}

fn verb() -> impl Strategy<Value = Verb> {
    prop_oneof![
        3 => (any::<bool>(), fields())
            .prop_map(|(allow_overlap, fields)| Verb::Begin { allow_overlap, fields }),
        2 => any::<bool>().prop_map(|reason| Verb::Hold { reason }),
        2 => any::<bool>().prop_map(|reason| Verb::Pause { reason }),
        2 => Just(Verb::Resume),
        2 => any::<Option<Index>>().prop_map(|entry| Verb::End { entry }),
        1 => (1..120i64, any::<bool>(), fields())
            .prop_map(|(minutes, allow_overlap, fields)| Verb::Log { minutes, allow_overlap, fields }),
        2 => (any::<Option<Index>>(), prop::option::of(0..180i64), prop::option::of(0..180i64))
            .prop_map(|(entry, start, end)| Verb::Adjust { entry, start, end }),
        1 => Just(Verb::Undo),
        1 => any::<Index>().prop_map(|entry| Verb::Forget { entry }),
    ]
}

fn step() -> impl Strategy<Value = Step> {
    (0..45i64, prop::option::of(0..90i64), verb()).prop_map(|(wait, back, verb)| Step {
        wait,
        back,
        verb,
    })
}

fn minutes_before(now: &ZonedDateTime, minutes: i64) -> Result<ZonedDateTime, TestCaseError> {
    now.checked_sub(SignedDuration::from_mins(minutes))
        .map_err(fail)
}

/// The entry the index picks among all entries, if there are any.
fn pick(state: &State, index: Index) -> Option<EntryId> {
    let ids: Vec<EntryId> = state.entries().map(|entry| entry.id).collect();
    (!ids.is_empty()).then(|| ids[index.index(ids.len())])
}

fn command(step: &Step, now: &ZonedDateTime, state: &State) -> Result<Command, TestCaseError> {
    let at = step
        .back
        .map(|back| minutes_before(now, back))
        .transpose()?;
    let reason = |given: bool| given.then(|| "why".parse()).transpose();
    Ok(match &step.verb {
        Verb::Begin {
            allow_overlap,
            fields,
        } => Command::Begin(Begin {
            at,
            role: fields.role()?,
            determined_by: fields.determined_by,
            billable: fields.billable,
            allow_overlap: *allow_overlap,
            ..Begin::default()
        }),
        Verb::Hold { reason: given } => Command::Hold(Hold {
            at,
            reason: reason(*given)?,
            ..Hold::default()
        }),
        Verb::Pause { reason: given } => Command::Pause(Pause {
            at,
            reason: reason(*given)?,
            ..Pause::default()
        }),
        Verb::Resume => Command::Resume(Resume { at, entry: None }),
        Verb::End { entry } => Command::End(End {
            at,
            entry: entry.and_then(|index| pick(state, index)),
        }),
        Verb::Log {
            minutes,
            allow_overlap,
            fields,
        } => {
            let end = at.unwrap_or_else(|| now.clone());
            Command::Log(Log {
                role: fields.role()?,
                determined_by: fields.determined_by,
                billable: fields.billable,
                allow_overlap: *allow_overlap,
                ..Log::new(minutes_before(&end, *minutes)?, end)
            })
        }
        Verb::Adjust { entry, start, end } => Command::Adjust(Adjust {
            entry: entry.and_then(|index| pick(state, index)),
            start: start.map(|back| minutes_before(now, back)).transpose()?,
            end: end.map(|back| minutes_before(now, back)).transpose()?,
            tag: BTreeSet::from(["dev".parse::<Tag>().map_err(fail)?]),
            ..Adjust::default()
        }),
        Verb::Undo => Command::Undo,
        // With no entry to forget, the step undoes instead.
        Verb::Forget { entry } => {
            pick(state, *entry).map_or(Command::Undo, |entry| Command::Forget(Forget { entry }))
        }
    })
}

/// A break of the entry: when it began and, once closed, when it ended.
type Break<'entry> = (
    &'entry str,
    &'entry ZonedDateTime,
    Option<&'entry ZonedDateTime>,
);

fn breaks(entry: &Entry) -> Vec<Break<'_>> {
    let mut breaks: Vec<Break<'_>> = entry
        .holds
        .iter()
        .map(|hold| ("hold", &hold.start, hold.end.as_ref()))
        .chain(
            entry
                .pauses
                .iter()
                .map(|pause| ("pause", &pause.start, pause.end.as_ref())),
        )
        .collect();
    // By start; at the same start closed breaks come first, by their end, so
    // a closed break of no length comes before a longer one.
    breaks.sort_by_key(|(_, start, end)| (*start, end.is_none(), *end));
    breaks
}

/// What makes the entry illegal, if anything.
fn illegal(entry: &Entry) -> Option<String> {
    if let Some(end) = &entry.end
        && *end <= entry.start
    {
        return Some(format!(
            "it ends at {end}, not after its start at {}",
            entry.start
        ));
    }
    let breaks = breaks(entry);
    let open = breaks.iter().filter(|(_, _, end)| end.is_none()).count();
    if open > 1 {
        return Some(format!("{open} breaks are open at once"));
    }
    if open == 1 && entry.end.is_some() {
        return Some("it has ended with a break still open".to_owned());
    }
    for (kind, start, end) in &breaks {
        if **start < entry.start {
            return Some(format!("a {kind} begins at {start}, before the entry"));
        }
        if let Some(end) = end {
            if end < start {
                return Some(format!(
                    "a {kind} ends at {end}, before it began at {start}"
                ));
            }
            if let Some(entry_end) = &entry.end
                && *end > entry_end
            {
                return Some(format!(
                    "a {kind} ends at {end}, after the entry at {entry_end}"
                ));
            }
        }
    }
    for pair in breaks.windows(2) {
        if let [(kind, _, Some(end)), (next, next_start, _)] = pair
            && *end > *next_start
        {
            return Some(format!(
                "a {kind} until {end} overlaps a {next} from {next_start}"
            ));
        }
        if let [(kind, _, None), (next, _, _)] = pair {
            return Some(format!("a {next} begins while a {kind} is open"));
        }
    }
    None
}

fn device() -> Result<DeviceId, TestCaseError> {
    "0193a8d0-000d-7000-8000-000000000001".parse().map_err(fail)
}

fn fail(error: impl std::fmt::Display) -> TestCaseError {
    TestCaseError::fail(error.to_string())
}

/// Runs the steps on one device and checks every entry after each one; a
/// refused command is part of the game.
fn run_steps(steps: &[Step]) -> Result<(), TestCaseError> {
    let mut now: ZonedDateTime = "2026-11-26T08:00[Europe/Berlin]".parse().map_err(fail)?;
    let clock = Arc::new(TestClock::new(now.clone()));
    let store = MemoryStore::new(device()?, clock.clone());
    let tracker = Tracker::new(
        store,
        clock.clone(),
        Arc::new(SequentialIds::starting_at(&clock.now(), 1)),
        Policy::default(),
        Catalog::default(),
        device()?,
    );
    let everything = TimeRange::new(None, None).map_err(fail)?;
    let mut state = tracker.state(&everything).map_err(fail)?;
    for (number, step) in steps.iter().enumerate() {
        now = now
            .checked_add(SignedDuration::from_mins(step.wait))
            .map_err(fail)?;
        clock.set(now.clone());
        let command = command(step, &now, &state)?;
        _ = tracker.run(&command);
        state = tracker.state(&everything).map_err(fail)?;
        for entry in state.entries() {
            if let Some(why) = illegal(entry) {
                return Err(TestCaseError::fail(format!(
                    "after step {number}, {command:?} at {now}: entry {} is illegal: {why}\n{entry:#?}",
                    entry.id
                )));
            }
        }
    }
    Ok(())
}

#[test]
fn zero_length_pause_and_longer_hold_from_same_start_are_legal() -> Result<(), TestCaseError> {
    // Begins at 07:41, pauses from 07:41 to 07:41, holds from 07:41 to
    // 08:40 and pauses again: legal, though both closed breaks start at 07:41.
    let steps = [
        (
            0,
            Some(19),
            Verb::Begin {
                allow_overlap: false,
                fields: Fields::default(),
            },
        ),
        (14, Some(33), Verb::Pause { reason: false }),
        (21, Some(54), Verb::Hold { reason: false }),
        (5, None, Verb::Pause { reason: false }),
    ]
    .map(|(wait, back, verb)| Step { wait, back, verb });
    run_steps(&steps)
}

/// Tests that cost more than the others: 256 sequences of up to 40 commands,
/// each run through the tracker and followed by a check of every entry.
/// `.config/nextest.toml` runs them in the test group `expensive`.
mod expensive {
    use super::*;

    proptest! {
        #![proptest_config(fixed_seed())]

        #[test]
        fn no_command_sequence_builds_an_illegal_entry(
            steps in prop::collection::vec(step(), 1..40)
        ) {
            run_steps(&steps)?;
        }
    }
}
