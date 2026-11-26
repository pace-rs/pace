//! `pace now`: what is open, and where new entries go.

use abscissa_core::{Command, Runnable};
use jiff::{ToSpan, ZonedDateTime};
use pace_core::{Entry, EntryState, ProjectRef, State};
use pace_error::{ErrorKind, PaceError, PaceResult};
use pace_time::{TimeFrame, format_duration, relative_time};
use serde::Serialize;

use crate::{
    application::{Context, PACE_APP, exit_on_error},
    output::{self, clock},
    session::Session,
};

/// `pace now`
#[derive(clap::Args, Command, Debug)]
pub struct NowCmd {
    #[arg(long, help = "Answer as JSON, for scripts and status bars")]
    json: bool,
}

impl Runnable for NowCmd {
    fn run(&self) {
        exit_on_error(show(self.json));
    }
}

/// Show what is open; as JSON with `json`.
///
/// # Errors
///
/// If the store can't be read.
pub fn show(json: bool) -> PaceResult<()> {
    let context = PACE_APP.context()?;
    let session = Session::open(context)?;
    let now = session.now();
    let today = TimeFrame::Today.resolve(&now, context.settings.settings.week_start)?;
    let state = session.state(&today)?;
    if json {
        println!("{}", as_json(context, &session, &state, &now)?);
    } else {
        for line in as_text(context, &session, &state, &now) {
            println!("{line}");
        }
    }
    Ok(())
}

fn as_text(
    context: &Context,
    session: &Session,
    state: &State,
    now: &ZonedDateTime,
) -> Vec<String> {
    let open: Vec<&Entry> = state.open().collect();
    let mut lines = Vec::new();
    if open.is_empty() {
        lines.push("Nothing is running.".to_owned());
    }
    for entry in &open {
        lines.push(headline(state, entry, now));
        lines.push(format!("  {}.", details(entry, session, now).join("; ")));
    }
    if open.len() > 1 {
        lines.push(format!(
            "{} entries are open; `pace end <id>` ends one.",
            open.len()
        ));
    }
    for (first, second, start, end) in overlaps(state, now) {
        lines.push(format!(
            "Overlap: {} and {} both cover {}–{}; change one with `pace adjust <id>`.",
            output::short(state, first.id),
            output::short(state, second.id),
            clock(&start, now),
            clock(&end, now)
        ));
    }
    if let Some(project) = &context.project {
        lines.push(format!(
            "New entries here go to {} ({}).",
            project.key, project.reason
        ));
    }
    lines
}

fn headline(state: &State, entry: &Entry, now: &ZonedDateTime) -> String {
    let name = output::name(state, entry);
    let since =
        |moment: &ZonedDateTime| format!("{} ({})", clock(moment, now), relative_time(moment, now));
    match entry.state() {
        EntryState::Held => {
            let hold = entry.holds.iter().find(|hold| hold.end.is_none());
            let why = hold
                .and_then(|hold| hold.reason.as_deref())
                .map_or_else(String::new, |reason| format!(": {reason}"));
            let start = hold.map_or(&entry.start, |hold| &hold.start);
            format!("Held: {name}, since {}{why}.", since(start))
        }
        EntryState::Paused => {
            let start = entry
                .pauses
                .iter()
                .find(|pause| pause.end.is_none())
                .map_or(&entry.start, |pause| &pause.start);
            format!("Paused: {name}, since {}.", since(start))
        }
        EntryState::Running | EntryState::Ended => {
            format!("Running: {name}, since {}.", since(&entry.start))
        }
    }
}

fn details(entry: &Entry, session: &Session, now: &ZonedDateTime) -> Vec<String> {
    let mut details = vec![format!("{} net so far", format_duration(entry.net(now)))];
    if entry.state() != EntryState::Running {
        details.push(format!("begun at {}", clock(&entry.start, now)));
    }
    let (held, paused) = (entry.hold_time(now), entry.pause_time(now));
    if !held.is_zero() {
        details.push(format!("held {}", format_duration(held)));
    }
    if !paused.is_zero() {
        details.push(format!("paused {}", format_duration(paused)));
    }
    if let Some(project) = &entry.project {
        details.push(format!("project {}", output::project_name(project)));
    }
    if !entry.tags.is_empty() {
        let tags: Vec<&str> = entry.tags.iter().map(pace_core::Tag::as_str).collect();
        details.push(format!("tags {}", tags.join(", ")));
    }
    if entry.device != session.device() {
        details.push("on another device".to_owned());
    }
    details
}

