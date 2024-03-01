use assert_cmd::Command;
use predicates::prelude::predicate;
// use similar_asserts::assert_eq;
use insta_cmd::assert_cmd_snapshot;
use std::process::Command as StdCommand;
use tempfile::TempDir;

// use pace_core::ActivityLog;

pub type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

pub fn pace_runner(/*temp_dir: &TempDir*/) -> TestResult<Command> {
    // TODO: when we have implemented init, we can use this to create a new pace project
    // let _repo_dir = temp_dir.path();

    let runner = Command::new(env!("CARGO_BIN_EXE_pace"));

    Ok(runner)
}

fn temp_dir_with(path: &str) -> TestResult<String> {
    // create directory
    std::fs::create_dir_all("./tests/generated")?;

    let tmp_dir = TempDir::new_in("./tests/generated")?.into_path().join(path);
    let dir_str = tmp_dir.to_string_lossy().to_string();

    Ok(dir_str)
}

fn fixture_begin_activity(dir_str: &String) -> TestResult<()> {
    StdCommand::new(env!("CARGO_BIN_EXE_pace"))
        .args([
            "--activity-log-file",
            dir_str,
            "begin",
            "MyActivity",
            "--tags",
            "tag1,tag2",
            "--category",
            "MyCategory::SubCategory",
        ])
        .output()?;

    Ok(())
}

#[test]
fn test_version_command_passes() -> TestResult<()> {
    _ = pace_runner()?
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));

    Ok(())
}

#[test]
fn test_subcommand_not_provided_snapshot_passes() {
    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")));
}

#[test]
fn test_help_snapshot_passes() {
    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).arg("help"));
}

#[test]
fn test_version_snapshot_passes() {
    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).arg("--version"));
}

#[test]
fn test_begin_snapshot_passes() -> TestResult<()> {
    let dir_str = temp_dir_with("activities.pace.toml")?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--activity-log-file",
        &dir_str,
        "begin",
        "MyActivity",
        "--tags",
        "tag1,tag2",
        "--category",
        "MyCategory::SubCategory",
    ]));

    Ok(())
}

#[test]
fn test_now_no_activities_snapshot_passes() -> TestResult<()> {
    let dir_str = temp_dir_with("activities.pace.toml")?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--activity-log-file",
        &dir_str,
        "now"
    ]));

    Ok(())
}

#[test]
fn test_now_with_active_activity_snapshot_passes() -> TestResult<()> {
    let dir_str = temp_dir_with("activities.pace.toml")?;

    fixture_begin_activity(&dir_str)?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--activity-log-file",
        &dir_str,
        "now"
    ]));

    Ok(())
}

#[test]
fn test_end_with_active_activity_snapshot_passes() -> TestResult<()> {
    let dir_str = temp_dir_with("activities.pace.toml")?;

    fixture_begin_activity(&dir_str)?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--activity-log-file",
        &dir_str,
        "end"
    ]));

    Ok(())
}

#[test]
fn test_hold_with_active_activity_snapshot_passes() -> TestResult<()> {
    let dir_str = temp_dir_with("activities.pace.toml")?;

    fixture_begin_activity(&dir_str)?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--activity-log-file",
        &dir_str,
        "hold"
    ]));

    Ok(())
}

#[test]
fn test_resume_with_held_activity_snapshot_passes() -> TestResult<()> {
    let dir_str = temp_dir_with("activities.pace.toml")?;

    fixture_begin_activity(&dir_str)?;

    StdCommand::new(env!("CARGO_BIN_EXE_pace"))
        .args(["--activity-log-file", &dir_str, "hold"])
        .output()?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--activity-log-file",
        &dir_str,
        "resume"
    ]));

    Ok(())
}

#[test]
fn test_adjust_activity_snapshot_passes() -> TestResult<()> {
    let dir_str = temp_dir_with("activities.pace.toml")?;

    fixture_begin_activity(&dir_str)?;

    StdCommand::new(env!("CARGO_BIN_EXE_pace"))
        .args([
            "--activity-log-file",
            &dir_str,
            "adjust",
            "--description",
            "NewDescription",
            "--category",
            "NewCategory::SubCategory",
        ])
        .output()?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--activity-log-file",
        &dir_str,
        "now",
    ]));

    Ok(())
}
