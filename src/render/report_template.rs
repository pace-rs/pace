//! The report of `reflect` from a Tera template, for `--format text`,
//! `markdown` and `html`: the built-in templates and the reader of a
//! template file.
//!
//! A template receives the JSON document of the schema, version 1:
//! `version`, `kind` and `rows`. Tera escapes HTML in a template whose
//! name ends in `.html`, `.htm`, `.html.tera` or `.htm.tera`.

use std::path::{Path, PathBuf};

use pace_error::{ErrorKind, PaceError, PaceResult, Snippet};
use serde::Serialize;

/// The template names Tera escapes HTML in.
const HTML_SUFFIXES: [&str; 4] = [".html", ".htm", ".html.tera", ".htm.tera"];

/// A built-in template of the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInTemplate {
    /// Plain text to paste into a mail: no box drawing, no colour.
    Text,
    /// Markdown, with a table per part.
    Markdown,
    /// An HTML page, with a table per part.
    Html,
}

impl BuiltInTemplate {
    /// The template's name, which a file in `reports.templates` takes to
    /// replace it: `reflect.markdown.tera`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Text => "reflect.text.tera",
            Self::Markdown => "reflect.markdown.tera",
            Self::Html => "reflect.html.tera",
        }
    }

    /// The template's source, embedded in the binary.
    #[must_use]
    pub const fn source(self) -> &'static str {
        match self {
            Self::Text => include_str!("../../templates/reflect.text.tera"),
            Self::Markdown => include_str!("../../templates/reflect.markdown.tera"),
            Self::Html => include_str!("../../templates/reflect.html.tera"),
        }
    }
}

/// A template to render the report with: a built-in one, or the text of
/// a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportTemplate {
    /// A template embedded in the binary.
    BuiltIn(BuiltInTemplate),
    /// A template file and its text.
    File {
        /// The file, as an error names it.
        path: PathBuf,
        /// The file's text.
        source: String,
    },
}

impl ReportTemplate {
    /// The template in the file `path`.
    ///
    /// # Errors
    ///
    /// If the file can't be read: the error names it.
    pub fn read(path: &Path) -> PaceResult<Self> {
        let source = std::fs::read_to_string(path).map_err(|error| {
            PaceError::with_source(
                ErrorKind::InputOutput,
                "cannot read the template {path}",
                error,
            )
            .attach_context("path", path.display().to_string())
            .attach_hint("check the path of the template")
        })?;
        Ok(Self::File {
            path: path.to_path_buf(),
            source,
        })
    }

    /// The name Tera knows the template by, and an error names: the
    /// built-in name, or the file's path.
    #[must_use]
    pub fn name(&self) -> String {
        match self {
            Self::BuiltIn(template) => template.name().to_owned(),
            Self::File { path, .. } => path.display().to_string(),
        }
    }

    fn source(&self) -> &str {
        match self {
            Self::BuiltIn(template) => template.source(),
            Self::File { source, .. } => source,
        }
    }
}

/// `document` rendered by `template`.
///
/// # Errors
///
/// If the template has a mistake, or fails to render: the error names the
/// template and the line. A built-in template that fails is a bug.
pub fn render_report_template(
    template: &ReportTemplate,
    document: &impl Serialize,
) -> PaceResult<String> {
    let name = template.name();
    let mut tera = tera::Tera::new();
    tera.autoescape_on(HTML_SUFFIXES);
    let context = tera::Context::from_serialize(document).map_err(|error| {
        PaceError::with_source(
            ErrorKind::Internal,
            "cannot hand the report to a template",
            error,
        )
        .ask_report()
    })?;
    tera.add_raw_template(&name, template.source())
        .and_then(|()| tera.render(&name, &context))
        .map_err(|error| template_error(template, &error))
}

