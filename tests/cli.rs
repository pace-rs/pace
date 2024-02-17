use assert_cmd::Command;
// use pace_core::ActivityLog;
use predicates::prelude::predicate;
// use tempfile::tempdir;

pub type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

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

// TODO!: Test begin command
// #[test]
// fn test_begin_command_passes() -> TestResult<()> {
//     let activity_log_file = tempdir()?.into_path().join("activity_log.toml");

//     let desc = "Test description";
//     let category = "Test::Category";
//     let time = "22:00";

//     if activity_log_file.exists() {
//         std::fs::remove_file(&activity_log_file)?;
//     }

//     std::fs::write(&activity_log_file, "")?;

//     _ = pace_runner()?
//         .args([
//             "-a",
//             activity_log_file.as_path().to_str().unwrap(),
//             "begin",
//             desc,
//             "-c",
//             category,
//             "-t",
//             time,
//         ])
//         .assert()
//         .success()
//         .stdout(predicate::str::contains("started"));

//     let contents = std::fs::read_to_string(&activity_log_file)?;

//     let activity_log = toml::from_str::<ActivityLog>(&contents)?;

//     assert_eq!(activity_log.activities().len(), 1);

//     let activity = activity_log.activities().front().unwrap();

//     assert_eq!(activity.description(), &Some(desc.to_string()));
//     assert_eq!(activity.category(), &Some(category.to_string()));
//     assert_eq!(format!("{:?}", activity.begin()), time);

//     if activity_log_file.exists() {
//         std::fs::remove_file(&activity_log_file)?;
//     }

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
