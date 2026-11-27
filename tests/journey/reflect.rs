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

/// The ID of the only device of `sandbox`, from the name of its directory.
fn device_id(sandbox: &Sandbox) -> Result<String, Box<dyn std::error::Error>> {
    let entry = std::fs::read_dir(sandbox.path("home/events"))?
        .next()
        .ok_or("no device directory")??;
    Ok(entry.file_name().to_string_lossy().into_owned())
}

#[test]
fn device_is_named_by_id_short_id_or_name() -> TestResult {
    let sandbox = week_sandbox()?;
    let id = device_id(&sandbox)?;
    let file = sandbox.path(format!("home/events/{id}/device.toml"));
    let text = std::fs::read_to_string(&file)?;
    let renamed: Vec<String> = text
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
    let whole = run(sandbox.pace_at("15:00").args(["reflect"]))?;
    let short = &id[id.len() - 4..];

    for device in [id.as_str(), short, "laptop"] {
        let filtered = run(sandbox
            .pace_at("15:00")
            .args(["reflect", "--device", device]))?;
        assert_eq!(filtered, whole, "{device}");
    }
    let output = sandbox
        .pace_at("15:00")
        .args(["reflect", "--device", "lapton"])
        .output()?;
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("did you mean `laptop`?"), "{stderr}");
    Ok(())
}

#[test]
fn entries_of_other_device_leave_report_of_this_one() -> TestResult {
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
    let here = std::fs::read_to_string(sandbox.path("machine/device.toml"))?;
    let here_id = here
        .lines()
        .find_map(|line| line.strip_prefix("id = "))
        .ok_or("no device ID")?
        .trim_matches('"')
        .to_owned();

    let output = run(sandbox
        .pace_at("10:00")
        .args(["reflect", "today", "--device", &here_id]))?;

    assert!(output.contains("Tracked 1h"), "{output}");
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
