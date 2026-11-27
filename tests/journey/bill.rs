//! `pace bill`: the bill of a client over a period, its formats, the
//! choice of the client, and the refusals.

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

const BETA: &str = r#"
[beta]
name = "Beta & Sons"
currency = "EUR"

[[beta.rates]]
hourly = 80
from = 2026-10-15
"#;

const WEBSHOP: &str = r#"version = 1
id = "0193a7c4-0000-7000-8000-000000000001"
key = "acme-webshop"
name = "Webshop relaunch"
client = "acme"
billable = true
"#;

const BLOG: &str = r#"version = 1
id = "0193a7c4-0000-7000-8000-000000000002"
key = "acme-blog"
name = "Company blog"
client = "acme"
billable = true
"#;

const BETA_SITE: &str = r#"version = 1
id = "0193a7c4-0000-7000-8000-000000000003"
key = "beta-site"
name = "Beta's site"
client = "beta"
billable = true
"#;

/// A sandbox with the client `acme` (95 EUR an hour, 120 on the webshop,
/// rounded up to 15 minutes per entry), its projects `acme-webshop` and
/// `acme-blog`, and reports in Berlin; with `beta`, also the client `beta`
/// (80 EUR an hour from 2026-10-15) and its project `beta-site`.
fn catalog_sandbox(beta: bool) -> Result<Sandbox, Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    let clients = if beta {
        format!("{ACME}{BETA}")
    } else {
        ACME.to_owned()
    };
    sandbox.write("home/clients.toml", &clients)?;
    sandbox.write("home/projects/acme-webshop.toml", WEBSHOP)?;
    sandbox.write("home/projects/acme-blog.toml", BLOG)?;
    if beta {
        sandbox.write("home/projects/beta-site.toml", BETA_SITE)?;
    }
    sandbox.write_config("[reports]\ntime-zone = \"Europe/Berlin\"\n")?;
    Ok(sandbox)
}

/// `HH:MM` on `day` of October 2026 in Berlin, for [`Sandbox::pace_at`].
fn october(day: u8, time: &str) -> String {
    format!("2026-10-{day:02}T{time}[Europe/Berlin]")
}

/// October 2026 for acme: a planning entry and a fix with a hold on the
/// webshop, a post on the blog, a lunch without a project, and an offer
/// on the blog that still runs.
fn october_sandbox() -> Result<Sandbox, Box<dyn std::error::Error>> {
    let sandbox = catalog_sandbox(false)?;
    let steps: [(String, &[&str]); 8] = [
        (
            october(5, "12:00"),
            &[
                "log",
                "Plan the relaunch",
                "--project",
                "acme-webshop",
                "--from",
                "2026-10-05 09:00",
                "--to",
                "2026-10-05 11:10",
            ],
        ),
        (
            october(7, "09:00"),
            &["begin", "Fix the checkout", "--project", "acme-webshop"],
        ),
        (
            october(7, "10:00"),
            &["hold", "--outer", "--reason", "phone call"],
        ),
        (october(7, "10:30"), &["resume"]),
        (october(7, "12:00"), &["end"]),
        (
            october(20, "16:00"),
            &[
                "log",
                "Write the launch post",
                "--project",
                "acme-blog",
                "--from",
                "2026-10-20 14:00",
                "--to",
                "2026-10-20 15:20",
            ],
        ),
        (
            october(20, "16:00"),
            &[
                "log",
                "Team lunch",
                "--from",
                "2026-10-20 12:00",
                "--to",
                "2026-10-20 13:00",
            ],
        ),
        (
            october(30, "16:00"),
            &["begin", "Write the offer", "--project", "acme-blog"],
        ),
    ];
    for (time, args) in steps {
        _ = run(sandbox.pace_at(&time).args(args))?;
    }
    Ok(sandbox)
}

