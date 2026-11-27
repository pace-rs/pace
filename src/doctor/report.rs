//! The findings of `pace doctor` as text, one block per finding and a
//! last line that counts them, and as rows of the schema.

use crate::doctor::{Finding, FindingSeverity};
use crate::render::schema::{Column, Row};

/// The text of `findings`: each as a line `error[files]: message`, then
/// the place and the fix, indented; then a line that counts them, or
/// "No problems found." without any.
#[must_use]
pub fn doctor_lines(findings: &[Finding]) -> Vec<String> {
    let mut lines = Vec::new();
    for finding in findings {
        lines.push(format!(
            "{}[{}]: {}",
            finding.severity.name(),
            finding.area.name(),
            finding.message
        ));
        if let Some(place) = place(finding) {
            lines.push(format!("  at {place}"));
        }
        if let Some(fix) = &finding.fix {
            lines.push(format!("  fix: {fix}"));
        }
        lines.push(String::new());
    }
    lines.push(count_line(findings));
    lines
}

/// Where a finding is: its file and line, or its entry.
fn place(finding: &Finding) -> Option<String> {
    let file = finding.file.as_ref().map(|file| {
        finding.line.map_or_else(
            || file.display().to_string(),
            |line| format!("{}:{line}", file.display()),
        )
    });
    match (file, &finding.entry) {
        (Some(file), Some(entry)) => Some(format!("{file}, entry {entry}")),
        (Some(file), None) => Some(file),
        (None, Some(entry)) => Some(format!("entry {entry}")),
        (None, None) => None,
    }
}

/// The last line: how many errors and warnings doctor found.
fn count_line(findings: &[Finding]) -> String {
    let count = |severity| {
        findings
            .iter()
            .filter(|finding| finding.severity == severity)
            .count()
    };
    let counted = [
        (count(FindingSeverity::Error), "error", "errors"),
        (count(FindingSeverity::Warning), "warning", "warnings"),
    ]
    .into_iter()
    .filter(|(count, _, _)| *count > 0)
    .map(|(count, one, more)| format!("{count} {}", if count == 1 { one } else { more }))
    .collect::<Vec<_>>();
    if counted.is_empty() {
        "No problems found.".to_owned()
    } else {
        format!("Found {}.", counted.join(" and "))
    }
}

/// `findings` as rows of the schema's kind `doctor`: `severity`, `area`,
/// `message`, and `file`, `line`, `entry` and `fix` where a finding has
/// them.
#[must_use]
pub fn doctor_rows(findings: &[Finding]) -> Vec<Row> {
    findings
        .iter()
        .map(|finding| {
            let row = Row::default()
                .text(Column::Severity, finding.severity.name())
                .text(Column::Area, finding.area.name())
                .text(Column::Message, finding.message.clone())
                .maybe_text(
                    Column::File,
                    finding.file.as_ref().map(|file| file.display().to_string()),
                )
                .maybe_text(Column::Entry, finding.entry.clone())
                .maybe_text(Column::Fix, finding.fix.clone());
            match finding.line {
                Some(line) => row.number(Column::Line, i64::try_from(line).unwrap_or(i64::MAX)),
                None => row,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::doctor::FindingArea;

    fn event_mistake() -> Finding {
        Finding {
            line: Some(3),
            ..Finding::new(
                FindingSeverity::Error,
                FindingArea::Files,
                "pace doesn't know the event type `entry.bogun`",
            )
            .in_file(Path::new("2026-11.toml"))
            .with_fix("update pace")
        }
    }

    #[test]
    fn no_finding_is_no_problem() {
        assert_eq!(doctor_lines(&[]), ["No problems found."]);
    }

    #[test]
    fn each_finding_names_its_place_and_fix_and_the_last_line_counts_them() {
        let setup = Finding::new(
            FindingSeverity::Warning,
            FindingArea::Setup,
            "a setup waits",
        );

        let lines = doctor_lines(&[event_mistake(), setup]);

        assert_eq!(
            lines,
            [
                "error[files]: pace doesn't know the event type `entry.bogun`",
                "  at 2026-11.toml:3",
                "  fix: update pace",
                "",
                "warning[setup]: a setup waits",
                "",
                "Found 1 error and 1 warning.",
            ]
        );
    }

    #[test]
    fn several_findings_of_kind_are_counted_in_plural() {
        let warning = Finding::new(FindingSeverity::Warning, FindingArea::Setup, "a");

        let lines = doctor_lines(&[event_mistake(), event_mistake(), warning]);

        assert_eq!(
            lines.last().map(String::as_str),
            Some("Found 2 errors and 1 warning.")
        );
    }

    #[test]
    fn entry_is_named_beside_its_file_or_alone() {
        let with_file = Finding {
            entry: Some("ad5e".to_owned()),
            ..event_mistake()
        };
        let alone = Finding {
            entry: Some("ad5e".to_owned()),
            ..Finding::new(FindingSeverity::Warning, FindingArea::Files, "x")
        };

        assert_eq!(
            place(&with_file).as_deref(),
            Some("2026-11.toml:3, entry ad5e")
        );
        assert_eq!(place(&alone).as_deref(), Some("entry ad5e"));
    }

    #[test]
    fn row_has_cells_only_for_what_finding_names() {
        let rows = doctor_rows(&[event_mistake()]);

        let columns: Vec<Column> = rows[0].columns().collect();
        assert_eq!(
            columns,
            [
                Column::Severity,
                Column::Area,
                Column::Message,
                Column::File,
                Column::Line,
                Column::Fix
            ]
        );
    }
}
