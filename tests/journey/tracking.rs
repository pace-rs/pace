//! Tracking with a fixed clock: begin, hold, pause, resume, catch, end.

use crate::support::{Sandbox, run, short_id_after};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Run each step at its time and snapshot all outputs together.
#[test]
fn the_first_entry_creates_the_data_directory_and_says_where() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[("09:00", &["begin", "Write", "offer"])])?;

    assert!(sandbox.path("home/events").is_dir());
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn working_morning_with_an_interruption_and_lunch() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:00", &["begin", "Write offer", "--tag", "offer"]),
        ("10:00", &["hold", "--outer", "--reason", "phone call"]),
        ("10:05", &["hold", "--reason", "Slack"]),
        ("10:20", &["resume"]),
        ("11:00", &["catch", "call", "the", "tax", "office"]),
        ("12:00", &["pause", "--lunch"]),
        ("12:30", &["resume"]),
        ("13:00", &["end"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn begin_while_running_switches() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:00", &["begin", "Write offer"]),
        ("10:30", &["begin", "Call ACME", "--at", "10:15"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn nothing_to_do_exits_with_three() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[("09:00", &["end"]), ("09:00", &["resume"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn begin_and_end_in_different_time_zones() -> TestResult {
    let sandbox = Sandbox::new()?;

    let mut transcript = run(sandbox.pace_at("09:00").args(["begin", "Flight"]))?;
    transcript.push_str(&run(sandbox
        .pace_at("2026-11-26T05:00[America/New_York]")
        .env("PACE_TIME_ZONE", "America/New_York")
        .arg("end"))?);

    let events = std::fs::read_dir(sandbox.path("home/events"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("2026/2026-11.toml"))
        .find(|path| path.is_file())
        .ok_or("the event file")?;
    let text = std::fs::read_to_string(events)?;
    assert!(
        text.contains("at = \"2026-11-26T09:00:00+01:00[Europe/Berlin]\""),
        "{text}"
    );
    assert!(
        text.contains("at = \"2026-11-26T05:00:00-05:00[America/New_York]\""),
        "{text}"
    );
    let transcript = sandbox.redact_ids(&transcript)?;
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}

#[test]
fn at_takes_the_grammar_and_refuses_the_future() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:30", &["begin", "--at", "soon"]),
        ("09:30", &["begin", "--at", "10:00"]),
        ("09:30", &["begin", "--at", "+15m"]),
        ("09:30", &["begin", "--at", "-15m"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn catch_without_running_entry_is_an_error() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[("09:00", &["catch", "an idea"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn begin_names_the_project_and_why() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/.pace.toml", "[project]\nkey = \"acme-webshop\"\n")?;

    let output = sandbox.journey(&[("09:00", &["begin", "Checkout flow"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn config_show_does_not_create_anything() -> TestResult {
    let sandbox = Sandbox::new()?;

    _ = run(sandbox.pace_at("09:00").args(["config", "show"]))?;

    assert!(!sandbox.path("home").exists());
    assert!(!sandbox.path("machine").exists());
    Ok(())
}

#[test]
fn every_event_has_its_own_id_also_at_the_same_moment() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox.pace_at("08:00").args(["begin", "Write offer"]))?;
    for text in ["mail", "chat", "coffee"] {
        _ = run(sandbox.pace_at("11:00").args(["catch", text]))?;
    }

    let mut ids = Vec::new();
    for device in std::fs::read_dir(sandbox.path("home/events"))? {
        let text = std::fs::read_to_string(device?.path().join("2026/2026-11.toml"))?;
        ids.extend(
            text.lines()
                .filter_map(|line| line.strip_prefix("id = "))
                .map(ToOwned::to_owned),
        );
    }
    let count = ids.len();
    ids.sort_unstable();
    ids.dedup();

    assert_eq!(count, 4, "one begin and three catches");
    assert_eq!(ids.len(), count, "event IDs repeat: {ids:?}");
    Ok(())
}

#[test]
fn end_names_an_entry_that_has_ended() -> TestResult {
    let sandbox = Sandbox::new()?;
    let began = run(sandbox.pace_at("09:00").args(["begin", "Write offer"]))?;
    let short = short_id_after(&began, "Began ")?;

    let output = sandbox.journey(&[
        ("09:15", &["end"]),
        ("09:30", &["end", &short]),
        ("09:31", &["resume", &short]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn begin_into_full_disk_is_error_not_crash() -> TestResult {
    let sandbox = Sandbox::new()?;
    let full = std::fs::File::options().write(true).open("/dev/full")?;

    let output = sandbox
        .pace_at("09:00")
        .args(["begin", "Write offer"])
        .stdout(full)
        .output()?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("cannot write to the standard output"),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn new_reason_for_open_break_says_reason_is_now() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:00", &["begin", "Write offer"]),
        ("09:10", &["pause"]),
        ("09:20", &["pause", "--lunch", "--reason", "food"]),
        ("09:30", &["resume"]),
        ("10:00", &["hold", "--outer"]),
        ("10:00", &["hold", "--reason", "phone call"]),
        ("10:05", &["now"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn text_without_words_is_usage_error() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:00", &["begin", "Write offer"]),
        ("09:10", &["catch", ""]),
        ("09:10", &["catch", " ", " "]),
        ("09:10", &["hold", "--reason", ""]),
        ("09:10", &["pause", "--reason", "  "]),
        ("09:10", &["now"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn begin_again_names_project_and_refuses_another() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:00", &["--project", "offer", "begin", "Write offer"]),
        ("10:00", &["end"]),
        ("10:30", &["begin", "--again"]),
        ("11:00", &["begin", "--again", "--project", "gamma"]),
        ("11:00", &["--project", "gamma", "begin", "--again"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}