#[test]
fn bill_shows_last_month_per_project_as_table() -> TestResult {
    let sandbox = october_sandbox()?;

    let output = sandbox.journey(&[("15:00", &["bill", "--client", "acme"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn bill_shows_month_as_json() -> TestResult {
    let sandbox = october_sandbox()?;

    let output = sandbox.journey(&[("15:00", &["bill", "--format", "json"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn bill_shows_month_as_csv() -> TestResult {
    let sandbox = october_sandbox()?;

    let output = sandbox.journey(&[("15:00", &["bill", "--format", "csv"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn bill_shows_month_as_markdown() -> TestResult {
    let sandbox = october_sandbox()?;

    let output = sandbox.journey(&[("15:00", &["bill", "--format", "markdown"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn templates_directory_replaces_bill_template_by_name() -> TestResult {
    let sandbox = october_sandbox()?;
    sandbox.write(
        "templates/bill.markdown.tera",
        "Own bill: {{ kind }}, {{ rows | length }} rows\n",
    )?;
    let templates = sandbox.path("templates");
    sandbox.write_config(&format!(
        "[reports]\ntime-zone = \"Europe/Berlin\"\ntemplates = {:?}\n",
        templates.display().to_string()
    ))?;

    let output = run(sandbox
        .pace_at("15:00")
        .args(["bill", "--format", "markdown"]))?;

    assert!(output.contains("Own bill: bill, "), "{output}");
    Ok(())
}

#[test]
fn single_client_makes_client_flag_optional() -> TestResult {
    let sandbox = october_sandbox()?;

    let named = run(sandbox.pace_at("15:00").args(["bill", "--client", "acme"]))?;
    let bare = run(sandbox.pace_at("15:00").args(["bill"]))?;

    assert!(named.starts_with("success: true"), "{named}");
    assert_eq!(bare, named);
    Ok(())
}

#[test]
fn two_clients_without_client_flag_exit_two_and_list_them() -> TestResult {
    let sandbox = catalog_sandbox(true)?;

    let output = sandbox.journey(&[("15:00", &["bill"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn unknown_client_is_usage_error_naming_closest() -> TestResult {
    let sandbox = catalog_sandbox(true)?;

    let output = sandbox.journey(&[("15:00", &["bill", "--client", "amce"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn month_flag_picks_calendar_month() -> TestResult {
    let sandbox = october_sandbox()?;

    let month = run(sandbox
        .pace_at("15:00")
        .args(["bill", "--month", "2026-10"]))?;
    let last_month = run(sandbox.pace_at("15:00").args(["bill", "last-month"]))?;
    let between =
        run(sandbox
            .pace_at("15:00")
            .args(["bill", "--from", "2026-10-01", "--to", "2026-10-31"]))?;

    assert_eq!(month, last_month);
    assert!(between.contains("Plan the relaunch"), "{between}");
    Ok(())
}

#[test]
fn month_flag_refuses_text_that_is_no_month() -> TestResult {
    let sandbox = october_sandbox()?;

    let output = sandbox.journey(&[("15:00", &["bill", "--month", "2026-13"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

/// Log an entry on a second device with the same catalog, whose home is
/// `other-home`, and copy its events into this sandbox: a sync. `log`
/// refuses an overlap on one device, so a sync is how one comes in.
fn log_on_other_device(sandbox: &Sandbox, args: &[&str]) -> TestResult {
    sandbox.write("other-home/clients.toml", ACME)?;
    sandbox.write("other-home/projects/acme-webshop.toml", WEBSHOP)?;
    sandbox.write("other-home/projects/acme-blog.toml", BLOG)?;
    let output = run(sandbox
        .pace_at("15:00")
        .env("PACE_HOME", sandbox.path("other-home"))
        .env("PACE_MACHINE_DIR", sandbox.path("other-machine"))
        .arg("log")
        .args(args))?;
    assert!(output.starts_with("success: true"), "{output}");
    sandbox.sync_events_from("other-home")?;
    Ok(())
}

#[test]
fn overlap_is_refused_with_entries_and_fix() -> TestResult {
    let sandbox = catalog_sandbox(false)?;
    _ = run(sandbox.pace_at("15:00").args([
        "log",
        "Plan the relaunch",
        "--project",
        "acme-webshop",
        "--from",
        "2026-10-05 09:00",
        "--to",
        "2026-10-05 10:00",
    ]))?;
    log_on_other_device(
        &sandbox,
        &[
            "Write the post",
            "--project",
            "acme-blog",
            "--from",
            "2026-10-05 09:30",
            "--to",
            "2026-10-05 10:30",
        ],
    )?;

    let output = sandbox.journey(&[("15:00", &["bill"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn overlap_with_entry_after_period_is_refused() -> TestResult {
    let sandbox = catalog_sandbox(false)?;
    _ = run(sandbox.pace_at("15:00").args([
        "log",
        "Deploy at night",
        "--project",
        "acme-webshop",
        "--from",
        "2026-10-31 23:00",
        "--to",
        "2026-11-01 01:00",
    ]))?;
    log_on_other_device(
        &sandbox,
        &[
            "Watch the logs",
            "--project",
            "acme-blog",
            "--from",
            "2026-11-01 00:30",
            "--to",
            "2026-11-01 02:00",
        ],
    )?;

    let output = run(sandbox.pace_at("15:00").args(["bill"]))?;

    assert!(output.contains("exit_code: 1"), "{output}");
    assert!(output.contains("these entries overlap"), "{output}");
    Ok(())
}

#[test]
fn entry_before_first_rate_is_refused() -> TestResult {
    let sandbox = catalog_sandbox(true)?;

    let output = sandbox.journey(&[
        (
            "15:00",
            &[
                "log",
                "Draft the sitemap",
                "--project",
                "beta-site",
                "--from",
                "2026-10-10 09:00",
                "--to",
                "2026-10-10 10:00",
            ],
        ),
        ("15:00", &["bill", "--client", "beta"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn nothing_to_bill_exits_three() -> TestResult {
    let sandbox = catalog_sandbox(false)?;

    let output = sandbox.journey(&[
        (
            "15:00",
            &[
                "log",
                "Plan the relaunch",
                "--project",
                "acme-webshop",
                "--from",
                "2026-11-05 09:00",
                "--to",
                "2026-11-05 10:00",
            ],
        ),
        ("15:00", &["bill"]),
        ("15:00", &["bill", "--format", "json"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn nothing_to_bill_but_running_entry_names_it() -> TestResult {
    let sandbox = catalog_sandbox(false)?;

    let output = sandbox.journey(&[
        (
            "2026-10-30T16:00[Europe/Berlin]",
            &["begin", "Write the offer", "--project", "acme-blog"],
        ),
        ("15:00", &["bill"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn no_client_is_nothing_to_bill() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[("15:00", &["bill"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

/// The `billing.marked` events of October 2026, as the event files hold
/// them: each is filed under its entry's start.
fn october_billings(sandbox: &Sandbox) -> Result<String, Box<dyn std::error::Error>> {
    let mut text = String::new();
    for device in std::fs::read_dir(sandbox.path("home/events"))? {
        let file = device?.path().join("2026/2026-10.toml");
        if file.is_file() {
            let events = std::fs::read_to_string(file)?;
            for event in events.split("[[event]]\n") {
                if event.contains("type = \"billing.marked\"") {
                    text.push_str("[[event]]\n");
                    text.push_str(event);
                }
            }
        }
    }
    Ok(text)
}

#[test]
fn mark_billed_freezes_bill_and_leaves_nothing_to_bill() -> TestResult {
    let sandbox = october_sandbox()?;

    let transcript = sandbox.transcript_in(
        "",
        &[
            ("15:00", &["bill", "--mark-billed", "2026-0042"]),
            ("15:05", &["bill"]),
        ],
    )?;
    let output = sandbox.redact_ids(&format!(
        "{transcript}--- billing.marked in home/events ---\n{}",
        october_billings(&sandbox)?
    ))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn undo_takes_marking_back() -> TestResult {
    let sandbox = october_sandbox()?;

    let output = sandbox.journey(&[
        ("15:00", &["bill", "--mark-billed", "2026-0042"]),
        ("15:05", &["undo"]),
        ("15:10", &["bill", "--format", "csv"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn invoice_reference_of_client_is_refused_second_time() -> TestResult {
    let sandbox = october_sandbox()?;

    let output = sandbox.journey(&[
        ("15:00", &["bill", "--mark-billed", "2026-0042"]),
        (
            "15:05",
            &[
                "log",
                "Fix the cart",
                "--project",
                "acme-webshop",
                "--from",
                "2026-10-21 09:00",
                "--to",
                "2026-10-21 10:00",
            ],
        ),
        ("15:10", &["bill", "--mark-billed", "2026-0042"]),
        ("15:15", &["bill", "--mark-billed", "2026-0043"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn mark_billed_with_nothing_to_bill_exits_three() -> TestResult {
    let sandbox = catalog_sandbox(false)?;

    let output = run(sandbox
        .pace_at("15:00")
        .args(["bill", "--mark-billed", "2026-0042"]))?;

    assert!(output.contains("exit_code: 3"), "{output}");
    assert!(
        output.contains("nothing to bill for acme in October 2026"),
        "{output}"
    );
    Ok(())
}

#[test]
fn mark_billed_refuses_format_and_blank_reference_as_usage() -> TestResult {
    let sandbox = october_sandbox()?;

    let output = sandbox.journey(&[
        (
            "15:00",
            &["bill", "--mark-billed", "2026-0042", "--format", "json"],
        ),
        ("15:00", &["bill", "--mark-billed", " "]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

/// An entry that is open and billed: it ended on this device, which
/// syncs it to a second device with the same catalog. That device marks
/// October billed, and this one takes back the end before it syncs the
/// marking in, with `--change-billed`, for the end is of a billed entry.
fn open_billed_entry(sandbox: &Sandbox) -> TestResult {
    for (time, args) in [
        (
            october(7, "09:00"),
            &["begin", "Fix the checkout", "--project", "acme-webshop"][..],
        ),
        (october(7, "12:00"), &["end"][..]),
    ] {
        _ = run(sandbox.pace_at(&time).args(args))?;
    }
    sandbox.sync_events_to("other-home")?;
    sandbox.write("other-home/clients.toml", ACME)?;
    sandbox.write("other-home/projects/acme-webshop.toml", WEBSHOP)?;
    let marked = run(sandbox
        .pace_at("14:00")
        .env("PACE_HOME", sandbox.path("other-home"))
        .env("PACE_MACHINE_DIR", sandbox.path("other-machine"))
        .args(["bill", "--mark-billed", "2026-0042"]))?;
    assert!(marked.starts_with("success: true"), "{marked}");
    sandbox.sync_events_from("other-home")?;
    let undone = run(sandbox.pace_at("14:30").args(["undo", "--change-billed"]))?;
    assert!(undone.starts_with("success: true"), "{undone}");
    Ok(())
}

#[test]
fn now_shows_invoice_of_billed_entry() -> TestResult {
    let sandbox = catalog_sandbox(false)?;
    open_billed_entry(&sandbox)?;

    let output = sandbox.journey(&[("15:00", &["now"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn now_json_names_invoice_of_billed_entry() -> TestResult {
    let sandbox = catalog_sandbox(false)?;
    open_billed_entry(&sandbox)?;

    let output = run(sandbox.pace_at("15:00").args(["now", "--json"]))?;

    assert!(output.contains(r#""billed": "2026-0042""#), "{output}");
    Ok(())
}

#[test]
fn mark_billed_rounds_days_of_reports_time_zone() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write(
        "home/clients.toml",
        "version = 1\n\n[acme]\nname = \"ACME GmbH\"\ncurrency = \"EUR\"\n\
         rounding = { per = \"day\", step = \"30m\", mode = \"up\" }\n\n\
         [[acme.rates]]\nhourly = 60\nfrom = 2025-01-01\n",
    )?;
    sandbox.write("home/projects/acme-webshop.toml", WEBSHOP)?;
    sandbox.write_config("[reports]\ntime-zone = \"Europe/Berlin\"\n")?;
    // 01:00 in Berlin is 23:00 of the day before in UTC: in Berlin both
    // entries share a day, 20 minutes rounded to 30.
    let output = sandbox.journey(&[
        (
            &october(6, "12:00"),
            &[
                "log",
                "Night fix",
                "--project",
                "acme-webshop",
                "--from",
                "2026-10-06 01:00",
                "--to",
                "2026-10-06 01:10",
            ],
        ),
        (
            &october(6, "12:01"),
            &[
                "log",
                "Morning fix",
                "--project",
                "acme-webshop",
                "--from",
                "2026-10-06 10:00",
                "--to",
                "2026-10-06 10:10",
            ],
        ),
        (
            &october(6, "12:02"),
            &[
                "bill",
                "--mark-billed",
                "2026-0043",
                "--from",
                "2026-10-01",
                "--to",
                "2026-10-31",
            ],
        ),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

// Billed entries are protected

#[test]
fn adjust_of_billed_entry_names_invoice_and_change_billed() -> TestResult {
    let sandbox = october_sandbox()?;

    let output = sandbox.journey(&[
        ("15:00", &["bill", "--mark-billed", "2026-0042"]),
        (
            "15:05",
            &["adjust", "--last", "--note", "Write the blog post"],
        ),
        (
            "15:06",
            &[
                "adjust",
                "--last",
                "--note",
                "Write the blog post",
                "--change-billed",
            ],
        ),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn forget_of_billed_entry_is_refused() -> TestResult {
    let sandbox = october_sandbox()?;

    let output = sandbox.journey(&[
        ("15:00", &["bill", "--mark-billed", "2026-0042"]),
        ("15:05", &["forget", "--last", "--for-good"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[cfg(unix)]
#[test]
fn edit_of_billed_entry_is_refused_before_editor_opens() -> TestResult {
    let sandbox = october_sandbox()?;
    _ = run(sandbox
        .pace_at("15:00")
        .args(["bill", "--mark-billed", "2026-0042"]))?;

    let refused = run(sandbox
        .pace_at("15:05")
        .env("VISUAL", "false")
        .args(["edit", "--last"]))?;
    let edited = run(sandbox
        .pace_at("15:06")
        .env(
            "VISUAL",
            "sed -i -e 's/^note = .*/note = \"Write the blog post\"/'",
        )
        .args(["edit", "--last", "--change-billed"]))?;

    let transcript = sandbox.redact_ids(&format!("{refused}\n{edited}"))?;
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}

/// An entry logged on this device and billed on a second one, whose
/// marking this device then syncs in: the last batch of this device is
/// the entry's own.
fn entry_billed_on_other_device(sandbox: &Sandbox) -> TestResult {
    _ = run(sandbox.pace_at(&october(7, "12:00")).args([
        "log",
        "Fix the checkout",
        "--project",
        "acme-webshop",
        "--from",
        "2026-10-07 09:00",
        "--to",
        "2026-10-07 12:00",
    ]))?;
    sandbox.sync_events_to("other-home")?;
    sandbox.write("other-home/clients.toml", ACME)?;
    sandbox.write("other-home/projects/acme-webshop.toml", WEBSHOP)?;
    let marked = run(sandbox
        .pace_at("14:00")
        .env("PACE_HOME", sandbox.path("other-home"))
        .env("PACE_MACHINE_DIR", sandbox.path("other-machine"))
        .args(["bill", "--mark-billed", "2026-0042"]))?;
    assert!(marked.starts_with("success: true"), "{marked}");
    sandbox.sync_events_from("other-home")?;
    Ok(())
}

#[test]
fn undo_of_begin_of_billed_entry_is_refused() -> TestResult {
    let sandbox = catalog_sandbox(false)?;
    entry_billed_on_other_device(&sandbox)?;

    let output = sandbox.journey(&[
        ("15:00", &["undo"]),
        ("15:05", &["undo", "--change-billed"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}
