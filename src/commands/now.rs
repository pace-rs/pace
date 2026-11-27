//! `pace now`: what is open, and where new entries go.

use abscissa_core::{Command, Runnable};
use jiff::{ToSpan, ZonedDateTime};
use pace_core::{Catalog, DeterminedBy, Entry, EntryState, ProjectRef, State};
use pace_error::{ErrorKind, PaceError, PaceResult};
use pace_time::{TimeFrame, format_duration, format_moment, relative_time};
use serde::Serialize;

use crate::output::{project_label, role_key, role_label};
use crate::print::print_lines;
use crate::{
    application::{Context, PACE_APP, exit_on_error},
    project::{Project, Reason},
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
        let format = if self.json {
            NowFormat::Json
        } else {
            NowFormat::Text
        };
        exit_on_error(show(format));
    }
}

/// How `pace now` answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NowFormat {
    /// Sentences for people.
    Text,
    /// JSON for scripts and status bars: `--json`.
    Json,
}

/// Show what is open, in `format`.
///
/// # Errors
///
/// If the store can't be read.
pub fn show(format: NowFormat) -> PaceResult<()> {
    let context = PACE_APP.context()?;
    let session = Session::open_existing(context)?;
    let now = session.now();
    let today = TimeFrame::Today.resolve(&now, context.settings.settings.week_start)?;
    let state = session.state(&today)?;
    match format {
        NowFormat::Json => print_lines([render_json(context, &session, &state, &now)?]),
        NowFormat::Text => print_lines(render_text(context, &session, &state, &now)),
    }
}

fn render_text(
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
        let details = details(state, entry, session, &context.catalog, now);
        lines.push(format!("  {}.", details.join("; ")));
    }
    if open.len() > 1 {
        lines.push(format!(
            "{} entries are open; `pace end <id>` ends one.",
            open.len()
        ));
    }
    // two open entries overlap until now: the notice above names them
    let shown = state
        .overlaps(now)
        .into_iter()
        .filter(|overlap| !(overlap.first.is_open() && overlap.second.is_open()));
    for overlap in shown {
        lines.push(format!(
            "Overlap: {} and {} both cover {}–{}; change one with `pace adjust <id>`.",
            state.short_id(overlap.first.id),
            state.short_id(overlap.second.id),
            format_moment(&overlap.start, now),
            format_moment(&overlap.end, now)
        ));
    }
    if let Some(project) = &context.project {
        let target = project.name.as_ref().map_or_else(
            || format!("{} ({})", project.key, project.reason),
            |name| format!("{name} ({}, {})", project.key, project.reason),
        );
        lines.push(format!("New entries here go to {target}."));
        lines.extend(project.warnings.iter().cloned());
    }
    lines
}

fn headline(state: &State, entry: &Entry, now: &ZonedDateTime) -> String {
    let name = state.entry_name(entry);
    let since = |moment: &ZonedDateTime| {
        format!(
            "{} ({})",
            format_moment(moment, now),
            relative_time(moment, now)
        )
    };
    match entry.state() {
        EntryState::Held => {
            let hold = entry.open_hold();
            let why = hold
                .and_then(|hold| hold.reason.as_deref())
                .map_or_else(String::new, |reason| format!(": {reason}"));
            let start = entry.open_break_start().unwrap_or(&entry.start);
            format!("Held: {name}, since {}{why}.", since(start))
        }
        EntryState::Paused => {
            let start = entry.open_break_start().unwrap_or(&entry.start);
            format!("Paused: {name}, since {}.", since(start))
        }
        EntryState::Running | EntryState::Ended => {
            format!("Running: {name}, since {}.", since(&entry.start))
        }
    }
}

