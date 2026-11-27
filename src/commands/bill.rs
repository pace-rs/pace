//! `pace bill`: the bill of a client over a period, as a table, JSON, CSV
//! or Markdown from a template.

use abscissa_core::{Command, Runnable};
use jiff::civil::Date;
use pace_core::{Bill, BillInput, Catalog, Client, ClientKey, State};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_time::{TimeFrame, TimeRange};

use crate::clock::zone;
use crate::commands::client::unknown_client;
use crate::output::{listed, usage};
use crate::print::{print_bytes, print_lines};
use crate::render::{
    BillPeriod, BillRows, BillTable, BuiltInTemplate, Canvas, ReportTemplate, bill_rows,
    bill_table, render_report_template,
    schema::{SchemaKind, schema_csv, schema_document, schema_json},
};
use crate::{
    application::{Context, PACE_APP, exit_on_error},
    session::Session,
};

/// `pace bill`
#[derive(clap::Args, Command, Debug)]
pub struct BillCmd {
    #[arg(
        long,
        value_name = "KEY",
        help = "The client to bill; may be left out while pace knows only one client"
    )]
    client: Option<String>,

    #[arg(
        value_name = "PERIOD",
        conflicts_with_all = ["month", "from", "to"],
        help = "today, yesterday, this-week, last-week, this-month, last-month (the default), \
                this-year, last-year or all"
    )]
    period: Option<String>,

    #[arg(
        long,
        value_name = "YYYY-MM",
        value_parser = parse_month,
        conflicts_with_all = ["from", "to"],
        help = "A calendar month, such as 2026-11"
    )]
    month: Option<Date>,

    #[arg(
        long,
        value_name = "DATE",
        help = "The first day, YYYY-MM-DD; without --to, the bill runs to now"
    )]
    from: Option<Date>,

    #[arg(
        long,
        value_name = "DATE",
        help = "The last day, YYYY-MM-DD; without --from, the bill starts at the first entry"
    )]
    to: Option<Date>,

    #[arg(
        long,
        value_enum,
        default_value_t = BillFormat::Table,
        help = "How to show the bill"
    )]
    format: BillFormat,
}

/// How `pace bill` shows the bill.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillFormat {
    /// A table for the terminal.
    Table,
    /// JSON for scripts, in the schema of version 1.
    Json,
    /// CSV for spreadsheets, in the schema of version 1.
    Csv,
    /// Markdown, from a template.
    Markdown,
}

impl Runnable for BillCmd {
    fn run(&self) {
        exit_on_error(PACE_APP.context().and_then(|context| self.execute(context)));
    }
}

impl BillCmd {
    fn execute(&self, context: &Context) -> PaceResult<()> {
        let catalog = &context.catalog;
        let client = bill_client(catalog, self.client.as_deref())?;
        let session = Session::open_existing(context)?;
        let zone = zone(&context.settings.settings.reports_time_zone)?;
        let now = session.now().with_time_zone(zone.clone());
        let range = self
            .frame()?
            .resolve(&now, context.settings.settings.week_start)
            .map_err(usage)?;
        let state = session.state(&bill_reading(&range)?)?;
        let bill = Bill::build(&BillInput {
            state: &state,
            catalog,
            client,
            period: &range,
            zone: &zone,
            now: &now,
        })?;
        let period = BillPeriod::of(&range, &zone);
        if bill.is_empty() {
            return Err(nothing_to_bill(&bill, client, period, &state));
        }
        let rows = || {
            bill_rows(&BillRows {
                bill: &bill,
                client,
                range: &range,
                period,
                catalog,
                state: &state,
            })
        };
        match self.format {
            BillFormat::Table => print_lines(bill_table(
                &BillTable {
                    bill: &bill,
                    client,
                    period,
                    zone: zone.iana_name().unwrap_or("the system's time zone"),
                    catalog,
                    state: &state,
                },
                Canvas::detect(context.plain),
            )),
            BillFormat::Json => print_lines([schema_json(SchemaKind::Bill, &rows())?]),
            BillFormat::Csv => print_lines(schema_csv(SchemaKind::Bill, &rows())),
            BillFormat::Markdown => {
                let dir = context.settings.settings.reports_templates.as_deref();
                let template =
                    ReportTemplate::from_templates_dir(BuiltInTemplate::BillMarkdown, dir)?;
                let rows = rows();
                let document = schema_document(SchemaKind::Bill, &rows);
                print_bytes(render_report_template(&template, &document)?.as_bytes())
            }
        }
    }

    /// The period: the named one, `--month`, or `--from` and `--to`; last
    /// month without any.
    fn frame(&self) -> PaceResult<TimeFrame> {
        match (&self.period, self.month, self.from, self.to) {
            (Some(period), ..) => period.parse().map_err(usage),
            (None, Some(first), ..) => Ok(TimeFrame::Between {
                from: Some(first),
                to: Some(first.last_of_month()),
            }),
            (None, None, None, None) => Ok(TimeFrame::LastMonth),
            (None, None, from, to) => Ok(TimeFrame::Between { from, to }),
        }
    }
}

