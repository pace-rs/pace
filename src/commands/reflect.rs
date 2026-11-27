//! `pace reflect`: the report of a period, as a table, JSON, CSV, or text,
//! Markdown and HTML from a template.

use std::path::PathBuf;

use abscissa_core::{Command, Runnable};
use jiff::civil::Date;
use pace_core::{Report, ReportComparison, ReportInput};
use pace_error::PaceResult;
use pace_time::{TimeFrame, TimeRange};

use crate::clock::zone;
use crate::commands::report_filter::{ReportFilterArgs, period_frame};
use crate::output::usage;
use crate::print::{print_bytes, print_lines};
use crate::render::{
    BuiltInTemplate, Canvas, ReportRows, ReportTable, ReportTemplate, nothing_tracked,
    render_report_template, report_rows, report_table,
    schema::{Row, SchemaKind, schema_csv, schema_document, schema_json},
};
use crate::{
    application::{Context, PACE_APP, exit_on_error},
    session::Session,
};

/// `pace reflect`
#[derive(clap::Args, Command, Debug)]
pub struct ReflectCmd {
    #[arg(
        value_name = "PERIOD",
        conflicts_with_all = ["from", "to"],
        help = "today, yesterday, this-week (the default), last-week, this-month, \
                last-month, this-year, last-year or all"
    )]
    period: Option<String>,

    #[arg(
        long,
        value_name = "DATE",
        help = "The first day, YYYY-MM-DD; without --to, the report runs to now"
    )]
    from: Option<Date>,

    #[arg(
        long,
        value_name = "DATE",
        help = "The last day, YYYY-MM-DD; without --from, the report starts at the first entry"
    )]
    to: Option<Date>,

    #[command(flatten)]
    filters: ReportFilterArgs,

    #[arg(
        long,
        value_enum,
        default_value_t = ReflectFormat::Table,
        help = "How to show the report"
    )]
    format: ReflectFormat,

    #[arg(
        long,
        value_name = "FILE",
        conflicts_with = "format",
        help = "Render the report with this Tera template; it receives the rows of --format json"
    )]
    template: Option<PathBuf>,

    #[arg(
        long,
        help = "Set the previous period of the same length beside each number, with the change, \
                and name the busiest day and the top project"
    )]
    compare: bool,
}

/// How `pace reflect` shows the report.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflectFormat {
    /// A table for the terminal, with bars.
    Table,
    /// JSON for scripts, in the schema of version 1.
    Json,
    /// CSV for spreadsheets, in the schema of version 1.
    Csv,
    /// Plain text to paste into a mail, from a template.
    Text,
    /// Markdown, from a template.
    Markdown,
    /// HTML, from a template.
    Html,
}

impl ReflectFormat {
    /// The built-in template of a format that comes from one.
    const fn template(self) -> Option<BuiltInTemplate> {
        match self {
            Self::Text => Some(BuiltInTemplate::ReflectText),
            Self::Markdown => Some(BuiltInTemplate::ReflectMarkdown),
            Self::Html => Some(BuiltInTemplate::ReflectHtml),
            Self::Table | Self::Json | Self::Csv => None,
        }
    }
}

impl Runnable for ReflectCmd {
    fn run(&self) {
        exit_on_error(PACE_APP.context().and_then(|context| self.execute(context)));
    }
}

