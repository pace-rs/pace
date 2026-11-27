//! `pace reflect`: the report of a period as a table, its periods and its
//! filters.

use crate::support::{Sandbox, run};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ROLES: &str = r#"version = 1

[work]
id = "0193a7c4-0000-7000-8000-0000000000a1"
kind = "area"
name = "Work"
weekly-target = "30h"

[health]
id = "0193a7c4-0000-7000-8000-0000000000a2"
kind = "area"
name = "Health"
"#;

const CLIENTS: &str = r#"version = 1

[acme]
name = "ACME GmbH"
currency = "EUR"

[[acme.rates]]
hourly = 95
from = 2025-01-01
"#;

const WEBSHOP: &str = r#"version = 1
id = "0193a7c4-0000-7000-8000-000000000001"
key = "acme-webshop"
name = "Webshop relaunch"
client = "acme"
role = "work"
billable = true
budget = "10h"
"#;

/// A sandbox with the roles `work` (30 hours a week) and `health`, the
/// client `acme` at 95 EUR an hour, the project `acme-webshop` of `acme`
/// with a budget of ten hours, and reports in Berlin.
fn catalog_sandbox() -> Result<Sandbox, Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    sandbox.write("home/roles.toml", ROLES)?;
    sandbox.write("home/clients.toml", CLIENTS)?;
    sandbox.write("home/projects/acme-webshop.toml", WEBSHOP)?;
    sandbox.write_config("[reports]\ntime-zone = \"Europe/Berlin\"\n")?;
    Ok(sandbox)
}

/// The steps of a week up to Thursday 15:00: a night shift over midnight
/// on the webshop, a run for health, an offer with a hold and a lunch
/// break, and mail that others decided on, still running, with a catch.
const WEEK: [(&str, &[&str]); 10] = [
    (
        "09:00",
        &[
            "log",
            "Deploy at night",
            "--project",
            "acme-webshop",
            "--from",
            "2026-11-24 23:00",
            "--to",
            "2026-11-25 01:00",
        ],
    ),
    (
        "09:00",
        &[
            "log",
            "Run",
            "--role",
            "health",
            "--by-me",
            "--from",
            "2026-11-25 07:00",
            "--to",
            "2026-11-25 08:00",
        ],
    ),
    (
        "09:00",
        &[
            "begin",
            "Write offer",
            "--project",
            "acme-webshop",
            "--tag",
            "offer",
        ],
    ),
    ("10:00", &["hold", "--outer", "--reason", "phone call"]),
    ("10:15", &["resume"]),
    ("12:00", &["pause", "--lunch"]),
    ("12:30", &["resume"]),
    ("13:00", &["end"]),
    ("14:00", &["begin", "Read mail", "--by-others"]),
    ("14:30", &["catch", "call Bob back"]),
];

/// The sandbox of [`catalog_sandbox`] after the steps of [`WEEK`].
fn week_sandbox() -> Result<Sandbox, Box<dyn std::error::Error>> {
    let sandbox = catalog_sandbox()?;
    for (time, args) in WEEK {
        _ = run(sandbox.pace_at(time).args(args))?;
    }
    Ok(sandbox)
}

