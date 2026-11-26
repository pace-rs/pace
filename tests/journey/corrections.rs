//! Correcting the log: adjust, log, edit, undo and forget.

use crate::support::{Sandbox, run};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// All event files of the sandbox, as one text.
fn event_text(sandbox: &Sandbox) -> Result<String, Box<dyn std::error::Error>> {
    let mut text = String::new();
    for device in std::fs::read_dir(sandbox.path("home/events"))? {
        let file = device?.path().join("2026/2026-11.toml");
        if file.is_file() {
            text.push_str(&std::fs::read_to_string(file)?);
        }
    }
    Ok(text)
}

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
    let short = logged
        .lines()
        .find_map(|line| line.strip_prefix("Logged "))
        .and_then(|rest| rest.split_whitespace().next())
        .ok_or("the short ID")?
        .to_owned();

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
fn undo_goes_back_one_command_at_a_time() -> TestResult {
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
fn forget_asks_for_good_without_a_terminal() -> TestResult {
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
