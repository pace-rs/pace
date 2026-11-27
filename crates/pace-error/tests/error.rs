//! The contract of [`PaceError`]: what a user and a script get to see.

use std::{backtrace::BacktraceStatus, error::Error as _};

use miette::{Diagnostic, NarratableReportHandler};
use pace_error::{ErrorKind, ExitCode, PaceError, Severity, Snippet};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn help(error: &PaceError) -> String {
    error
        .help()
        .map_or_else(String::new, |help| help.to_string())
}

fn narrate(error: &PaceError) -> Result<String, std::fmt::Error> {
    let mut out = String::new();
    NarratableReportHandler::new().render_report(&mut out, error)?;
    Ok(out)
}

#[test]
fn guidance_placeholders_are_filled_from_the_context() {
    let error = PaceError::new(ErrorKind::Project, "found {visible} and {hidden} in {dir}")
        .attach_context("dir", "/work/acme")
        .attach_context("visible", "pace.toml")
        .attach_context("hidden", ".pace.toml");

    assert_eq!(
        error.to_string(),
        "found pace.toml and .pace.toml in /work/acme"
    );
}

#[test]
fn placeholder_in_context_value_stays_as_written() {
    let error = PaceError::new(
        ErrorKind::Directories,
        "the machine directory {machine} lies inside {data}",
    )
    .attach_context("machine", "/s/home/{data}")
    .attach_context("data", "/s/home")
    .attach_context("reason", "{machine} is synced");

    assert_eq!(
        error.to_string(),
        "the machine directory /s/home/{data} lies inside /s/home"
    );
    assert_eq!(help(&error), "Details:\n- reason: {machine} is synced");
}

#[test]
fn placeholder_without_context_stays_as_written() {
    let error =
        PaceError::new(ErrorKind::Other, "keep {unknown} and {data").attach_context("data", "/s");

    assert_eq!(error.to_string(), "keep {unknown} and {data");
}

#[test]
fn the_hint_is_the_help_of_the_diagnostic() {
    let error = PaceError::new(ErrorKind::Other, "nothing is running")
        .attach_hint("`pace begin` starts an entry");

    assert_eq!(help(&error), "`pace begin` starts an entry");
}

#[test]
fn context_left_out_of_the_guidance_is_listed_as_details() {
    let error = PaceError::new(ErrorKind::Configuration, "cannot read {file}")
        .attach_hint("check the file permissions")
        .attach_context("file", "/c/pace.global.toml")
        .attach_context("reason", "permission denied");

    let help = help(&error);

    assert!(help.starts_with("check the file permissions"), "{help}");
    assert!(help.contains("reason: permission denied"), "{help}");
    assert!(!help.contains("file:"), "{help}");
}

#[test]
fn guidance_lines_can_be_added_before_and_after() {
    let error = PaceError::new(ErrorKind::Other, "middle")
        .append_guidance_line("last")
        .prepend_guidance_line("first");

    assert_eq!(error.to_string(), "first\nmiddle\nlast");
}

#[test]
fn errors_exit_with_the_error_code_unless_told_otherwise() {
    let plain = PaceError::new(ErrorKind::Other, "it failed");
    let idle = PaceError::new(ErrorKind::Other, "nothing is running")
        .attach_exit_code(ExitCode::NothingToDo);

    assert_eq!(plain.exit_code(), ExitCode::Error);
    assert_eq!(idle.exit_code(), ExitCode::NothingToDo);
}

#[test]
fn exit_codes_are_the_documented_numbers_for_scripts() {
    let codes = [
        ExitCode::Done,
        ExitCode::Error,
        ExitCode::Usage,
        ExitCode::NothingToDo,
    ];

    assert_eq!(codes.map(i32::from), [0, 1, 2, 3]);
}

#[test]
fn the_error_code_is_the_code_of_the_diagnostic() {
    let error = PaceError::new(ErrorKind::Configuration, "unknown key")
        .attach_error_code("pace::config::unknown-key");

    assert_eq!(
        error.code().map(|code| code.to_string()).as_deref(),
        Some("pace::config::unknown-key")
    );
    assert!(error.is_code("pace::config::unknown-key"));
    assert!(!error.is_code("pace::config"));
}

#[test]
fn the_kind_can_be_overwritten_on_the_way_up() {
    let error = PaceError::new(ErrorKind::InputOutput, "cannot read the manifest")
        .overwrite_kind(ErrorKind::Project);

    assert_eq!(error.kind(), ErrorKind::Project);
}

