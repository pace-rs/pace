//! `pace doctor` checks the directories, the files, the device and setup,
//! names every problem with its place and its fix, and changes nothing.

use std::path::PathBuf;

use crate::support::{Sandbox, run};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The event file of November 2026 of the one device in the sandbox.
fn november_file(sandbox: &Sandbox) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let device = std::fs::read_dir(sandbox.path("home/events"))?
        .next()
        .ok_or("pace wrote no device directory")??;
    Ok(device.path().join("2026/2026-11.toml"))
}

/// Two invalid events after the valid ones of the file: one of a type
/// pace doesn't know, and one with a key it doesn't know.
const TWO_INVALID_EVENTS: &str = r#"
[[event]]
id = "0193a8d0-0000-7000-8000-00000000e001"
type = "entry.bogun"
v = 1
batch = "0193a8d0-0000-7000-8000-00000000b001"
at = "2026-11-26T11:00:00+01:00[Europe/Berlin]"
hlc = "2026-11-26T10:00:00.000Z/0"
entry = "0193a8d0-0000-7000-8000-00000000a001"

[[event]]
id = "0193a8d0-0000-7000-8000-00000000e002"
type = "entry.ended"
v = 1
batch = "0193a8d0-0000-7000-8000-00000000b002"
at = "2026-11-26T11:30:00+01:00[Europe/Berlin]"
hlc = "2026-11-26T10:30:00.000Z/0"
entry = "0193a8d0-0000-7000-8000-00000000a001"
whereabouts = "office"
"#;

/// A sandbox with one entry, and two invalid events in its event file.
fn sandbox_with_two_invalid_events() -> Result<Sandbox, Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    _ = sandbox.journey(&[("09:00", &["begin", "Write offer"]), ("10:00", &["end"])])?;
    let file = november_file(&sandbox)?;
    let mut text = std::fs::read_to_string(&file)?;
    text.push_str(TWO_INVALID_EVENTS);
    std::fs::write(file, text)?;
    Ok(sandbox)
}

#[test]
fn clean_home_has_no_problems() -> TestResult {
    let sandbox = Sandbox::new()?;

    let transcript = sandbox.journey(&[
        ("08:00", &["doctor"]),
        ("09:00", &["begin", "Write offer"]),
        ("10:00", &["end"]),
        ("10:05", &["doctor"]),
    ])?;
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}

#[test]
fn event_file_with_two_invalid_events_lists_both_with_their_lines() -> TestResult {
    let sandbox = sandbox_with_two_invalid_events()?;

    let transcript = sandbox.journey(&[("12:00", &["now"]), ("12:00", &["doctor"])])?;
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}

#[test]
fn findings_as_json_in_schema() -> TestResult {
    let sandbox = sandbox_with_two_invalid_events()?;
    sandbox.write("machine/setup.toml", "version = 1\nstep = \"roles\"\n")?;

    let transcript = sandbox.journey(&[("12:00", &["doctor", "--json"])])?;
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}

#[test]
fn machine_directory_inside_data_directory_is_error() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox
        .pace()
        .env("PACE_MACHINE_DIR", sandbox.path("home/machine"))
        .arg("doctor"))?;

    assert!(!sandbox.path("home").exists(), "doctor changes nothing");
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn unfinished_setup_is_warning_that_names_pace_setup() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("machine/setup.toml", "version = 1\nstep = \"roles\"\n")?;

    let output = run(sandbox.pace().arg("doctor"))?;

    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn broken_files_that_stop_commands_are_all_named() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write(
        "home/roles.toml",
        "version = 1\n\n[work]\nid = \"0193a7c4-0000-7000-8000-000000000001\"\nkind = \"aera\"\nname = \"Work\"\n",
    )?;
    sandbox.write("work/pace.toml", "version = 1\nkey = \"Web Shop\"\n")?;
    sandbox.write_config("version = 1\nweek-start = \"someday\"\n")?;

    let output = run(sandbox.pace().arg("doctor"))?;

    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn device_id_written_by_machine_copy_is_named() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = sandbox.journey(&[("09:00", &["begin", "Write offer"])])?;
    // A backup of the machine directory restored onto a second machine.
    sandbox.mkdir("copy")?;
    for file in ["device.toml", "cursors.toml"] {
        _ = std::fs::copy(
            sandbox.path("machine").join(file),
            sandbox.path("copy").join(file),
        )?;
    }
    let copy = run(sandbox
        .pace_at("10:00")
        .env("PACE_MACHINE_DIR", sandbox.path("copy"))
        .arg("end"))?;
    assert!(copy.starts_with("success: true"), "{copy}");

    let transcript = sandbox.journey(&[("11:00", &["doctor"])])?;
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}