/// Pairs of entries that overlap, unless both are open (that is the
/// "entries are open" notice), with the time they share.
fn overlaps<'s>(
    state: &'s State,
    now: &ZonedDateTime,
) -> Vec<(&'s Entry, &'s Entry, ZonedDateTime, ZonedDateTime)> {
    let entries: Vec<&Entry> = state.entries().collect();
    let mut pairs = Vec::new();
    for (index, first) in entries.iter().enumerate() {
        for second in &entries[index + 1..] {
            if first.is_open() && second.is_open() {
                continue;
            }
            let start = (&first.start).max(&second.start).clone();
            let end = first
                .end
                .as_ref()
                .unwrap_or(now)
                .min(second.end.as_ref().unwrap_or(now))
                .clone();
            if start < end {
                pairs.push((*first, *second, start, end));
            }
        }
    }
    pairs
}

#[derive(Serialize)]
struct NowJson<'a> {
    version: u32,
    now: String,
    open: Vec<OpenJson<'a>>,
    project: Option<ProjectJson>,
}

#[derive(Serialize)]
struct OpenJson<'a> {
    id: String,
    short: String,
    state: &'static str,
    note: Option<&'a str>,
    project: Option<&'a ProjectRef>,
    tags: Vec<&'a str>,
    start: String,
    since: String,
    net_seconds: i64,
    net: String,
    gross_seconds: i64,
    held_seconds: i64,
    paused_seconds: i64,
    this_device: bool,
}

#[derive(Serialize)]
struct ProjectJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    key: String,
    reason: String,
}

fn as_json(
    context: &Context,
    session: &Session,
    state: &State,
    now: &ZonedDateTime,
) -> PaceResult<String> {
    let open = state
        .open()
        .map(|entry| {
            let since = match entry.state() {
                EntryState::Held => entry
                    .holds
                    .iter()
                    .find(|hold| hold.end.is_none())
                    .map(|hold| &hold.start),
                EntryState::Paused => entry
                    .pauses
                    .iter()
                    .find(|pause| pause.end.is_none())
                    .map(|pause| &pause.start),
                EntryState::Running | EntryState::Ended => None,
            }
            .unwrap_or(&entry.start);
            OpenJson {
                id: entry.id.to_string(),
                short: output::short(state, entry.id),
                state: match entry.state() {
                    EntryState::Running => "running",
                    EntryState::Held => "held",
                    EntryState::Paused => "paused",
                    EntryState::Ended => "ended",
                },
                note: entry.note.as_deref(),
                project: entry.project.as_ref(),
                tags: entry.tags.iter().map(pace_core::Tag::as_str).collect(),
                start: entry.start.to_string(),
                since: since.to_string(),
                net_seconds: entry.net(now).as_secs(),
                net: format_duration(entry.net(now)),
                gross_seconds: entry.gross(now).as_secs(),
                held_seconds: entry.hold_time(now).as_secs(),
                paused_seconds: entry.pause_time(now).as_secs(),
                this_device: entry.device == session.device(),
            }
        })
        .collect();
    let json = NowJson {
        version: 1,
        now: now.to_string(),
        open,
        project: context.project.as_ref().map(|project| ProjectJson {
            id: project.id.map(|id| id.to_string()),
            key: project.key.to_string(),
            reason: project.reason.to_string(),
        }),
    };
    serde_json::to_string_pretty(&json).map_err(|error| {
        PaceError::with_source(ErrorKind::Internal, "cannot write JSON", error).ask_report()
    })
}

/// Recent entries are those of the last 30 days.
#[must_use]
pub fn recent_range_start(now: &ZonedDateTime) -> ZonedDateTime {
    now.checked_sub(30.days()).unwrap_or_else(|_| now.clone())
}
