//! The settings that shape tracking, and the project ID of a manifest.

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
fn a_hold_without_a_source_takes_the_setting() -> TestResult {
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
fn a_new_device_splits_its_events_as_the_setting_says() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write_config("[storage]\nsplit = \"week\"\n")?;

    _ = run(sandbox.pace_at("09:00").args(["begin", "Write offer"]))?;

    assert_eq!(event_files(&sandbox)?, ["2026/2026-W48.toml"]);
    Ok(())
}

#[test]
fn a_manifest_with_an_id_records_the_project_id() -> TestResult {
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
fn a_manifest_id_must_be_an_id() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write(
        "work/.pace.toml",
        "[project]\nid = \"acme\"\nkey = \"acme-webshop\"\n",
    )?;

    let output = run(sandbox.pace().args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}
