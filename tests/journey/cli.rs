use assert_cmd::Command;
use predicates::prelude::predicate;
// use similar_asserts::assert_eq;
use insta_cmd::assert_cmd_snapshot;
use std::{path::PathBuf, process::Command as StdCommand};
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
            "--config",
            "tests/fixtures/configs/pace.toml",
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
fn test_help_command_passes() -> TestResult<()> {
    _ = pace_runner()?
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));

    Ok(())
}

#[test]
#[ignore = "Snapshot is only tested in CI, due to time zones being involved"]
fn test_begin_snapshot_passes() -> TestResult<()> {
    let dir_str = temp_dir_with("activities.pace.toml")?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--config",
        "tests/fixtures/configs/pace.toml",
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
        "--config",
        "tests/fixtures/configs/pace.toml",
        "--activity-log-file",
        &dir_str,
        "now"
    ]));

    Ok(())
}

#[test]
#[ignore = "Snapshot is only tested in CI, due to time zones being involved"]
fn test_now_with_active_activity_snapshot_passes() -> TestResult<()> {
    let dir_str = temp_dir_with("activities.pace.toml")?;

    fixture_begin_activity(&dir_str)?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--config",
        "tests/fixtures/configs/pace.toml",
        "--activity-log-file",
        &dir_str,
        "now"
    ]));

    Ok(())
}

#[test]
#[ignore = "Snapshot is only tested in CI, due to time zones being involved"]
fn test_end_with_active_activity_snapshot_passes() -> TestResult<()> {
    let dir_str = temp_dir_with("activities.pace.toml")?;

    fixture_begin_activity(&dir_str)?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--config",
        "tests/fixtures/configs/pace.toml",
        "--activity-log-file",
        &dir_str,
        "end"
    ]));

    Ok(())
}

#[test]
#[ignore = "Snapshot is only tested in CI, due to time zones being involved"]
fn test_hold_with_active_activity_snapshot_passes() -> TestResult<()> {
    let dir_str = temp_dir_with("activities.pace.toml")?;

    fixture_begin_activity(&dir_str)?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--config",
        "tests/fixtures/configs/pace.toml",
        "--activity-log-file",
        &dir_str,
        "hold"
    ]));

    Ok(())
}

#[test]
#[ignore = "Snapshot is only tested in CI, due to time zones being involved"]
fn test_resume_with_held_activity_snapshot_passes() -> TestResult<()> {
    let dir_str = temp_dir_with("activities.pace.toml")?;

    fixture_begin_activity(&dir_str)?;

    StdCommand::new(env!("CARGO_BIN_EXE_pace"))
        .args([
            "--config",
            "tests/fixtures/configs/pace.toml",
            "--activity-log-file",
            &dir_str,
            "hold",
        ])
        .output()?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--config",
        "tests/fixtures/configs/pace.toml",
        "--activity-log-file",
        &dir_str,
        "resume"
    ]));

    Ok(())
}

#[test]
#[ignore = "Snapshot is only tested in CI, due to time zones being involved"]
fn test_adjust_activity_snapshot_passes() -> TestResult<()> {
    let dir_str = temp_dir_with("activities.pace.toml")?;

    fixture_begin_activity(&dir_str)?;

    StdCommand::new(env!("CARGO_BIN_EXE_pace"))
        .args([
            "--config",
            "tests/fixtures/configs/pace.toml",
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
        "--config",
        "tests/fixtures/configs/pace.toml",
        "--activity-log-file",
        &dir_str,
        "now",
    ]));

    Ok(())
}

#[test]
#[ignore = "Snapshot is only tested in CI, due to time zones being involved"]
fn test_reflect_from_to_snapshot_passes() -> TestResult<()> {
    let path = PathBuf::from("./tests/fixtures/activity_tracker/activities.pace.toml");
    let activities = path.to_str().ok_or("Could not convert path to string")?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--config",
        "tests/fixtures/configs/pace.toml",
        "--activity-log-file",
        &activities,
        "reflect",
        "--from",
        "2024-02-26",
        "--to",
        "2024-02-28",
    ]));

    Ok(())
}

#[test]
#[ignore = "Snapshot is only tested in CI, due to time zones being involved"]
fn test_reflect_date_snapshot_passes() -> TestResult<()> {
    let path = PathBuf::from("./tests/fixtures/activity_tracker/activities.pace.toml");
    let activities = path.to_str().ok_or("Could not convert path to string")?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--config",
        "tests/fixtures/configs/pace.toml",
        "--activity-log-file",
        &activities,
        "reflect",
        "--date",
        "2024-02-26",
    ]));

    Ok(())
}

#[test]
fn test_reflect_today_snapshot_passes() -> TestResult<()> {
    let path = PathBuf::from("./tests/fixtures/activity_tracker/activities.pace.toml");
    let activities = path.to_str().ok_or("Could not convert path to string")?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--config",
        "tests/fixtures/configs/pace.toml",
        "--activity-log-file",
        &activities,
        "reflect",
    ]));

    Ok(())
}