/// The error of `template`: the file, the line and what Tera says.
fn template_error(template: &ReportTemplate, error: &tera::Error) -> Box<PaceError> {
    let failed = match error.kind() {
        tera::ErrorKind::SyntaxError(report) | tera::ErrorKind::RenderingError(report) => {
            located_error(template, report)
        }
        _ => PaceError::new(
            ErrorKind::InvalidInput,
            "the template {template} cannot render: {message}",
        )
        .attach_context("message", error.to_string()),
    }
    .attach_context("template", template.name());
    match template {
        ReportTemplate::BuiltIn(_) => failed.ask_report(),
        ReportTemplate::File { .. } => {
            failed.attach_hint("the user guide's page on reports describes the data of a template")
        }
    }
}

/// The error at the span of `report`: its line, and the template's text
/// around it.
fn located_error(template: &ReportTemplate, report: &tera::ReportError) -> Box<PaceError> {
    PaceError::new(
        ErrorKind::InvalidInput,
        "the template {template} has a mistake on line {line}: {message}",
    )
    .attach_context("line", report.span().start_line.to_string())
    .attach_context("message", report.message())
    .attach_snippet(Snippet::new(
        template.name(),
        template.source(),
        report.span().range.clone(),
        "here",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::schema::{Column, Row, SchemaKind, schema_document};

    fn file(source: &str) -> ReportTemplate {
        ReportTemplate::File {
            path: PathBuf::from("/reports/mine.tera"),
            source: source.to_owned(),
        }
    }

    fn rows() -> Vec<Row> {
        vec![Row::of("time", "tracked")]
    }

    #[test]
    fn template_receives_version_kind_and_rows() {
        let template = file("{{ kind }} {{ version }}: {{ rows | length }} {{ rows[0].group }}");

        let text =
            render_report_template(&template, &schema_document(SchemaKind::Reflect, &rows()))
                .unwrap();

        assert_eq!(text, "reflect 1: 1 tracked");
    }

    #[test]
    fn syntax_mistake_names_file_and_line() {
        let template = file("first line\n{% for row in rows %}\n");

        let error =
            render_report_template(&template, &schema_document(SchemaKind::Reflect, &rows()))
                .unwrap_err();

        let text = error.to_string();
        assert!(
            text.contains("the template /reports/mine.tera has a mistake on line"),
            "{text}"
        );
        assert_eq!(error.snippet_name(), Some("/reports/mine.tera"));
        assert!(error.hint().is_some());
    }

    #[test]
    fn rendering_mistake_names_its_line() {
        let template = file("one\ntwo\n{{ nothing.here }}\n");

        let error =
            render_report_template(&template, &schema_document(SchemaKind::Reflect, &rows()))
                .unwrap_err();

        assert!(error.to_string().contains("on line 3:"), "{error}");
    }

    #[test]
    fn mistake_without_line_names_template() {
        let template = file("{% extends \"base.tera\" %}");

        let error =
            render_report_template(&template, &schema_document(SchemaKind::Reflect, &rows()))
                .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("the template /reports/mine.tera cannot render:"),
            "{error}"
        );
    }

    #[test]
    fn html_template_escapes_text_and_others_do_not() {
        let rows = [Row::of("pauses", "catch").text(Column::Note, "<b>")];
        let document = schema_document(SchemaKind::Reflect, &rows);
        let source = "{{ rows[0].note }}".to_owned();
        let html = ReportTemplate::File {
            path: PathBuf::from("mine.html.tera"),
            source: source.clone(),
        };
        let text = ReportTemplate::File {
            path: PathBuf::from("mine.txt"),
            source,
        };

        assert_eq!(
            render_report_template(&html, &document).unwrap(),
            "&lt;b&gt;"
        );
        assert_eq!(render_report_template(&text, &document).unwrap(), "<b>");
    }

    #[test]
    fn built_in_templates_render_report_without_rows() {
        let document = schema_document(SchemaKind::Reflect, &[]);
        for template in [
            BuiltInTemplate::Text,
            BuiltInTemplate::Markdown,
            BuiltInTemplate::Html,
        ] {
            let rendered = render_report_template(&ReportTemplate::BuiltIn(template), &document);

            assert!(rendered.is_ok(), "{}: {rendered:?}", template.name());
        }
    }
}
