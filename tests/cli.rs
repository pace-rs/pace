use assert_cmd::Command;
use predicates::prelude::predicate;
// use tempfile::{tempdir, TempDir};

pub type TestResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn pace_runner(/*temp_dir: &TempDir*/) -> TestResult<Command> {
    // TODO: when we have implemented init, we can use this to create a new pace project
    // let _repo_dir = temp_dir.path();

    let runner = Command::new(env!("CARGO_BIN_EXE_pace"));

    Ok(runner)
}

// TODO: when we have implemented init, we can use this to create a new pace project
// fn setup() -> TestResult<TempDir> {
//     let temp_dir = tempdir()?;
//     pace_runner(&temp_dir)?.args(["init"]).assert().success();

//     Ok(temp_dir)
// }

#[test]
fn test_version_command_passes() -> TestResult<()> {
    pace_runner()?
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));

    Ok(())
}

#[test]
fn test_help_command_passes() -> TestResult<()> {
    pace_runner()?
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));

    Ok(())
}

// TODO: Test begin command
// #[test]
// fn test_begin_command_passes() -> TestResult<()> {
//     pace_runner()?
//         .args([
//             "-a",
//             "./activity_log.toml",
//             "begin",
//             "This is my task description",
//             "-c",
//             "MyCategory::SubCategory",
//         ])
//         .assert()
//         .success()
//         .stdout(predicate::str::contains("started")); // TODO

//     Ok(())
// }

// TODO: Test end command
// #[test]
// fn test_end_command_passes() -> TestResult<()> {
//     pace_runner()?
//         .args(["-a", "./activity_log.toml", "end"])
//         .assert()
//         .success()
//         .stdout(predicate::str::contains("finished")); // TODO

//     Ok(())
// }

// TODO: Test now command
// #[test]
// fn test_now_command_passes() -> TestResult<()> {
//     pace_runner()?
//         .args(["-a", "./activity_log.toml", "now"])
//         .assert()
//         .success()
//         .stdout(predicate::str::contains("current")); // TODO

//     Ok(())
// }