fn details(
    state: &State,
    entry: &Entry,
    session: &Session,
    catalog: &Catalog,
    now: &ZonedDateTime,
) -> Vec<String> {
    let mut details = vec![format!("{} net so far", format_duration(entry.net(now)))];
    if entry.state() != EntryState::Running {
        details.push(format!("begun at {}", format_moment(&entry.start, now)));
    }
    let (held, paused) = (entry.hold_time(now), entry.pause_time(now));
    if !held.is_zero() {
        details.push(format!("held {}", format_duration(held)));
    }
    if !paused.is_zero() {
        details.push(format!("paused {}", format_duration(paused)));
    }
    if let Some(project) = &entry.project {
        details.push(format!("project {}", project_label(project, catalog)));
    }
    if let Some(role) = entry.effective_role(catalog) {
        details.push(format!("role {}", role_label(role, catalog)));
    }
    if !entry.tags.is_empty() {
        let tags: Vec<&str> = entry.tags.iter().map(pace_core::Tag::as_str).collect();
        details.push(format!("tags {}", tags.join(", ")));
    }
    if entry.determined_by == Some(DeterminedBy::Others) {
        details.push("other-determined".to_owned());
    }
    if let Some(billing) = state.billing(entry.id) {
        details.push(format!("billed ({})", billing.invoice));
    }
    if entry.device != session.device() {
        details.push("on another device".to_owned());
    }
    details
}

#[derive(Serialize)]
struct NowJson<'state> {
    version: u32,
    now: String,
    open: Vec<OpenJson<'state>>,
    project: Option<ProjectJson>,
}

#[derive(Serialize)]
struct OpenJson<'state> {
    id: String,
    short: String,
    state: &'static str,
    note: Option<&'state str>,
    project: Option<&'state ProjectRef>,
    tags: Vec<&'state str>,
    /// The key of the role its time goes to, its own or its project's;
    /// the role's ID without a record.
    role: Option<String>,
    determined_by: Option<DeterminedBy>,
    /// Whether its time is billable, by its own flag or its project's.
    billable: bool,
    /// Whether its overlap is marked as intended.
    allow_overlap: bool,
    /// The invoice it is billed on; `None` while it is not billed.
    billed: Option<&'state str>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    kind: ReasonKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    reason: String,
}

/// Why the project was picked, for scripts: [`Reason`] without its
/// sentence.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
enum ReasonKind {
    Flag,
    Variable,
    Manifest,
    Mapping,
}

impl ProjectJson {
    fn new(project: &Project) -> Self {
        let (kind, path) = match &project.reason {
            Reason::Flag => (ReasonKind::Flag, None),
            Reason::Variable => (ReasonKind::Variable, None),
            Reason::Manifest(path) => (ReasonKind::Manifest, Some(path)),
            Reason::Mapping { file, .. } => (ReasonKind::Mapping, Some(file)),
        };
        Self {
            id: project.id.map(|id| id.to_string()),
            key: project.key.to_string(),
            name: project.name.clone(),
            kind,
            path: path.map(|path| path.display().to_string()),
            reason: project.reason.to_string(),
        }
    }
}

fn render_json(
    context: &Context,
    session: &Session,
    state: &State,
    now: &ZonedDateTime,
) -> PaceResult<String> {
    let open = state
        .open()
        .map(|entry| {
            let since = entry.open_break_start().unwrap_or(&entry.start);
            OpenJson {
                id: entry.id.to_string(),
                short: state.short_id(entry.id),
                state: match entry.state() {
                    EntryState::Running => "running",
                    EntryState::Held => "held",
                    EntryState::Paused => "paused",
                    EntryState::Ended => "ended",
                },
                note: entry.note.as_deref(),
                project: entry.project.as_ref(),
                tags: entry.tags.iter().map(pace_core::Tag::as_str).collect(),
                role: entry
                    .effective_role(&context.catalog)
                    .map(|role| role_key(role, &context.catalog)),
                determined_by: entry.determined_by,
                billable: entry.effective_billable(&context.catalog),
                allow_overlap: entry.allow_overlap,
                billed: state
                    .billing(entry.id)
                    .map(|billing| billing.invoice.as_str()),
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
        project: context.project.as_ref().map(ProjectJson::new),
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