#[test]
fn reflect_shows_week_as_table() -> TestResult {
    let sandbox = week_sandbox()?;

    let output = sandbox.journey(&[("15:00", &["reflect"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn running_entry_marks_its_target_and_budget() -> TestResult {
    let sandbox = catalog_sandbox()?;
    _ = run(sandbox
        .pace_at("09:00")
        .args(["begin", "Write offer", "--project", "acme-webshop"]))?;

    let output = run(sandbox.pace_at("11:00").args(["reflect"]))?;

    assert!(output.contains("\n  Work  2h*  of 30h  7%\n"), "{output}");
    assert!(
        output.contains("\n  Webshop relaunch  2h*  of 10h  20%\n"),
        "{output}"
    );
    Ok(())
}

#[test]
fn running_entry_outside_period_marks_budget_with_footnote() -> TestResult {
    let sandbox = catalog_sandbox()?;
    _ = run(sandbox.pace_at("09:00").args([
        "log",
        "Plan relaunch",
        "--project",
        "acme-webshop",
        "--from",
        "2026-11-18 09:00",
        "--to",
        "2026-11-18 12:00",
    ]))?;
    _ = run(sandbox
        .pace_at("09:00")
        .args(["begin", "Write offer", "--project", "acme-webshop"]))?;

    let output = run(sandbox.pace_at("11:00").args(["reflect", "last-week"]))?;

    assert!(
        output.contains("\n  Webshop relaunch  5h*  of 10h  50%\n"),
        "{output}"
    );
    assert!(
        output.contains("\n\n* counts a running entry up to now.\n"),
        "{output}"
    );
    Ok(())
}

#[test]
fn billable_entry_begun_at_now_marks_revenue_with_footnote() -> TestResult {
    let sandbox = catalog_sandbox()?;
    sandbox.write(
        "home/projects/acme-webshop.toml",
        &WEBSHOP.replace("budget = \"10h\"\n", ""),
    )?;
    _ = run(sandbox.pace_at("11:00").args([
        "log",
        "Plan relaunch",
        "--project",
        "acme-webshop",
        "--from",
        "09:00",
        "--to",
        "10:00",
    ]))?;
    _ = run(sandbox
        .pace_at("11:00")
        .args(["begin", "Write offer", "--project", "acme-webshop"]))?;

    let output = run(sandbox.pace_at("11:00").args(["reflect", "today"]))?;

    assert!(
        output.contains("\n    Webshop relaunch  1h*  95.00 EUR\n"),
        "{output}"
    );
    assert!(
        output.contains("\n\n* counts a running entry up to now.\n"),
        "{output}"
    );
    Ok(())
}

#[test]
fn ref_is_short_for_reflect() -> TestResult {
    let sandbox = week_sandbox()?;

    let long = run(sandbox.pace_at("15:00").args(["reflect"]))?;
    let short = run(sandbox.pace_at("15:00").args(["ref", "this-week"]))?;

    assert_eq!(long, short);
    Ok(())
}

#[test]
fn filters_narrow_report() -> TestResult {
    let sandbox = week_sandbox()?;

    let output = sandbox.journey(&[
        ("15:00", &["reflect", "--role", "health"]),
        (
            "15:00",
            &["reflect", "--project", "acme-webshop", "--tag", "offer"],
        ),
        ("15:00", &["reflect", "--client", "acme", "today"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn from_and_to_pick_days() -> TestResult {
    let sandbox = week_sandbox()?;

    let output = sandbox.journey(&[
        (
            "15:00",
            &["reflect", "--from", "2026-11-25", "--to", "2026-11-25"],
        ),
        ("15:00", &["reflect", "--to", "2026-11-24"]),
        ("15:00", &["reflect", "--from", "2026-11-26"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn unknown_project_key_is_usage_error_naming_closest() -> TestResult {
    let sandbox = week_sandbox()?;

    let output = sandbox
        .pace_at("15:00")
        .args(["reflect", "--project", "acme-websop"])
        .output()?;

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("did you mean `acme-webshop`?"), "{stderr}");
    Ok(())
}

#[test]
fn unknown_role_and_client_are_usage_errors() -> TestResult {
    let sandbox = week_sandbox()?;

    for (flag, typo, closest) in [("--role", "helth", "health"), ("--client", "acm", "acme")] {
        let output = sandbox
            .pace_at("15:00")
            .args(["reflect", flag, typo])
            .output()?;

        assert_eq!(output.status.code(), Some(2), "{flag}");
        let stderr = String::from_utf8(output.stderr)?;
        assert!(
            stderr.contains(&format!("did you mean `{closest}`?")),
            "{stderr}"
        );
    }
    Ok(())
}

#[test]
fn unknown_period_is_usage_error_naming_closest() -> TestResult {
    let sandbox = week_sandbox()?;

    let output = sandbox
        .pace_at("15:00")
        .args(["reflect", "last-wek"])
        .output()?;

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("did you mean `last-week`?"), "{stderr}");
    Ok(())
}

#[test]
fn empty_period_says_so_and_exits_zero() -> TestResult {
    let sandbox = week_sandbox()?;

    let output = sandbox.journey(&[
        ("15:00", &["reflect", "last-week"]),
        (
            "15:00",
            &["reflect", "--from", "2026-11-01", "--to", "2026-11-02"],
        ),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn reflect_without_any_entry_says_nothing_tracked() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.pace_at("15:00").args(["reflect", "all"]).output()?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8(output.stdout)?, "Nothing tracked yet.\n");
    Ok(())
}

/// A sandbox with two devices: another one that logged half an hour at
/// 08:00, and this one, named `laptop`, that logged an hour at 09:00. It
/// gives the ID of this device.
fn two_device_sandbox() -> Result<(Sandbox, String), Box<dyn std::error::Error>> {
    let sandbox = catalog_sandbox()?;
    _ = run(sandbox
        .pace_at("09:00")
        .env("PACE_HOME", sandbox.path("other-home"))
        .env("PACE_MACHINE_DIR", sandbox.path("other-machine"))
        .args(["log", "Elsewhere", "--from", "08:00", "--to", "08:30"]))?;
    sandbox.sync_events_from("other-home")?;
    _ = run(sandbox
        .pace_at("10:00")
        .args(["log", "Here", "--from", "09:00", "--to", "10:00"]))?;
    let machine = std::fs::read_to_string(sandbox.path("machine/device.toml"))?;
    let id = machine
        .lines()
        .find_map(|line| line.strip_prefix("id = "))
        .ok_or("no device ID")?
        .trim_matches('"')
        .to_owned();
    let file = sandbox.path(format!("home/events/{id}/device.toml"));
    let renamed: Vec<String> = std::fs::read_to_string(&file)?
        .lines()
        .map(|line| {
            if line.starts_with("name = ") {
                "name = \"laptop\"".to_owned()
            } else {
                line.to_owned()
            }
        })
        .collect();
    std::fs::write(&file, renamed.join("\n") + "\n")?;
    Ok((sandbox, id))
}

/// The summary line of a report's transcript: `Tracked …`.
fn summary_line(transcript: &str) -> Option<&str> {
    transcript.lines().find(|line| line.starts_with("Tracked "))
}

#[test]
fn device_filter_drops_entries_of_other_device() -> TestResult {
    let (sandbox, id) = two_device_sandbox()?;

    let whole = run(sandbox.pace_at("10:00").args(["reflect", "today"]))?;
    assert_eq!(
        summary_line(&whole),
        Some("Tracked 1h 30m, untracked 8h 30m."),
        "{whole}"
    );
    let id_end = &id[id.len() - 6..];
    for device in [id.as_str(), id_end, "laptop"] {
        let filtered = run(sandbox
            .pace_at("10:00")
            .args(["reflect", "today", "--device", device]))?;
        assert_eq!(
            summary_line(&filtered),
            Some("Tracked 1h, untracked 9h."),
            "{device}: {filtered}"
        );
    }
    Ok(())
}

#[test]
fn unknown_device_name_or_id_is_usage_error() -> TestResult {
    let (sandbox, _) = two_device_sandbox()?;

    for (device, expected) in [
        ("lapton", "did you mean `laptop`?"),
        (
            "0193a7c4-0000-7000-8000-00000001ab12",
            "no device has the name or ID `0193a7c4-0000-7000-8000-00000001ab12`",
        ),
    ] {
        let output = sandbox
            .pace_at("10:00")
            .args(["reflect", "--device", device])
            .output()?;

        assert_eq!(output.status.code(), Some(2), "{device}");
        let stderr = String::from_utf8(output.stderr)?;
        assert!(stderr.contains(expected), "{stderr}");
    }
    Ok(())
}

const BUSY_ROLES: &str = r#"version = 1

[work]
id = "0193a7c4-0000-7000-8000-0000000000a1"
kind = "area"
name = "Work"
weekly-target = "100h"

[family]
id = "0193a7c4-0000-7000-8000-0000000000a3"
kind = "area"
name = "Family"
weekly-target = "80h"
"#;

const LATE_RATE: &str = r#"version = 1

[acme]
name = "ACME GmbH"
currency = "EUR"

[[acme.rates]]
hourly = 95
from = 2027-01-01
"#;

const LONG_PROJECT: &str = r#"version = 1
id = "0193a7c4-0000-7000-8000-000000000002"
key = "acme-relaunch"
name = "Relaunch of the webshop for the spring collection"
client = "acme"
role = "work"
billable = true
"#;

/// A sandbox of a busy Tuesday: role targets of 180 hours a week, a
/// project whose client has no rate before 2027, an overlap and a long
/// hold reason.
fn busy_sandbox() -> Result<Sandbox, Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    sandbox.write("home/roles.toml", BUSY_ROLES)?;
    sandbox.write("home/clients.toml", LATE_RATE)?;
    sandbox.write("home/projects/acme-relaunch.toml", LONG_PROJECT)?;
    sandbox.write_config("[reports]\ntime-zone = \"Europe/Berlin\"\n")?;
    let steps: [(&str, &[&str]); 6] = [
        (
            "09:00",
            &[
                "log",
                "Plan the spring collection",
                "--project",
                "acme-relaunch",
                "--from",
                "2026-11-24 09:00",
                "--to",
                "2026-11-24 11:00",
            ],
        ),
        (
            "09:00",
            &[
                "log",
                "Review the photos",
                "--allow-overlap",
                "--from",
                "2026-11-24 10:00",
                "--to",
                "2026-11-24 12:00",
            ],
        ),
        ("09:00", &["begin", "Write the newsletter"]),
        (
            "10:00",
            &[
                "hold",
                "--outer",
                "--reason",
                "the delivery service rang the bell twice",
            ],
        ),
        ("10:15", &["resume"]),
        ("11:00", &["end"]),
    ];
    for (time, args) in steps {
        _ = run(sandbox.pace_at(time).args(args))?;
    }
    Ok(sandbox)
}

#[test]
fn sentence_lines_wrap_to_eighty_columns() -> TestResult {
    let sandbox = busy_sandbox()?;

    let raw = run(sandbox.pace_at("15:00").args(["reflect", "--plain"]))?;
    let output = sandbox.journey(&[("15:00", &["reflect", "--plain"])])?;

    let long: Vec<&str> = raw
        .lines()
        .filter(|line| line.chars().count() > 80)
        .collect();
    assert!(long.is_empty(), "{long:#?}");
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn overlap_is_named_by_short_ids() -> TestResult {
    let sandbox = catalog_sandbox()?;

    let output = sandbox.journey(&[
        (
            "10:00",
            &["log", "Standup", "--from", "08:00", "--to", "09:00"],
        ),
        (
            "10:00",
            &[
                "log",
                "Review",
                "--from",
                "08:30",
                "--to",
                "09:30",
                "--allow-overlap",
            ],
        ),
        ("10:00", &["reflect", "today", "--plain"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn reflect_shows_week_as_json() -> TestResult {
    let sandbox = week_sandbox()?;

    let output = sandbox.journey(&[("15:00", &["reflect", "--format", "json"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn reflect_shows_week_as_csv() -> TestResult {
    let sandbox = week_sandbox()?;

    let output = sandbox.journey(&[("15:00", &["reflect", "--format", "csv"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn csv_quotes_note_with_comma_and_quote() -> TestResult {
    let sandbox = catalog_sandbox()?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "Write offer"]))?;
    _ = run(sandbox
        .pace_at("09:30")
        .args(["catch", r#"call Bob, say "hi""#]))?;

    let output = run(sandbox
        .pace_at("10:00")
        .args(["reflect", "today", "--format", "csv"]))?;

    let catch = output
        .lines()
        .find(|line| line.starts_with("pauses,catch,"))
        .ok_or_else(|| format!("no catch row in {output}"))?;
    assert!(catch.contains(r#","call Bob, say ""hi""","#), "{catch}");
    Ok(())
}

#[test]
fn json_money_is_decimal_string_with_currency() -> TestResult {
    let sandbox = catalog_sandbox()?;
    sandbox.write(
        "home/clients.toml",
        &CLIENTS.replace("hourly = 95", "hourly = 95.50"),
    )?;
    _ = run(sandbox.pace_at("11:00").args([
        "log",
        "Plan relaunch",
        "--project",
        "acme-webshop",
        "--from",
        "09:00",
        "--to",
        "10:00",
    ]))?;

    let output = run(sandbox
        .pace_at("11:00")
        .args(["reflect", "today", "--format", "json"]))?;

    assert!(output.contains(r#""amount": "95.50","#), "{output}");
    assert!(output.contains(r#""currency": "EUR""#), "{output}");
    Ok(())
}

#[test]
fn json_names_warnings_and_time_without_rate() -> TestResult {
    let sandbox = busy_sandbox()?;

    let output = sandbox.journey(&[(
        "15:00",
        &[
            "reflect",
            "--from",
            "2026-11-24",
            "--to",
            "2026-11-24",
            "--format",
            "json",
        ],
    )])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn json_and_csv_of_empty_period_write_rows_and_exit_zero() -> TestResult {
    let sandbox = week_sandbox()?;

    let output = sandbox.journey(&[
        ("15:00", &["reflect", "last-week", "--format", "json"]),
        ("15:00", &["reflect", "last-week", "--format", "csv"]),
    ])?;

    assert!(!output.contains("Nothing tracked"), "{output}");
    assert!(
        output.contains("\"group\": \"tracked\",") && output.contains("\"net_seconds\": 0,"),
        "{output}"
    );
    assert!(output.contains("\ntime,tracked,"), "{output}");
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn json_without_any_entry_writes_rows() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox
        .pace_at("15:00")
        .args(["reflect", "today", "--format", "json"])
        .output()?;

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.starts_with("{\n  \"version\": 1,"), "{stdout}");
    assert!(!stdout.contains("Nothing tracked"), "{stdout}");
    Ok(())
}

#[test]
fn reflect_shows_week_as_text() -> TestResult {
    let sandbox = week_sandbox()?;

    let output = sandbox.journey(&[("15:00", &["reflect", "--format", "text"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn reflect_shows_week_as_markdown() -> TestResult {
    let sandbox = week_sandbox()?;

    let output = sandbox.journey(&[("15:00", &["reflect", "--format", "markdown"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn reflect_shows_week_as_html() -> TestResult {
    let sandbox = week_sandbox()?;

    let output = sandbox.journey(&[("15:00", &["reflect", "--format", "html"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn text_has_no_box_drawing_and_no_colour() -> TestResult {
    let sandbox = week_sandbox()?;

    let output = run(sandbox
        .pace_at("15:00")
        .env("CLICOLOR_FORCE", "1")
        .args(["reflect", "--format", "text"]))?;

    let drawn: Vec<char> = output
        .chars()
        .filter(|c| ('\u{2500}'..='\u{259f}').contains(c) || *c == '\x1b')
        .collect();
    assert!(drawn.is_empty(), "{drawn:?} in {output}");
    Ok(())
}

#[test]
fn html_escapes_note() -> TestResult {
    let sandbox = catalog_sandbox()?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "Write offer"]))?;
    _ = run(sandbox
        .pace_at("09:30")
        .args(["catch", "<b>call Bob</b> & Alice"]))?;

    let output = run(sandbox
        .pace_at("10:00")
        .args(["reflect", "today", "--format", "html"]))?;

    assert!(
        output.contains("&lt;b&gt;call Bob&lt;&#x2F;b&gt; &amp; Alice")
            || output.contains("&lt;b&gt;call Bob&lt;/b&gt; &amp; Alice"),
        "{output}"
    );
    Ok(())
}

#[test]
fn text_of_empty_period_says_nothing_tracked() -> TestResult {
    let sandbox = week_sandbox()?;

    let output = run(sandbox
        .pace_at("15:00")
        .args(["reflect", "last-week", "--format", "text"]))?;

    assert!(output.contains("Nothing tracked"), "{output}");
    Ok(())
}

#[test]
fn templates_name_warnings_and_time_without_rate() -> TestResult {
    let sandbox = busy_sandbox()?;
    let day = ["reflect", "--from", "2026-11-24", "--to", "2026-11-24"];
    let text = [&day[..], &["--format", "text"]].concat();
    let markdown = [&day[..], &["--format", "markdown"]].concat();
    let html = [&day[..], &["--format", "html"]].concat();

    let output = sandbox.journey(&[("15:00", &text), ("15:00", &markdown), ("15:00", &html)])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn own_template_renders_report() -> TestResult {
    let sandbox = week_sandbox()?;
    sandbox.write(
        "work/mine.tera",
        "{{ kind }} {{ version }}: {% for row in rows %}\
         {% if row.group == \"tracked\" %}tracked {{ row.net }}{% endif %}{% endfor %}\n",
    )?;

    let output = run(sandbox
        .pace_at("15:00")
        .args(["reflect", "--template", "mine.tera"]))?;

    assert!(
        output.contains("----- stdout -----\nreflect 1: tracked 7h 15m\n"),
        "{output}"
    );
    Ok(())
}

#[test]
fn broken_template_names_its_file_and_line() -> TestResult {
    let sandbox = week_sandbox()?;
    sandbox.write(
        "work/broken.tera",
        "Report\n{% for row in rows %}\n{{ row.net }\n{% endfor %}\n",
    )?;

    let output = sandbox.journey(&[("15:00", &["reflect", "--template", "broken.tera"])])?;

    assert!(
        output.contains("the template broken.tera has a mistake on line 3"),
        "{output}"
    );
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn missing_template_file_is_named() -> TestResult {
    let sandbox = week_sandbox()?;

    let output = sandbox
        .pace_at("15:00")
        .args(["reflect", "--template", "missing.tera"])
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("cannot read the template missing.tera"),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn template_beside_format_is_usage_error() -> TestResult {
    let sandbox = week_sandbox()?;
    sandbox.write("work/mine.tera", "{{ kind }}\n")?;

    let output = sandbox
        .pace_at("15:00")
        .args(["reflect", "--format", "json", "--template", "mine.tera"])
        .output()?;

    assert_eq!(output.status.code(), Some(2));
    Ok(())
}
