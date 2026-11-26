//! Tracking with a fixed clock: begin, hold, pause, resume, catch, end.

use crate::support::{Sandbox, run};

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
fn a_working_morning_with_an_interruption_and_lunch() -> TestResult {
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
fn catch_without_a_running_entry_is_an_error() -> TestResult {
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