impl ReflectCmd {
    fn execute(&self, context: &Context) -> PaceResult<()> {
        let session = Session::open_existing(context)?;
        let zone = zone(&context.settings.settings.reports_time_zone)?;
        let now = session.now().with_time_zone(zone.clone());
        let week_start = context.settings.settings.week_start;
        let frame = self.frame()?;
        let range = frame.resolve(&now, week_start).map_err(usage)?;
        let previous_range = self
            .compare
            .then(|| frame.resolve_previous(&now, week_start).map_err(usage))
            .transpose()?;
        let state = session.state(&range)?;
        let filter = self.filters.filter(context, &session, &state)?;
        let catalog = &context.catalog;
        let all_time = if Report::needs_all_events(&state, catalog, &filter) {
            Some(session.state(&TimeRange::everything())?)
        } else {
            None
        };
        let input = ReportInput {
            state: &state,
            catalog,
            period: &range,
            filter: &filter,
            now: &now,
            zone: &zone,
            all_time: all_time.as_ref(),
        };
        let report = Report::build(&input)?;
        let comparison = previous_range
            .map(|period| -> PaceResult<ReportComparison> {
                let state = session.state(&period)?;
                let previous = Report::build(&ReportInput {
                    state: &state,
                    period: &period,
                    ..input
                })?;
                Ok(ReportComparison::of(&report, &previous))
            })
            .transpose()?;
        let rows = || {
            report_rows(&ReportRows {
                report: &report,
                comparison: comparison.as_ref(),
                catalog,
                state: &state,
            })
        };
        match (self.report_template(context)?, self.format) {
            (Some(template), _) => print_from_template(&template, &rows()),
            (None, ReflectFormat::Json) => {
                print_lines([schema_json(SchemaKind::Reflect, &rows())?])
            }
            (None, ReflectFormat::Csv) => print_lines(schema_csv(SchemaKind::Reflect, &rows())),
            (None, _) if report.roles.is_empty() => print_lines([nothing_tracked(&frame)]),
            (None, _) => print_lines(report_table(
                &ReportTable {
                    report: &report,
                    comparison: comparison.as_ref(),
                    catalog,
                    state: &state,
                    frame: &frame,
                    now: &now,
                    zone: &zone,
                },
                Canvas::detect(context.plain),
            )),
        }
    }

    /// The template to render the report with: the file of
    /// `--template`, else the template of `--format`, if it has one. A
    /// file of the same name in `reports.templates` replaces the built-in
    /// one.
    fn report_template(&self, context: &Context) -> PaceResult<Option<ReportTemplate>> {
        if let Some(path) = &self.template {
            return ReportTemplate::read(path).map(Some);
        }
        let Some(template) = self.format.template() else {
            return Ok(None);
        };
        let dir = context.settings.settings.reports_templates.as_deref();
        ReportTemplate::from_templates_dir(template, dir).map(Some)
    }

    /// The period: the named one, or `--from` and `--to`; this week
    /// without either.
    fn frame(&self) -> PaceResult<TimeFrame> {
        period_frame(
            self.period.as_deref(),
            self.from,
            self.to,
            TimeFrame::ThisWeek,
        )
    }
}

/// The report's `rows`, rendered by `template` as the document of the
/// schema.
fn print_from_template(template: &ReportTemplate, rows: &[Row]) -> PaceResult<()> {
    let document = schema_document(SchemaKind::Reflect, rows);
    print_bytes(render_report_template(template, &document)?.as_bytes())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;
    use crate::commands::{EntryPoint, PaceCmd};

    fn reflect(args: &[&str]) -> ReflectCmd {
        let args = ["pace", "reflect"].iter().chain(args).copied();
        match EntryPoint::try_parse_from(args).unwrap().cmd {
            Some(PaceCmd::Reflect(reflect)) => reflect,
            other => panic!("not reflect: {other:?}"),
        }
    }

    #[test]
    fn project_flag_repeats_after_reflect() {
        let reflect = reflect(&["--project", "webshop", "--project", "blog"]);

        assert_eq!(reflect.filters.projects, ["webshop", "blog"]);
    }

    #[test]
    fn project_flag_of_reflect_wins_over_global_one() {
        let args = [
            "pace",
            "--project",
            "blog",
            "reflect",
            "--project",
            "webshop",
        ];
        let entry = EntryPoint::try_parse_from(args).unwrap();

        assert_eq!(entry.project.as_deref(), Some("webshop"));
        match entry.cmd {
            Some(PaceCmd::Reflect(reflect)) => assert_eq!(reflect.filters.projects, ["webshop"]),
            other => panic!("not reflect: {other:?}"),
        }
    }

    #[test]
    fn period_defaults_to_this_week_and_from_to_give_days() {
        assert_eq!(reflect(&[]).frame().unwrap(), TimeFrame::ThisWeek);
        assert_eq!(
            reflect(&["--from", "2026-11-02"]).frame().unwrap(),
            TimeFrame::Between {
                from: Some(jiff::civil::date(2026, 11, 2)),
                to: None,
            }
        );
    }

    #[test]
    fn period_beside_from_is_refused() {
        let args = ["pace", "reflect", "today", "--from", "2026-11-02"];

        assert!(EntryPoint::try_parse_from(args).is_err());
    }
}
