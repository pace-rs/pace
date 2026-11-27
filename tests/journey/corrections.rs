//! Correcting the log: adjust, log, edit, undo and forget.

use crate::support::{Sandbox, begin_apart, event_text, journey_and_events, run, short_id_after};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn adjust_changes_the_open_entry_or_the_last_one() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:00", &["begin", "Draft", "--tag", "draft"]),
        (
            "09:10",
            &[
                "adjust",
                "--start",
                "08:45",
                "--note",
                "Write offer",
                "--tag",
                "offer",
                "--untag",
                "draft",
            ],
        ),
        ("09:20", &["adjust"]),
        ("10:00", &["end"]),
        ("10:05", &["adjust", "--last", "--end", "09:55"]),
        ("10:06", &["adjust", "--last", "--end", "08:00"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn an_older_entry_is_found_by_its_short_id_and_last() -> TestResult {
    let sandbox = Sandbox::new()?;
    let logged = run(sandbox.pace_at("2026-10-01T18:00[Europe/Berlin]").args([
        "log",
        "Old offer",
        "--from",
        "14:00",
        "--to",
        "15:00",
    ]))?;
    let short = short_id_after(&logged, "Logged ")?;

    let output = sandbox.journey(&[
        ("10:00", &["adjust", &short, "--note", "Offer for ACME"]),
        ("10:01", &["adjust", "--last", "--tag", "offer"]),
        ("10:02", &["adjust", "deadbeef", "--note", "nothing"]),
        ("10:03", &["adjust", "no-id", "--note", "nothing"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn log_enters_time_afterwards() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/.pace.toml", "[project]\nkey = \"acme-webshop\"\n")?;

    let output = sandbox.journey(&[
        (
            "18:00",
            &["log", "Call with ACME", "--from", "14:00", "--to", "15:30"],
        ),
        (
            "18:00",
            &[
                "log",
                "Review",
                "--from",
                "16:00",
                "--duration",
                "45m",
                "--tag",
                "review",
            ],
        ),
        (
            "18:00",
            &["log", "Overlap", "--from", "15:00", "--duration", "1h"],
        ),
        (
            "18:00",
            &["log", "Backwards", "--from", "17:00", "--to", "16:50"],
        ),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn log_refuses_duration_that_is_not_positive() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        (
            "09:00",
            &["log", "x", "--from", "07:00", "--duration", "30m ago"],
        ),
        (
            "09:00",
            &["log", "x", "--from", "07:00", "--duration", "0m"],
        ),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    assert_eq!(event_text(&sandbox)?, "", "nothing is logged");
    Ok(())
}

#[test]
fn undo_goes_back_one_command_at_time() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:00", &["begin", "Write offer"]),
        ("10:00", &["end"]),
        ("10:01", &["undo"]),
        ("10:02", &["now"]),
        ("10:03", &["undo"]),
        ("10:04", &["undo"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn forget_asks_for_good_without_terminal() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "secret meeting"]))?;
    _ = run(sandbox.pace_at("10:00").arg("end"))?;
    let entry = event_text(&sandbox)?
        .lines()
        .find_map(|line| line.strip_prefix("entry = \""))
        .map(|id| id.trim_end_matches('"')[32..].to_owned())
        .ok_or("the entry ID")?;

    let output = sandbox.journey(&[
        ("10:05", &["forget", &entry]),
        ("10:05", &["forget", &entry, "--for-good"]),
        ("10:06", &["undo"]),
    ])?;

    assert!(!event_text(&sandbox)?.contains("secret meeting"));
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn undo_after_other_device_forgot_entry_has_nothing_to_do() -> TestResult {
    let sandbox = Sandbox::new()?;
    let began = run(sandbox.pace_at("09:00").args(["begin", "secret meeting"]))?;
    let entry = short_id_after(&began, "Began ")?;
    sandbox.sync_events_to("other-home")?;
    _ = run(sandbox
        .pace_at("09:30")
        .env("PACE_HOME", sandbox.path("other-home"))
        .env("PACE_MACHINE_DIR", sandbox.path("other-machine"))
        .args(["forget", &entry, "--for-good"]))?;
    sandbox.sync_events_from("other-home")?;

    let output = sandbox.journey(&[("10:00", &["undo"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn undo_after_other_device_undid_begin_has_nothing_to_do() -> TestResult {
    let sandbox = Sandbox::new()?;
    let desktop = |at: &str| {
        let mut command = sandbox.pace_at(at);
        _ = command
            .env("PACE_HOME", sandbox.path("other-home"))
            .env("PACE_MACHINE_DIR", sandbox.path("other-machine"));
        command
    };
    _ = run(sandbox.pace_at("09:00").args(["begin", "Write offer"]))?;
    sandbox.sync_events_to("other-home")?;
    _ = run(desktop("09:30").arg("end"))?;
    sandbox.sync_events_from("other-home")?;
    _ = run(sandbox.pace_at("10:00").arg("undo"))?;
    sandbox.sync_events_to("other-home")?;

    let undone = desktop("10:05").arg("undo").output()?;

    assert_eq!(undone.status.code(), Some(3));
    assert!(String::from_utf8(undone.stderr)?.contains("nothing left to undo"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn edit_turns_the_changes_into_an_adjustment() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox
        .pace_at("09:00")
        .args(["begin", "Draft", "--tag", "draft"]))?;
    _ = run(sandbox.pace_at("10:00").arg("end"))?;

    let edited = run(sandbox
        .pace_at("10:05")
        .env(
            "VISUAL",
            "sed -i -e 's/^note = .*/note = \"Write offer\"/' -e '/^tags/d'",
        )
        .args(["edit", "--last"]))?;
    let untouched = run(sandbox
        .pace_at("10:06")
        .env("VISUAL", "true")
        .args(["edit", "--last"]))?;
    let broken = run(sandbox
        .pace_at("10:07")
        .env("VISUAL", "sed -i -e 's/^note/notes/'")
        .args(["edit", "--last"]))?;

    let transcript = sandbox.redact_ids(&format!("{edited}\n{untouched}\n{broken}"))?;
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}

#[cfg(unix)]
#[test]
fn edit_without_entry_takes_the_open_one() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "Draft"]))?;

    let edited = run(sandbox
        .pace_at("09:30")
        .env("VISUAL", "sed -i -e 's/^note = .*/note = \"Write offer\"/'")
        .arg("edit"))?;
    let now = run(sandbox.pace_at("09:31").arg("now"))?;

    let transcript = sandbox.redact_ids(&format!("{edited}\n{now}"))?;
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}

#[test]
fn edit_without_terminal_or_editor_names_adjust() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "Draft"]))?;
    _ = run(sandbox.pace_at("10:00").arg("end"))?;

    let refused = run(sandbox
        .pace_at("10:05")
        .env_remove("VISUAL")
        .env_remove("EDITOR")
        .args(["edit", "--last"]))?;

    let transcript = sandbox.redact_ids(&refused)?;
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}

#[test]
fn forget_without_entry_and_nothing_running_names_the_flags() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[("09:00", &["forget"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn forget_without_entry_lists_the_open_ones_without_terminal() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "on this machine"]))?;
    begin_apart(&sandbox, "09:30", "on the other machine")?;

    let output = sandbox.journey(&[("10:00", &["forget"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn adjust_without_entry_names_one_of_open_ones_as_forget_does() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "on this machine"]))?;
    begin_apart(&sandbox, "09:30", "on the other machine")?;

    let output = sandbox.journey(&[("10:00", &["adjust", "--note", "Call"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn log_allow_overlap_enters_entry_over_another() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        (
            "18:00",
            &["log", "Call with ACME", "--from", "14:00", "--to", "15:30"],
        ),
        (
            "18:00",
            &[
                "log",
                "Review",
                "--from",
                "15:00",
                "--duration",
                "1h",
                "--allow-overlap",
            ],
        ),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn adjust_into_another_entry_needs_allow_overlap() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:00", &["begin", "Write offer"]),
        ("10:00", &["begin", "Call ACME"]),
        ("10:30", &["adjust", "--start", "09:30"]),
        ("10:30", &["adjust", "--start", "09:30", "--allow-overlap"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn adjust_allow_overlap_alone_marks_entry_once() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:00", &["begin", "Write offer"]),
        ("09:10", &["adjust", "--allow-overlap"]),
        ("09:20", &["adjust", "--allow-overlap"]),
    ])?;
    assert!(event_text(&sandbox)?.contains("allow-overlap = true"));
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn adjust_last_without_ended_entry_has_nothing_to_do() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:00", &["begin", "Write offer"]),
        ("09:30", &["adjust", "--last", "--note", "Offer"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn adjust_last_takes_entry_that_ended_last_not_one_logged_last() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        (
            "18:00",
            &["log", "Afternoon", "--from", "14:00", "--to", "15:00"],
        ),
        (
            "18:00",
            &["log", "Morning", "--from", "09:00", "--to", "10:00"],
        ),
        ("18:05", &["adjust", "--last", "--tag", "late"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn adjust_takes_full_entry_id() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "Draft"]))?;
    let output = sandbox.pace_at("09:10").args(["now", "--json"]).output()?;
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let id = json["open"][0]["id"].as_str().ok_or("the entry's ID")?;

    let adjusted = run(sandbox
        .pace_at("09:20")
        .args(["adjust", id, "--note", "Write offer"]))?;

    let transcript = sandbox.redact_ids(&adjusted)?;
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}

#[cfg(unix)]
#[test]
fn edit_into_another_entry_needs_allow_overlap() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "Write offer"]))?;
    _ = run(sandbox.pace_at("10:00").args(["begin", "Call ACME"]))?;
    let earlier = "sed -i -e 's/^start = .*/start = \"2026-11-26 09:30\"/'";

    let refused = run(sandbox.pace_at("10:30").env("VISUAL", earlier).arg("edit"))?;
    let allowed = run(sandbox
        .pace_at("10:30")
        .env("VISUAL", earlier)
        .args(["edit", "--allow-overlap"]))?;

    let transcript = sandbox.redact_ids(&format!("{refused}\n{allowed}"))?;
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}

#[test]
fn adjust_role_sets_role_and_empty_role_removes_it() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = journey_and_events(
        &sandbox,
        &[
            ("08:50", &["role", "add", "work", "--name", "Work"]),
            ("08:51", &["role", "add", "health", "--name", "Health"]),
            ("09:00", &["begin", "Write offer", "--role", "work"]),
            (
                "09:10",
                &["adjust", "--role", "health", "--by-others", "--billable"],
            ),
            (
                "09:20",
                &["adjust", "--role", "", "--by-me", "--not-billable"],
            ),
            ("09:30", &["adjust", "--role", "wrok"]),
            ("09:30", &["adjust", "--role", ""]),
        ],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn log_with_role_names_role_and_records_decided_and_billable() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = journey_and_events(
        &sandbox,
        &[
            ("08:50", &["role", "add", "work", "--name", "Work"]),
            (
                "12:00",
                &[
                    "log",
                    "Call ACME",
                    "--from",
                    "09:00",
                    "--to",
                    "09:30",
                    "--role",
                    "work",
                    "--by-others",
                    "--billable",
                ],
            ),
            (
                "12:00",
                &[
                    "log",
                    "Plan",
                    "--from",
                    "10:00",
                    "--duration",
                    "15m",
                    "--by-me",
                ],
            ),
        ],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn adjust_to_project_role_and_billable_writes_no_exception() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = journey_and_events(
        &sandbox,
        &[
            ("08:50", &["role", "add", "work", "--name", "Work"]),
            ("08:51", &["role", "add", "health", "--name", "Health"]),
            ("08:52", &["client", "add", "acme", "--currency", "EUR"]),
            (
                "08:53",
                &[
                    "project", "add", "webshop", "--client", "acme", "--role", "work",
                ],
            ),
            ("09:00", &["--project", "webshop", "begin", "Write offer"]),
            ("09:10", &["adjust", "--role", "work", "--billable"]),
            ("09:20", &["adjust", "--role", "health", "--not-billable"]),
            ("09:30", &["adjust", "--role", "work", "--billable"]),
        ],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn adjust_into_project_of_entry_own_role_and_billable_clears_them() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = journey_and_events(
        &sandbox,
        &[
            ("08:50", &["role", "add", "work", "--name", "Work"]),
            ("08:51", &["role", "add", "health", "--name", "Health"]),
            ("08:52", &["client", "add", "acme", "--currency", "EUR"]),
            (
                "08:53",
                &[
                    "project", "add", "webshop", "--client", "acme", "--role", "work",
                ],
            ),
            ("08:54", &["project", "add", "garden", "--role", "health"]),
            (
                "09:00",
                &[
                    "--project",
                    "webshop",
                    "begin",
                    "Write offer",
                    "--role",
                    "health",
                    "--not-billable",
                ],
            ),
            (
                "09:10",
                &[
                    "--project",
                    "garden",
                    "adjust",
                    "--role",
                    "health",
                    "--not-billable",
                ],
            ),
        ],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

/// An editor that keeps a copy of the file pace wrote as `<name>.toml` in
/// the sandbox, then runs `sed -i` with `expressions` on it; without
/// expressions it saves the file as it was.
#[cfg(unix)]
fn copying_editor(
    sandbox: &Sandbox,
    name: &str,
    expressions: &[&str],
) -> Result<String, Box<dyn std::error::Error>> {
    let copy = sandbox.path(format!("{name}.toml"));
    let script = sandbox.path(format!("{name}.sh"));
    let expressions: Vec<String> = expressions
        .iter()
        .map(|expression| format!("-e '{expression}'"))
        .collect();
    let change = if expressions.is_empty() {
        String::new()
    } else {
        format!("sed -i {} \"$1\"\n", expressions.join(" "))
    };
    std::fs::write(&script, format!("cp \"$1\" '{}'\n{change}", copy.display()))?;
    Ok(format!("sh '{}'", script.display()))
}

/// The copy that [`copying_editor`] kept as `<name>.toml`.
#[cfg(unix)]
fn copied_file(sandbox: &Sandbox, name: &str) -> std::io::Result<String> {
    std::fs::read_to_string(sandbox.path(format!("{name}.toml")))
}

#[cfg(unix)]
#[test]
fn edit_round_trip_sets_role_who_decided_billable_and_allowed_overlap() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox
        .pace_at("08:50")
        .args(["role", "add", "work", "--name", "Work"]))?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "Write offer"]))?;
    _ = run(sandbox.pace_at("10:00").arg("end"))?;
    let setting = copying_editor(
        &sandbox,
        "first",
        &[
            r#"s/^# role = .*/role = "work"/"#,
            r#"s/^# determined-by = "other".*/determined-by = "other"/"#,
            "s/^# billable = true/billable = true/",
            "s/^# allow-overlap = true/allow-overlap = true/",
        ],
    )?;
    let keeping = copying_editor(&sandbox, "second", &[])?;

    let set = run(sandbox
        .pace_at("10:05")
        .env("VISUAL", setting)
        .args(["edit", "--last"]))?;
    let kept = run(sandbox
        .pace_at("10:06")
        .env("VISUAL", keeping)
        .args(["edit", "--last"]))?;

    let transcript = sandbox.redact_ids(&format!(
        "{set}\n{kept}\n--- the file of the first edit ---\n{}\n\
         --- the file of the second edit ---\n{}\n--- home/events ---\n{}",
        copied_file(&sandbox, "first")?,
        copied_file(&sandbox, "second")?,
        event_text(&sandbox)?
    ))?;
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}

#[cfg(unix)]
#[test]
fn edit_without_role_line_clears_role() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox
        .pace_at("08:50")
        .args(["role", "add", "work", "--name", "Work"]))?;
    _ = run(sandbox
        .pace_at("09:00")
        .args(["begin", "Write offer", "--role", "work"]))?;

    let cleared = run(sandbox
        .pace_at("09:30")
        .env("VISUAL", "sed -i -e '/^role = /d'")
        .arg("edit"))?;

    let transcript = sandbox.redact_ids(&format!(
        "{cleared}\n--- home/events ---\n{}",
        event_text(&sandbox)?
    ))?;
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}

#[cfg(unix)]
#[test]
fn edit_with_unknown_role_key_names_its_line_and_closest_key() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox
        .pace_at("08:50")
        .args(["role", "add", "work", "--name", "Work"]))?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "Write offer"]))?;

    let refused = run(sandbox
        .pace_at("09:30")
        .env("VISUAL", r#"sed -i -e 's/^# role = .*/role = "wrok"/'"#)
        .arg("edit"))?;

    let transcript = sandbox.redact_ids(&format!(
        "{refused}\n--- home/events ---\n{}",
        event_text(&sandbox)?
    ))?;
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}

