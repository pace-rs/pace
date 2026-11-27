//! `pace export`: every entry of a period, one row each, as JSON or CSV in
//! the schema of version 1.

use abscissa_core::{Command, Runnable};
use jiff::civil::Date;
use pace_core::{ExportInput, export_entries};
use pace_error::PaceResult;
use pace_time::{TimeFrame, TimeRange};

use crate::clock::zone;
use crate::commands::report_filter::{ReportFilterArgs, period_frame};
use crate::output::usage;
use crate::print::print_lines;
use crate::render::{
    export_rows,
    schema::{SchemaKind, schema_csv, schema_json},
};
use crate::{
    application::{Context, PACE_APP, exit_on_error},
    session::Session,
};

/// `pace export`
#[derive(clap::Args, Command, Debug)]
pub struct ExportCmd {
    #[arg(
        value_name = "PERIOD",
        conflicts_with_all = ["from", "to"],
        help = "today, yesterday, this-week, last-week, this-month, last-month, this-year, \
                last-year or all (the default)"
    )]
    period: Option<String>,

    #[arg(
        long,
        value_name = "DATE",
        help = "The first day, YYYY-MM-DD; without --to, the export runs to now"
    )]
    from: Option<Date>,

    #[arg(
        long,
        value_name = "DATE",
        help = "The last day, YYYY-MM-DD; without --from, the export starts at the first entry"
    )]
    to: Option<Date>,

    #[command(flatten)]
    filters: ReportFilterArgs,

    #[arg(
        long,
        value_enum,
        default_value_t = ExportFormat::Json,
        help = "How to write the entries"
    )]
    format: ExportFormat,
}

/// How `pace export` writes the entries.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// JSON for scripts, in the schema of version 1.
    Json,
    /// CSV for spreadsheets, in the schema of version 1.
    Csv,
}

impl Runnable for ExportCmd {
    fn run(&self) {
        exit_on_error(PACE_APP.context().and_then(|context| self.execute(context)));
    }
}

impl ExportCmd {
    fn execute(&self, context: &Context) -> PaceResult<()> {
        let session = Session::open_existing(context)?;
        let zone = zone(&context.settings.settings.reports_time_zone)?;
        let now = session.now().with_time_zone(zone.clone());
        let period = self
            .frame()?
            .resolve(&now, context.settings.settings.week_start)
            .map_err(usage)?;
        let state = session.state(&export_reading(&period)?)?;
        let filter = self.filters.filter(context, &session, &state)?;
        let entries = export_entries(&ExportInput {
            state: &state,
            catalog: &context.catalog,
            period: &period,
            filter: &filter,
            zone: &zone,
            now: &now,
        })?;
        let rows = export_rows(&entries);
        match self.format {
            ExportFormat::Json => print_lines([schema_json(SchemaKind::Export, &rows)?]),
            ExportFormat::Csv => print_lines(schema_csv(SchemaKind::Export, &rows)),
        }
    }

    /// The period: the named one, or `--from` and `--to`; all time
    /// without either.
    fn frame(&self) -> PaceResult<TimeFrame> {
        period_frame(self.period.as_deref(), self.from, self.to, TimeFrame::All)
    }
}

/// The range of events the export reads: from the period's start on,
/// with no end, so the state holds every later billing of its entries.
fn export_reading(period: &TimeRange) -> PaceResult<TimeRange> {
    TimeRange::new(period.start().cloned(), None)
}
