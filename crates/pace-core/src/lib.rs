//! The domain of pace: its rules, its step machine and its commands. No I/O.
//!
//! The core is the hexagon of a ports-and-adapters architecture. Adapters
//! (CLI, later TUI and daemon) drive it with [`Command`] values. What the
//! core needs from the outside world comes in as values, or through the
//! [`Clock`] and [`Ids`] traits; the [`Notifier`] says where notices go.
//!
//! A command runs as a step machine without I/O: [`Reading`] names the
//! [`ReadScope`] to read, [`Reading::decide`] lets the [`Rules`] decide, and
//! [`Appending::appended`] gives the [`Outcome`] or a new read. The event
//! store and the blocking driver of the machine live in `pace_store`.
//!
//! The [`Catalog`] holds the roles, clients and project records, with their
//! [`Money`] and [`Rounding`]: values the store reads and the core takes as
//! an argument.
//!
//! [`Report::build`] computes the report of `reflect` from the [`State`] of
//! a period's events and the [`Catalog`]; the app reads the events.
//! [`billable_charges`] rounds and prices billable time, for the report's
//! [`Revenue`] and for the [`Bill`] of a client, which [`Bill::build`]
//! decides from the [`State`] and the [`Catalog`]. A billed entry keeps its
//! frozen [`Billing`] from its `billing.marked` event.
//!
//! Every timestamp is a jiff [`ZonedDateTime`](jiff::ZonedDateTime), so
//! entries keep the zone they were recorded in. All IDs are UUIDv7.
//!
//! A `pace begin` on an empty log, decided without a store; the IDs come
//! from `testing` (feature `testing`):
//!
//! ```
//! use jiff::ZonedDateTime;
//! use pace_core::{
//!     AfterRead, Catalog, ChangeMarker, Command, EventKind, Policy, Reading,
//!     command::Begin, testing::SequentialIds,
//! };
//!
//! let now: ZonedDateTime = "2026-11-26T09:00[Europe/Berlin]".parse()?;
//! let ids = SequentialIds::starting_at(&now, 1);
//! let policy = Policy::default();
//! let catalog = Catalog::default();
//! let begin = Command::Begin(Begin {
//!     note: Some("Write offer".parse()?),
//!     ..Begin::default()
//! });
//! let device = "0193a8d0-000d-7000-8000-000000000001".parse()?;
//!
//! let reading = Reading::start(&begin, now, device, &ids, &policy, &catalog)?;
//! // A driver reads `reading.scope()` from its store; this log is empty.
//! let after = reading.decide(ChangeMarker::new(0), Vec::new())?;
//! let AfterRead::Decided(decided) = after else {
//!     panic!("a begin needs no more than the open entries");
//! };
//!
//! let events = &decided.batch.events;
//! let kinds: Vec<&EventKind> = events.iter().map(|event| &event.kind).collect();
//! assert!(matches!(kinds[..], [EventKind::EntryBegun { .. }]));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod bill;
mod billing;
mod catalog;
pub mod command;
mod entry;
mod event;
mod id;
mod key;
mod open;
mod port;
mod report;
mod rules;
mod scope;
mod short_id;
mod state;
mod steps;
#[cfg(feature = "testing")]
pub mod testing;
mod text;
mod tracking;

pub use bill::{
    BILL_BEFORE_FIRST_RATE, BILL_OVERLAP, Bill, BillEntry, BillInput, BillLine, HoldTimeShare,
    InvoiceInput, ProjectBill, UNKNOWN_INVOICE, invoice_client,
};
pub use billing::{Billing, Charge, InvoiceRef, Price, billable_charges};
pub use catalog::{
    ArchiveReason, Archived, Catalog, Client, Currency, Money, ProjectRecord, Rate, Rates, Role,
    RoleKind, Rounding, RoundingMode, RoundingPer, Totals,
};
pub use command::Command;
pub use entry::{Entry, EntryCatch, EntryHold, EntryOrigin, EntryPause, EntryState, ProjectRef};
pub use event::{
    Batch, Change, ChangeMarker, Device, EntryChanges, Event, EventKind, Hlc, RecordedEvent,
};
pub use id::{BatchId, DeviceId, EntryId, EventId, ProjectId, RoleId};
pub use key::{ClientKey, ProjectKey, RoleKey, Tag};
pub use open::EntryVerb;
pub use port::{Appended, Clock, FixedClock, Ids, Notice, Notifier};
pub use report::{
    BudgetUse, ClientRevenue, Compared, ComparedBreaks, ComparedDecided, ComparedHolds,
    ComparedRevenue, DayTime, DecidedShares, Highlights, HoldReport, HoldShare, PauseShare,
    ProjectRevenue, ProjectTime, ReasonCount, Report, ReportCatch, ReportComparison, ReportFilter,
    ReportInput, ReportPeriod, ReportWarning, Revenue, RoleTarget, RoleTime, TOP_REASONS,
    TimeShare,
};
pub use rules::{BILLED, BilledChange, INVOICE_USED, OVERLAP, Policy, Rules, refuse_billed_change};
pub use scope::ReadScope;
pub use state::{Overlap, State, UndoTarget};
pub use steps::{AfterAppend, AfterRead, Appending, Decided, Outcome, Reading};
pub use text::Text;
pub use tracking::{DeterminedBy, HoldSource, PauseKind};