#[test]
fn test_reflect_current_week_snapshot_passes() -> TestResult<()> {
    let path = PathBuf::from("./tests/fixtures/activity_tracker/activities.pace.toml");
    let activities = path.to_str().ok_or("Could not convert path to string")?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--config",
        "tests/fixtures/configs/pace.toml",
        "--activity-log-file",
        &activities,
        "reflect",
        "current-week",
    ]));

    Ok(())
}

#[test]
fn test_reflect_current_month_snapshot_passes() -> TestResult<()> {
    let path = PathBuf::from("./tests/fixtures/activity_tracker/activities.pace.toml");
    let activities = path.to_str().ok_or("Could not convert path to string")?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--config",
        "tests/fixtures/configs/pace.toml",
        "--activity-log-file",
        &activities,
        "reflect",
        "current-month",
    ]));

    Ok(())
}

#[test]
#[ignore = "Snapshot is only tested in CI, due to time zones being involved"]
fn test_reflect_from_to_filter_category_glob_front_snapshot_passes() -> TestResult<()> {
    let path = PathBuf::from("./tests/fixtures/activity_tracker/activities.pace.toml");
    let activities = path.to_str().ok_or("Could not convert path to string")?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--config",
        "tests/fixtures/configs/pace.toml",
        "--activity-log-file",
        &activities,
        "reflect",
        "--from",
        "2024-02-26",
        "--to",
        "2024-02-28",
        "--category",
        "*pace",
    ]));

    Ok(())
}

#[test]
#[ignore = "Snapshot is only tested in CI, due to time zones being involved"]
fn test_reflect_from_to_filter_category_case_sensitive_snapshot_passes() -> TestResult<()> {
    let path = PathBuf::from("./tests/fixtures/activity_tracker/activities.pace.toml");
    let activities = path.to_str().ok_or("Could not convert path to string")?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--config",
        "tests/fixtures/configs/pace.toml",
        "--activity-log-file",
        &activities,
        "reflect",
        "--from",
        "2024-02-26",
        "--to",
        "2024-02-28",
        "--category",
        "*Pace",
        "--case-sensitive",
    ]));

    Ok(())
}

#[test]
#[ignore = "Snapshot is only tested in CI, due to time zones being involved"]
fn test_reflect_from_to_filter_category_full_snapshot_passes() -> TestResult<()> {
    let path = PathBuf::from("./tests/fixtures/activity_tracker/activities.pace.toml");
    let activities = path.to_str().ok_or("Could not convert path to string")?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--config",
        "tests/fixtures/configs/pace.toml",
        "--activity-log-file",
        &activities,
        "reflect",
        "--from",
        "2024-02-26",
        "--to",
        "2024-02-28",
        "--category",
        "development::pace",
    ]));

    Ok(())
}

#[test]
#[ignore = "Snapshot is only tested in CI, due to time zones being involved"]
fn test_reflect_from_to_filter_category_glob_back_snapshot_passes() -> TestResult<()> {
    let path = PathBuf::from("./tests/fixtures/activity_tracker/activities.pace.toml");
    let activities = path.to_str().ok_or("Could not convert path to string")?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--config",
        "tests/fixtures/configs/pace.toml",
        "--activity-log-file",
        &activities,
        "reflect",
        "--from",
        "2024-02-26",
        "--to",
        "2024-02-28",
        "--category",
        "dev*",
    ]));

    Ok(())
}

#[test]
#[ignore = "Snapshot is only tested in CI, due to time zones being involved"]
fn test_reflect_from_to_filter_category_glob_back_case_sensitive_snapshot_passes() -> TestResult<()>
{
    let path = PathBuf::from("./tests/fixtures/activity_tracker/activities.pace.toml");
    let activities = path.to_str().ok_or("Could not convert path to string")?;

    assert_cmd_snapshot!(StdCommand::new(env!("CARGO_BIN_EXE_pace")).args([
        "--config",
        "tests/fixtures/configs/pace.toml",
        "--activity-log-file",
        &activities,
        "reflect",
        "--from",
        "2024-02-26",
        "--to",
        "2024-02-28",
        "--category",
        "Dev*",
        "--case-sensitive",
    ]));

    Ok(())
}

// Test use cases with commands and take the activity log into account as well
//
// Use cases to test:
// - [ ] `pace begin` with tags and category
// - [ ] `pace begin` with active activity
// - [ ] `pace begin` with held activity
// - [ ] `pace adjust` with active activity
// - [ ] `pace now` with no activities
// - [ ] `pace now` with active activity
// - [ ] `pace end` with active activity
// - [ ] `pace hold` with active activity
// - [ ] `pace hold` with no activities
// - [ ] `pace resume` with held activity
