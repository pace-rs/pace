//! The settings that shape tracking, `config get` and `config set`, and the
//! project ID of a manifest.

use std::fmt::Write as _;

use crate::support::{Sandbox, run};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The paths of the event files below `home/events`, without the device.
fn event_files(sandbox: &Sandbox) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    let mut pending = vec![sandbox.path("home/events")];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.file_name().is_some_and(|name| name != "device.toml") {
                let relative = path.strip_prefix(sandbox.path("home/events"))?;
                let parts: Vec<String> = relative
                    .components()
                    .skip(1)
                    .map(|part| part.as_os_str().to_string_lossy().into_owned())
                    .collect();
                files.push(parts.join("/"));
            }
        }
    }
    files.sort();
    Ok(files)
}

#[test]
fn hold_without_source_takes_the_setting() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write_config("[holds]\ndefault-source = \"outer\"\n")?;

    let output = sandbox.journey(&[("09:00", &["begin", "Write offer"]), ("10:00", &["hold"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn begin_again_offers_the_most_recent_count() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write_config("most-recent-count = 1\n")?;

    let output = sandbox.journey(&[
        ("09:00", &["begin", "Write offer"]),
        ("10:00", &["begin", "Call ACME"]),
        ("10:30", &["end"]),
        ("11:00", &["begin", "--again"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn new_device_splits_its_events_as_the_setting_says() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write_config("[storage]\nsplit = \"week\"\n")?;

    _ = run(sandbox.pace_at("09:00").args(["begin", "Write offer"]))?;

    assert_eq!(event_files(&sandbox)?, ["2026/2026-W48.toml"]);
    Ok(())
}

#[test]
fn manifest_with_an_id_records_the_project_id() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write(
        "work/.pace.toml",
        "[project]\nid = \"0193a7c4-0000-7000-8000-000000000001\"\nkey = \"acme-webshop\"\n",
    )?;

    let output = sandbox.journey(&[
        ("09:00", &["begin", "Write offer"]),
        ("09:30", &["now", "--json"]),
    ])?;
    let events = std::fs::read_to_string(
        std::fs::read_dir(sandbox.path("home/events"))?
            .next()
            .ok_or("a device directory")??
            .path()
            .join("2026/2026-11.toml"),
    )?;

    assert!(
        events.contains("project = \"0193a7c4-0000-7000-8000-000000000001\""),
        "{events}"
    );
    assert!(!events.contains("project-key"), "{events}");
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn manifest_id_must_be_an_id() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write(
        "work/.pace.toml",
        "[project]\nid = \"acme\"\nkey = \"acme-webshop\"\n",
    )?;

    let output = run(sandbox.pace().args(["config", "show"]))?;
    assert!(!output.contains("pace project"), "{output}");
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn manifest_id_that_is_no_text_names_no_missing_command() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write(
        "work/.pace.toml",
        "[project]\nid = 7\nkey = \"acme-webshop\"\n",
    )?;

    let output = run(sandbox.pace().args(["config", "show"]))?;

    assert!(output.contains("`id` takes the project's ID"), "{output}");
    assert!(!output.contains("pace project"), "{output}");
    Ok(())
}

#[test]
fn config_show_lists_reports_time_zone() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write_config("[reports]\ntime-zone = \"Europe/Vienna\"\n")?;

    let output = run(sandbox.pace().args(["config", "show", "--origin"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn config_get_prints_effective_value_and_with_origin_where_it_comes_from() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write_config("version = 1\nweek-start = \"Sunday\"\n")?;

    let output = sandbox.journey(&[
        ("09:00", &["config", "get", "week-start"]),
        ("09:00", &["config", "get", "week-start", "--origin"]),
        ("09:00", &["config", "get", "most-recent-count", "--origin"]),
        ("09:00", &["config", "get", "reports.time-zone"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn config_get_names_variable_that_sets_the_value() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox
        .pace()
        .env("PACE_HOLDS__DEFAULT_SOURCE", "outer")
        .args(["config", "get", "holds.default-source", "--origin"]))?;

    assert!(
        output.contains("outer  # from PACE_HOLDS__DEFAULT_SOURCE"),
        "{output}"
    );
    Ok(())
}

#[test]
fn config_get_of_unknown_key_names_the_closest() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[
        ("09:00", &["config", "get", "time-zon"]),
        ("09:00", &["config", "get", "project-paths"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

/// A global file edited by hand, with comments, a table and a mapping.
const HAND_WRITTEN_CONFIG: &str = "\
# My settings
version = 1

# The office
time-zone = \"Europe/Berlin\" # Berlin
week-start = \"Sunday\"

[storage]
split = \"week\"
";

#[test]
fn config_set_writes_one_field_and_keeps_the_comments() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write_config(HAND_WRITTEN_CONFIG)?;

    let mut transcript = String::new();
    for args in [
        ["config", "set", "time-zone", "europe/vienna"],
        ["config", "set", "holds.default-source", "outer"],
        ["config", "set", "most-recent-count", "5"],
    ] {
        writeln!(transcript, "$ pace {}", args.join(" "))?;
        transcript.push_str(&run(sandbox.pace().args(args))?);
        transcript.push('\n');
    }
    let file = std::fs::read_to_string(sandbox.path("config/pace.global.toml"))?;
    write!(transcript, "----- pace.global.toml -----\n{file}")?;
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}

#[test]
fn config_set_refuses_bad_value_and_names_what_fits() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write_config(HAND_WRITTEN_CONFIG)?;

    let output = sandbox.journey(&[
        ("09:00", &["config", "set", "week-start", "mondy"]),
        ("09:00", &["config", "set", "most-recent-count", "0"]),
        ("09:00", &["config", "set", "storage.split", "fortnight"]),
        ("09:00", &["config", "set", "time-zon", "Europe/Vienna"]),
        ("09:00", &["config", "set", "project-paths", "~/repo"]),
    ])?;

    let file = std::fs::read_to_string(sandbox.path("config/pace.global.toml"))?;
    assert_eq!(file, HAND_WRITTEN_CONFIG);
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn config_set_of_value_the_file_holds_has_nothing_to_change() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write_config(HAND_WRITTEN_CONFIG)?;

    let output = sandbox.journey(&[("09:00", &["config", "set", "week-start", "sunday"])])?;

    let file = std::fs::read_to_string(sandbox.path("config/pace.global.toml"))?;
    assert_eq!(file, HAND_WRITTEN_CONFIG);
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn config_set_says_when_variable_overrides_the_setting() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox.pace().env("PACE_TIME_ZONE", "Asia/Tokyo").args([
        "config",
        "set",
        "time-zone",
        "Europe/Vienna",
    ]))?;

    assert!(output.contains("PACE_TIME_ZONE overrides it"), "{output}");
    let file = std::fs::read_to_string(sandbox.path("config/pace.global.toml"))?;
    assert!(file.contains("time-zone = \"Europe/Vienna\""), "{file}");
    Ok(())
}

#[test]
fn config_set_creates_missing_config_file() -> TestResult {
    let sandbox = Sandbox::new()?;
    std::fs::remove_file(sandbox.path("config/pace.global.toml"))?;

    _ = run(sandbox
        .pace()
        .args(["config", "set", "reports.time-zone", "Europe/Vienna"]))?;

    let file = std::fs::read_to_string(sandbox.path("config/pace.global.toml"))?;
    assert_eq!(
        file,
        "version = 1\n\n[reports]\ntime-zone = \"Europe/Vienna\"\n"
    );
    Ok(())
}