/// The range of events the bill reads: from the period's start on, with
/// no end. So the state holds every entry that may overlap an entry of
/// the bill, also one that starts after the period, and every later
/// billing of its entries.
fn bill_reading(period: &TimeRange) -> PaceResult<TimeRange> {
    TimeRange::new(period.start().cloned(), None)
}

/// The first day of the month `text` names, `YYYY-MM`.
fn parse_month(text: &str) -> Result<Date, String> {
    let shaped = text.len() == 7 && text.as_bytes().get(4) == Some(&b'-');
    shaped
        .then(|| format!("{text}-01").parse::<Date>().ok())
        .flatten()
        .ok_or_else(|| format!("`{text}` is no month; give one as YYYY-MM, such as 2026-11"))
}

/// The client of `--client`, or without it the only client pace knows.
///
/// # Errors
///
/// - A usage error for a key that is no client: it names the closest.
/// - A usage error without `--client` while pace knows several clients:
///   it lists their keys.
/// - Nothing to do while pace knows no client.
fn bill_client<'catalog>(
    catalog: &'catalog Catalog,
    key: Option<&str>,
) -> PaceResult<&'catalog Client> {
    if let Some(text) = key {
        let key: ClientKey = text.parse().map_err(usage)?;
        return catalog.client(&key).ok_or_else(|| {
            let known = catalog.clients().map(|client| client.key.as_str());
            usage(unknown_client(&key, known, None))
        });
    }
    let clients: Vec<&Client> = catalog.clients().collect();
    match clients.as_slice() {
        [only] => Ok(only),
        [] => Err(
            PaceError::new(ErrorKind::InvalidInput, "pace knows no client to bill")
                .attach_hint("`pace client add <key>` adds one")
                .attach_exit_code(ExitCode::NothingToDo),
        ),
        several => {
            let keys: Vec<String> = several
                .iter()
                .map(|client| client.key.to_string())
                .collect();
            Err(usage(
                PaceError::new(
                    ErrorKind::InvalidInput,
                    "pace bills one client at a time, and knows {count}: {keys}",
                )
                .attach_context("count", keys.len().to_string())
                .attach_context("keys", listed(&keys))
                .attach_hint("name one with `--client <key>`"),
            ))
        }
    }
}

/// The error of a bill without a line: nothing to do. Its hint names the
/// running entries the bill leaves out.
fn nothing_to_bill(
    bill: &Bill,
    client: &Client,
    period: BillPeriod,
    state: &State,
) -> Box<PaceError> {
    let running: Vec<String> = bill
        .running
        .iter()
        .filter_map(|id| state.entry(*id))
        .map(|entry| state.entry_name(entry))
        .collect();
    let hint = if running.is_empty() {
        "a bill takes the ended, billable entries that are not billed yet".to_owned()
    } else {
        format!("end {} to bill it", listed(&running))
    };
    PaceError::new(
        ErrorKind::InvalidInput,
        "nothing to bill for {client} {period}",
    )
    .attach_context("client", client.key.to_string())
    .attach_context("period", period.phrase())
    .attach_hint(hint)
    .attach_exit_code(ExitCode::NothingToDo)
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use jiff::civil::date;

    use super::*;
    use crate::commands::{EntryPoint, PaceCmd};

    fn bill(args: &[&str]) -> BillCmd {
        let args = ["pace", "bill"].iter().chain(args).copied();
        match EntryPoint::try_parse_from(args).unwrap().cmd {
            Some(PaceCmd::Bill(bill)) => bill,
            other => panic!("not bill: {other:?}"),
        }
    }

    #[test]
    fn period_defaults_to_last_month() {
        assert_eq!(bill(&[]).frame().unwrap(), TimeFrame::LastMonth);
    }

    #[test]
    fn month_flag_gives_its_first_and_last_day() {
        assert_eq!(
            bill(&["--month", "2026-02"]).frame().unwrap(),
            TimeFrame::Between {
                from: Some(date(2026, 2, 1)),
                to: Some(date(2026, 2, 28)),
            }
        );
    }

    #[test]
    fn month_is_refused_unless_shaped_as_year_and_month() {
        assert_eq!(parse_month("2026-11"), Ok(date(2026, 11, 1)));
        for text in ["2026-13", "2026-1", "2026-11-01", "november"] {
            let error = parse_month(text).unwrap_err();
            assert!(error.contains(&format!("`{text}` is no month")), "{error}");
        }
    }

    #[test]
    fn month_beside_period_or_from_is_refused() {
        let refused: [&[&str]; 2] = [
            &["pace", "bill", "last-month", "--month", "2026-10"],
            &["pace", "bill", "--from", "2026-10-01", "--month", "2026-10"],
        ];
        for args in refused {
            assert!(EntryPoint::try_parse_from(args).is_err(), "{args:?}");
        }
    }

    #[test]
    fn reading_starts_at_period_and_has_no_end() {
        let period = TimeRange::new(
            Some("2026-10-01T00:00[Europe/Berlin]".parse().unwrap()),
            Some("2026-11-01T00:00[Europe/Berlin]".parse().unwrap()),
        )
        .unwrap();

        let reading = bill_reading(&period).unwrap();

        assert_eq!(reading.start(), period.start());
        assert_eq!(reading.end(), None);
    }
}
