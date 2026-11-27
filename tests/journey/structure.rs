//! Roles: `pace role add`, `list`, `archive` and `edit` keep the roles
//! file `PACE_HOME/roles.toml`.

use crate::support::Sandbox;

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
