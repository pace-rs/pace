//! `pace now` and bare `pace`: what is running, and choosing entries by
//! short ID.

use std::fmt::Write as _;

use crate::support::{Sandbox, run};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn journey(
    sandbox: &Sandbox,
    steps: &[(&str, &[&str])],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut transcript = String::new();
    for (time, args) in steps {
        writeln!(transcript, "$ pace {}   # at {time}", args.join(" "))?;
        transcript.push_str(&run(sandbox.pace_at(time).args(*args))?);
        transcript.push('\n');
    }
    Ok(transcript)
}

/// The short ID that `pace now` shows first.
fn first_short_id(sandbox: &Sandbox, time: &str) -> Result<String, Box<dyn std::error::Error>> {
    let output = sandbox.pace_at(time).args(["now", "--json"]).output()?;
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    json["open"][0]["short"]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| "no open entry".into())
}

#[test]
fn now_with_nothing_running_says_so() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = journey(&sandbox, &[("09:00", &["now"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn now_shows_the_open_entry_and_its_breaks() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/.pace.toml", "[project]\nkey = \"acme-webshop\"\n")?;

    let output = journey(
        &sandbox,
        &[
            ("09:00", &["begin", "Write offer", "--tag", "offer"]),
            ("10:00", &["now"]),
            ("10:15", &["hold", "--outer", "--reason", "phone call"]),
            ("10:30", &["now"]),
            ("10:45", &["resume"]),
            ("12:00", &["pause", "--lunch"]),
            ("12:10", &["now"]),
        ],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn now_speaks_json_for_scripts() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/.pace.toml", "[project]\nkey = \"acme-webshop\"\n")?;

    let output = journey(
        &sandbox,
        &[
            ("09:00", &["begin", "Write offer", "--tag", "offer"]),
            ("10:15", &["hold", "--outer", "--reason", "phone call"]),
            ("10:30", &["now", "--json"]),
        ],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn pace_without_arguments_shows_now_and_how_to_get_help() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = journey(
        &sandbox,
        &[("09:00", &["begin", "Write offer"]), ("09:30", &[])],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

/// An entry begun on another machine, apart: it has its own data
/// directory, which is then synced into this one.
fn begin_apart(sandbox: &Sandbox, time: &str, note: &str) -> TestResult {
    _ = run(sandbox
        .pace_at(time)
        .env("PACE_HOME", sandbox.path("other-home"))
        .env("PACE_MACHINE_DIR", sandbox.path("other-machine"))
        .args(["begin", note]))?;
    sandbox.sync_events_from("other-home")?;
    Ok(())
}

#[test]
fn entries_open_on_two_devices_are_both_shown() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "on this machine"]))?;
    begin_apart(&sandbox, "09:30", "on the other machine")?;

    let output = journey(&sandbox, &[("10:00", &["now"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn end_takes_a_short_id() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "on this machine"]))?;
    begin_apart(&sandbox, "09:30", "on the other machine")?;
    let first = first_short_id(&sandbox, "10:00")?;

    let output = journey(
        &sandbox,
        &[
            ("10:00", &["end", "beef"]),
            ("10:00", &["end", &first]),
            ("10:01", &["now"]),
        ],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn begin_again_without_a_terminal_lists_the_choices() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = journey(
        &sandbox,
        &[
            ("08:00", &["begin", "--again"]),
            ("09:00", &["begin", "Write offer", "--tag", "offer"]),
            ("10:00", &["end"]),
            ("10:30", &["begin", "--again"]),
            ("11:00", &["begin", "Call ACME"]),
            ("11:20", &["end"]),
            ("11:30", &["begin", "--again"]),
        ],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn a_time_that_happened_twice_takes_the_earlier_without_a_terminal() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = journey(
        &sandbox,
        &[(
            "2026-10-25T12:00[Europe/Berlin]",
            &["begin", "--at", "02:30"],
        )],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}
