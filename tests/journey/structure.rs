//! Roles and clients: `pace role add`, `list`, `archive` and `edit` keep
//! the roles file `PACE_HOME/roles.toml`; `pace client add`, `list` and
//! `edit` keep the clients file `PACE_HOME/clients.toml`.

use crate::support::{Sandbox, run};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The transcript of `steps`, then the roles file as pace left it.
fn journey_and_roles_file(
    sandbox: &Sandbox,
    steps: &[(&str, &[&str])],
) -> Result<String, Box<dyn std::error::Error>> {
    let transcript = sandbox.journey(steps)?;
    let roles = std::fs::read_to_string(sandbox.path("home/roles.toml"))?;
    Ok(format!(
        "{transcript}--- home/roles.toml ---\n{}",
        sandbox.redact_ids(&roles)?
    ))
}

#[test]
fn role_add_with_every_flag_writes_roles_file() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = journey_and_roles_file(
        &sandbox,
        &[(
            "09:00",
            &[
                "role",
                "add",
                "work",
                "--name",
                "Work",
                "--kind",
                "area-project",
                "--purpose",
                "Earn a living with good work",
                "--weekly-target",
                "30h",
            ],
        )],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn role_add_without_terminal_names_defaults_it_took() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = journey_and_roles_file(&sandbox, &[("09:00", &["role", "add", "work"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn role_add_keeps_comments_of_roles_file() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write(
        "home/roles.toml",
        "version = 1\n\n# what I am paid for\n[work]\nid = \"0193a7c4-0000-7000-8000-000000000001\"\nkind = \"area\"\nname = \"Work\"\n",
    )?;

    let output = journey_and_roles_file(
        &sandbox,
        &[("09:00", &["role", "add", "health", "--kind", "meta"])],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn second_role_add_of_same_key_fails_naming_role_edit() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:00", &["role", "add", "work", "--name", "Work"]),
        ("09:01", &["role", "add", "work", "--name", "Paid work"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn role_add_refuses_weekly_target_that_is_no_duration() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[(
        "09:00",
        &["role", "add", "work", "--weekly-target", "thirty"],
    )])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn role_add_refuses_blank_name_flag_and_key_version() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:00", &["role", "add", "work", "--name", "  "]),
        ("09:01", &["role", "add", "version"]),
        ("09:02", &["role", "list"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn role_list_shows_roles_in_use_and_archived_ones_on_request() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:00", &["role", "list"]),
        (
            "09:01",
            &[
                "role",
                "add",
                "work",
                "--name",
                "Work",
                "--weekly-target",
                "30h",
            ],
        ),
        ("09:02", &["role", "add", "health", "--name", "Health"]),
        ("09:03", &["role", "add", "review", "--kind", "meta"]),
        (
            "09:04",
            &["role", "archive", "health", "--reason", "inactive"],
        ),
        ("09:05", &["role", "list"]),
        ("09:06", &["role", "list", "--archived"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn role_archive_writes_day_and_reason() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = journey_and_roles_file(
        &sandbox,
        &[
            ("09:00", &["role", "add", "work", "--name", "Work"]),
            ("09:01", &["role", "archive", "work", "--reason", "ended"]),
            ("09:02", &["role", "archive", "work", "--reason", "ended"]),
        ],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn role_archive_of_unknown_role_names_closest_key() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:00", &["role", "add", "work"]),
        ("09:01", &["role", "archive", "wrok", "--reason", "ended"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn role_edit_without_terminal_names_roles_file() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[("09:00", &["role", "edit"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

/// The transcript of `steps`, then the clients file as pace left it.
fn journey_and_clients_file(
    sandbox: &Sandbox,
    steps: &[(&str, &[&str])],
) -> Result<String, Box<dyn std::error::Error>> {
    let transcript = sandbox.journey(steps)?;
    let clients = std::fs::read_to_string(sandbox.path("home/clients.toml"))?;
    Ok(format!("{transcript}--- home/clients.toml ---\n{clients}"))
}

/// A clients file with a rate that changes on 2026-12-01, after the
/// sandbox's today, and a client without a rate.
const CLIENTS: &str = r#"version = 1

# pays late
[acme]
name = "ACME GmbH"
currency = "EUR"
rounding = { per = "entry", step = "15m", mode = "up" }

[[acme.rates]]
hourly = 95.50
from = 2026-01-01

[[acme.rates]]
hourly = 105
from = 2026-12-01

[globex]
name = "Globex"
currency = "USD"
"#;

#[test]
fn client_add_with_every_flag_writes_clients_file() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = journey_and_clients_file(
        &sandbox,
        &[(
            "09:00",
            &[
                "client",
                "add",
                "acme",
                "--name",
                "ACME GmbH",
                "--currency",
                "EUR",
                "--rate",
                "95.50",
                "--from",
                "2026-11-01",
                "--rounding-step",
                "15m",
                "--rounding-mode",
                "up",
                "--rounding-per",
                "entry",
            ],
        )],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn client_add_without_terminal_names_defaults_it_took() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = journey_and_clients_file(
        &sandbox,
        &[(
            "09:00",
            &["client", "add", "acme", "--currency", "EUR", "--rate", "95"],
        )],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn client_add_without_currency_is_usage_error_naming_currency_flag() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:00", &["client", "add", "acme", "--name", "ACME GmbH"]),
        ("09:01", &["client", "list"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn client_add_refuses_what_does_not_fit_and_names_what_fits() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        (
            "09:00",
            &[
                "client",
                "add",
                "acme",
                "--currency",
                "EUR",
                "--rate",
                "95.555",
            ],
        ),
        ("09:01", &["client", "add", "acme", "--currency", "EURO"]),
        (
            "09:02",
            &[
                "client",
                "add",
                "acme",
                "--currency",
                "EUR",
                "--rounding-step",
                "0m",
            ],
        ),
        ("09:03", &["client", "add", "acme", "--from", "2026-11-01"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn second_client_add_of_same_key_names_client_edit() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:00", &["client", "add", "acme", "--currency", "EUR"]),
        ("09:01", &["client", "add", "acme", "--currency", "USD"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn client_list_shows_rate_of_today_also_before_later_rate() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("home/clients.toml", CLIENTS)?;

    let output = sandbox.journey(&[
        ("09:00", &["client", "list"]),
        ("2026-12-01T09:00[Europe/Berlin]", &["client", "list"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[cfg(unix)]
#[test]
fn client_edit_writes_edited_table_in_its_place() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("home/clients.toml", CLIENTS)?;

    let edited = run(sandbox
        .pace_at("09:00")
        .env(
            "VISUAL",
            "sed -i -e 's/^name = \"ACME GmbH\"/name = \"ACME AG\"/' -e 's/^hourly = 105$/hourly = 110/'",
        )
        .args(["client", "edit", "acme"]))?;
    let broken = run(sandbox
        .pace_at("09:01")
        .env("VISUAL", "sed -i -e 's/^hourly = 95.50$/hourly = 95.555/'")
        .args(["client", "edit", "acme"]))?;
    let unknown = run(sandbox
        .pace_at("09:02")
        .env("VISUAL", "true")
        .args(["client", "edit", "amce"]))?;
    let clients = std::fs::read_to_string(sandbox.path("home/clients.toml"))?;

    let output = format!("{edited}\n{broken}\n{unknown}\n--- home/clients.toml ---\n{clients}");
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn client_edit_without_terminal_or_editor_names_clients_file() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("home/clients.toml", CLIENTS)?;

    let output = run(sandbox
        .pace_at("09:00")
        .env_remove("VISUAL")
        .env_remove("EDITOR")
        .args(["client", "edit", "acme"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}
