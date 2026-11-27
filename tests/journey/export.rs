//! `pace export`: every entry of a period, one row each, in JSON and CSV;
//! a billed entry with its frozen amount, a running one up to now; the
//! filters and the periods of reflect.

use crate::support::{Sandbox, run};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ACME: &str = r#"version = 1

[acme]
name = "ACME GmbH"
currency = "EUR"
rounding = { per = "entry", step = "15m", mode = "up" }

[[acme.rates]]
hourly = 95
from = 2025-01-01

[[acme.projects.acme-webshop.rates]]
hourly = 120
from = 2026-09-01
"#;

const WORK: &str = r#"version = 1

[work]
id = "0193a7c4-0000-7000-8000-0000000000a1"
kind = "area"
name = "Work"
"#;

const WEBSHOP: &str = r#"version = 1
id = "0193a7c4-0000-7000-8000-000000000001"
key = "acme-webshop"
name = "Webshop relaunch"
client = "acme"
role = "work"
billable = true
"#;

const BLOG: &str = r#"version = 1
id = "0193a7c4-0000-7000-8000-000000000002"
key = "acme-blog"
name = "Company blog"
client = "acme"
role = "work"
billable = true
"#;

/// The client `acme` (95 EUR an hour, 120 on the webshop, rounded up to
/// 15 minutes per entry), the role `work`, the projects `acme-webshop`
/// and `acme-blog`, reports in Berlin; and this history:
///
/// - October: a planning entry and a fix with a hold on the webshop,
///   billed on the invoice `2026-0042` on 3 November;
/// - 20 November: a post on the blog, not billed yet, and a lunch without
///   a project;
/// - 26 November: an offer on the blog, running since 09:00.
fn history_sandbox() -> Result<Sandbox, Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    sandbox.write("home/clients.toml", ACME)?;
    sandbox.write("home/roles.toml", WORK)?;
    sandbox.write("home/projects/acme-webshop.toml", WEBSHOP)?;
    sandbox.write("home/projects/acme-blog.toml", BLOG)?;
    sandbox.write_config("[reports]\ntime-zone = \"Europe/Berlin\"\n")?;
    let steps: [(&str, &[&str]); 9] = [
        (
            "2026-10-05T12:00[Europe/Berlin]",
            &[
                "log",
                "Plan the relaunch",
                "--project",
                "acme-webshop",
                "--from",
                "2026-10-05 09:00",
                "--to",
                "2026-10-05 11:10",
                "--by-me",
            ],
        ),
        (
            "2026-10-07T09:00[Europe/Berlin]",
            &["begin", "Fix the checkout", "--project", "acme-webshop"],
        ),
        (
            "2026-10-07T10:00[Europe/Berlin]",
            &["hold", "--outer", "--reason", "phone call"],
        ),
        ("2026-10-07T10:30[Europe/Berlin]", &["resume"]),
        ("2026-10-07T12:00[Europe/Berlin]", &["end"]),
        (
            "2026-11-03T09:00[Europe/Berlin]",
            &["bill", "--mark-billed", "2026-0042"],
        ),
        (
            "2026-11-20T16:00[Europe/Berlin]",
            &[
                "log",
                "Write the launch post",
                "--project",
                "acme-blog",
                "--from",
                "2026-11-20 14:00",
                "--to",
                "2026-11-20 15:20",
                "--tag",
                "writing",
            ],
        ),
        (
            "2026-11-20T16:00[Europe/Berlin]",
            &[
                "log",
                "Team lunch",
                "--from",
                "2026-11-20 12:00",
                "--to",
                "2026-11-20 13:00",
                "--by-others",
            ],
        ),
        (
            "09:00",
            &["begin", "Write the offer", "--project", "acme-blog"],
        ),
    ];
    for (time, args) in steps {
        _ = run(sandbox.pace_at(time).args(args))?;
    }
    Ok(sandbox)
}

/// The lines of the CSV in `output`'s stdout, without the header.
fn csv_rows(output: &str) -> Vec<&str> {
    output
        .lines()
        .skip_while(|line| !line.starts_with("id,"))
        .skip(1)
        .take_while(|line| !line.is_empty())
        .collect()
}

#[test]
fn export_writes_every_entry_as_json() -> TestResult {
    let sandbox = history_sandbox()?;

    let output = sandbox.journey(&[("15:00", &["export", "--format", "json"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn export_writes_every_entry_as_csv() -> TestResult {
    let sandbox = history_sandbox()?;

    let output = sandbox.journey(&[("15:00", &["export", "--format", "csv"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn export_without_format_writes_json() -> TestResult {
    let sandbox = history_sandbox()?;

    let bare = run(sandbox.pace_at("15:00").args(["export"]))?;
    let json = run(sandbox
        .pace_at("15:00")
        .args(["export", "--format", "json"]))?;

    assert!(bare.contains("\"kind\": \"export\""), "{bare}");
    assert_eq!(bare, json);
    Ok(())
}

#[test]
fn export_filter_keeps_entries_of_project() -> TestResult {
    let sandbox = history_sandbox()?;

    let output = run(sandbox.pace_at("15:00").args([
        "export",
        "--project",
        "acme-blog",
        "--format",
        "csv",
    ]))?;

    let rows = csv_rows(&output);
    assert_eq!(rows.len(), 2, "{output}");
    assert!(rows[0].contains("Write the launch post"), "{output}");
    assert!(rows[1].contains("Write the offer"), "{output}");
    Ok(())
}

#[test]
fn export_period_takes_entries_that_start_in_it() -> TestResult {
    let sandbox = history_sandbox()?;

    let output = run(sandbox
        .pace_at("15:00")
        .args(["export", "last-month", "--format", "csv"]))?;

    let rows = csv_rows(&output);
    assert_eq!(rows.len(), 2, "{output}");
    assert!(rows[0].contains("Plan the relaunch"), "{output}");
    assert!(rows[1].contains("Fix the checkout"), "{output}");
    Ok(())
}

#[test]
fn export_of_empty_period_writes_header_only() -> TestResult {
    let sandbox = history_sandbox()?;

    let output = run(sandbox.pace_at("15:00").args([
        "export",
        "--from",
        "2026-09-01",
        "--to",
        "2026-09-30",
        "--format",
        "csv",
    ]))?;

    assert!(output.starts_with("success: true"), "{output}");
    assert!(output.contains("\nid,short,"), "{output}");
    assert!(csv_rows(&output).is_empty(), "{output}");
    Ok(())
}