#[test]
fn the_source_is_kept_as_the_cause() {
    let io = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
    let error = PaceError::with_source(ErrorKind::InputOutput, "cannot read {file}", io)
        .attach_context("file", "pace.toml");

    assert_eq!(
        error.source().map(ToString::to_string).as_deref(),
        Some("no such file")
    );
}

#[test]
fn context_value_can_be_looked_up() {
    let error = PaceError::new(ErrorKind::Project, "no project").attach_context("dir", "/w");

    assert_eq!(error.context_value("dir"), Some("/w"));
    assert_eq!(error.context_value("file"), None);
}

#[test]
fn bug_asks_to_be_reported_at_the_issue_tracker() {
    let error = PaceError::new(ErrorKind::Internal, "the event log is out of order").ask_report();

    assert!(
        help(&error).contains("https://github.com/pace-rs/pace/issues/new"),
        "{}",
        help(&error)
    );
}

#[test]
fn severities_map_onto_the_diagnostic() {
    let severity = |severity| {
        PaceError::new(ErrorKind::Other, "x")
            .attach_severity(severity)
            .severity()
    };

    assert_eq!(severity(Severity::Info), Some(miette::Severity::Advice));
    assert_eq!(severity(Severity::Warning), Some(miette::Severity::Warning));
    assert_eq!(severity(Severity::Error), Some(miette::Severity::Error));
    assert_eq!(severity(Severity::Fatal), Some(miette::Severity::Error));
}

#[test]
fn snippet_points_at_the_offending_line() -> TestResult {
    let source = "version = 1\ntime-zon = \"Europe/Berlin\"\n";
    let error = PaceError::new(ErrorKind::Configuration, "unknown key `time-zon`")
        .attach_hint("did you mean `time-zone`?")
        .attach_snippet(Snippet::new(
            "pace.global.toml",
            source,
            12..20,
            "not a key pace knows",
        ));

    let report = narrate(&error)?;

    assert!(report.contains("pace.global.toml"), "{report}");
    assert!(
        report.contains("label at line 2, columns 1 to 8: not a key pace knows"),
        "{report}"
    );
    assert!(report.contains("did you mean `time-zone`?"), "{report}");
    Ok(())
}

#[test]
fn docs_url_is_url_of_diagnostic() {
    let error = PaceError::new(ErrorKind::Time, "pace doesn't know the time zone")
        .attach_docs_url("https://pace.cli.rs/docs/time-zones.html");

    assert_eq!(
        error.url().map(|url| url.to_string()).as_deref(),
        Some("https://pace.cli.rs/docs/time-zones.html")
    );
}

#[test]
fn existing_issues_are_listed_as_related_issues() {
    let error = PaceError::new(ErrorKind::InputOutput, "the lock is held")
        .attach_existing_issue_url("https://github.com/pace-rs/pace/issues/1")
        .attach_existing_issue_url("https://github.com/pace-rs/pace/issues/2");

    assert_eq!(
        help(&error),
        "Related issues:\n\
         - https://github.com/pace-rs/pace/issues/1\n\
         - https://github.com/pace-rs/pace/issues/2"
    );
}

#[test]
fn bug_report_goes_to_new_issue_url_when_one_is_given() {
    let error = PaceError::new(ErrorKind::Internal, "the event log is out of order")
        .ask_report()
        .attach_new_issue_url("https://example.org/pace/new");

    assert_eq!(
        help(&error),
        "This is a bug in pace. Please report it at https://example.org/pace/new, \
         with the command you ran."
    );
}

/// Runs `backtrace_follows_rust_backtrace` in a new process of this test
/// binary, with `RUST_BACKTRACE` set to `value`, since std reads it once per
/// process.
fn backtrace_test_passes_with(value: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let output = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", "backtrace_follows_rust_backtrace"])
        .env("RUST_BACKTRACE", value)
        .env_remove("RUST_LIB_BACKTRACE")
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    Ok(output.status.success() && stdout.contains("1 passed"))
}

#[test]
fn backtrace_follows_rust_backtrace() {
    let asked = std::env::var("RUST_LIB_BACKTRACE")
        .or_else(|_| std::env::var("RUST_BACKTRACE"))
        .is_ok_and(|value| value != "0");
    let expected = if asked {
        BacktraceStatus::Captured
    } else {
        BacktraceStatus::Disabled
    };

    let error = PaceError::new(ErrorKind::Other, "x");

    assert_eq!(error.backtrace().status(), expected);
}

#[test]
fn backtrace_is_captured_only_when_rust_backtrace_asks() -> TestResult {
    assert!(backtrace_test_passes_with("1")?, "with RUST_BACKTRACE=1");
    assert!(backtrace_test_passes_with("0")?, "with RUST_BACKTRACE=0");
    Ok(())
}
