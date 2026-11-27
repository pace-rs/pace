//! `pace store split`: this device's event files, split by another period.

use crate::support::{Sandbox, run, short_id_after};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The event files of the only device, relative to its directory.
fn files(sandbox: &Sandbox) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let device = std::fs::read_dir(sandbox.path("home/events"))?
        .next()
        .ok_or("a device directory")??
        .path();
    let mut files = Vec::new();
    let mut pending = vec![device.clone()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let relative = path.strip_prefix(&device)?;
                files.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    files.sort();
    Ok(files)
}

#[test]
fn split_rewrites_this_devices_files() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox
        .pace_at("2026-10-30T09:00[Europe/Berlin]")
        .args(["log", "October", "--from", "08:00", "--to", "09:00"]))?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "November"]))?;

    let output = sandbox.journey(&[
        ("09:30", &["store", "split", "--by", "week"]),
        ("09:31", &["store", "split", "--by", "week"]),
        ("09:32", &["store", "split", "--by", "fortnight"]),
        ("09:33", &["now"]),
    ])?;

    assert_eq!(
        files(&sandbox)?,
        ["2026/2026-W44.toml", "2026/2026-W48.toml", "device.toml"]
    );
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn split_without_by_follows_the_setting() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "November"]))?;
    sandbox.write_config("[storage]\nsplit = \"day\"\n")?;

    let output = sandbox.journey(&[("09:30", &["store", "split"])])?;

    assert_eq!(files(&sandbox)?, ["2026/11/2026-11-26.toml", "device.toml"]);
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn split_again_finishes_split_that_stopped() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox
        .pace_at("2026-10-30T09:00[Europe/Berlin]")
        .args(["log", "October", "--from", "08:00", "--to", "09:00"]))?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "November"]))?;
    // What a stop in the middle of a split to weeks leaves.
    let device = std::fs::read_dir(sandbox.path("home/events"))?
        .next()
        .ok_or("a device directory")??
        .path();
    _ = std::fs::copy(
        device.join("2026/2026-10.toml"),
        device.join("2026/2026-W44.toml"),
    )?;

    let output = sandbox.journey(&[
        ("09:30", &["store", "split", "--by", "month"]),
        ("09:31", &["store", "split", "--by", "month"]),
    ])?;

    assert_eq!(
        files(&sandbox)?,
        ["2026/2026-10.toml", "2026/2026-11.toml", "device.toml"]
    );
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn split_with_nothing_to_do_writes_nothing() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox.pace_at("09:00").args(["store", "split"]))?;

    assert!(output.contains("exit_code: 3"), "{output}");
    assert!(!sandbox.path("machine").exists());
    assert!(!sandbox.path("home").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn failed_cleaning_warns_and_next_command_cleans() -> TestResult {
    use std::os::unix::fs::PermissionsExt as _;

    let sandbox = Sandbox::new()?;
    sandbox.write_config("[storage]\nsplit = \"day\"\n")?;
    let logged = run(sandbox
        .pace_at("2026-10-30T12:00[Europe/Berlin]")
        .args(["log", "secret", "--from", "09:00", "--to", "10:00"]))?;
    let entry = short_id_after(&logged, "Logged ")?;
    // A desktop on the same data forgets the entry; its events stay in the
    // laptop's files until the laptop writes.
    _ = run(sandbox
        .pace_at("08:00")
        .env("PACE_MACHINE_DIR", sandbox.path("desktop"))
        .args(["forget", &entry, "--for-good"]))?;
    let october = std::fs::read_dir(sandbox.path("home/events"))?
        .map(|entry| entry.map(|entry| entry.path().join("2026/10")))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .find(|october| {
            std::fs::read_to_string(october.join("2026-10-30.toml"))
                .is_ok_and(|text| text.contains("secret"))
        })
        .ok_or("the laptop's October")?;
    let mode = std::fs::Permissions::from_mode;
    std::fs::set_permissions(&october, mode(0o555))?;

    let output = sandbox.journey(&[("09:00", &["begin", "Review"])]);
    std::fs::set_permissions(&october, mode(0o755))?;
    let output = output?;
    let ended = run(sandbox.pace_at("10:00").arg("end"))?;

    assert!(ended.contains("exit_code: 0"), "{ended}");
    assert!(
        !std::fs::read_to_string(october.join("2026-10-30.toml"))
            .is_ok_and(|text| text.contains("secret")),
        "the next command cleans the file"
    );
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn split_without_events_says_what_comes_next() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[("09:00", &["store", "split", "--by", "year"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}
