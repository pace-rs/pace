use assert_cmd::Command;
use predicates::prelude::predicate;

#[test]
fn test_version_command() {
    let mut cmd = Command::cargo_bin("pace").unwrap();
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("0.4.0"));
}

#[test]
fn test_help_command() {
    let mut cmd = Command::cargo_bin("pace").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_begin_command() {
    let mut cmd = Command::cargo_bin("pace").unwrap();
    cmd.args([
        "-a",
        "./activity_log.toml",
        "begin",
        "This is my task description",
        "-c",
        "MyCategory::SubCategory",
    ]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("begin"));
}