#[cfg(unix)]
#[test]
fn edit_to_project_role_and_billable_leaves_entry_unchanged() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox
        .pace_at("08:50")
        .args(["role", "add", "work", "--name", "Work"]))?;
    _ = run(sandbox
        .pace_at("08:51")
        .args(["client", "add", "acme", "--currency", "EUR"]))?;
    _ = run(sandbox.pace_at("08:52").args([
        "project", "add", "webshop", "--client", "acme", "--role", "work",
    ]))?;
    _ = run(sandbox
        .pace_at("09:00")
        .args(["--project", "webshop", "begin", "Write offer"]))?;

    let billable = run(sandbox
        .pace_at("09:30")
        .env(
            "VISUAL",
            "sed -i -e 's/^# billable = true/billable = true/'",
        )
        .arg("edit"))?;
    let role = run(sandbox
        .pace_at("09:31")
        .env("VISUAL", r#"sed -i -e 's/^# role = .*/role = "work"/'"#)
        .arg("edit"))?;

    let transcript = sandbox.redact_ids(&format!(
        "{billable}\n{role}\n--- home/events ---\n{}",
        event_text(&sandbox)?
    ))?;
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}

#[cfg(unix)]
#[test]
fn edit_keeps_own_role_and_billable_equal_to_later_project() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox
        .pace_at("08:50")
        .args(["role", "add", "work", "--name", "Work"]))?;
    _ = run(sandbox.pace_at("09:00").args([
        "--project",
        "webshop",
        "begin",
        "Write offer",
        "--role",
        "work",
        "--billable",
    ]))?;
    _ = run(sandbox.pace_at("09:10").args([
        "project",
        "add",
        "webshop",
        "--role",
        "work",
        "--billable",
    ]))?;

    let unchanged = run(sandbox.pace_at("09:30").env("VISUAL", "true").arg("edit"))?;
    let note = run(sandbox
        .pace_at("09:31")
        .env("VISUAL", r#"sed -i -e 's/^note = .*/note = "Send offer"/'"#)
        .arg("edit"))?;

    let transcript = sandbox.redact_ids(&format!(
        "{unchanged}\n{note}\n--- home/events ---\n{}",
        event_text(&sandbox)?
    ))?;
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}
