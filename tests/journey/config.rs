//! Settings come from defaults, the global file, the environment and
//! flags, and pace says which layer each value comes from.

use crate::support::{Sandbox, run};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn defaults_apply_without_settings() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox.pace().args(["config", "show", "--origin"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn the_global_file_names_its_line() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox
        .write_config("version = 1\n\ntime-zone = \"Europe/Berlin\"\nweek-start = \"sunday\"\n")?;

    let output = run(sandbox.pace().args(["config", "show", "--origin"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn the_environment_wins_over_the_file() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write_config("week-start = \"sunday\"\n")?;

    let output = run(sandbox
        .pace()
        .env("PACE_WEEK_START", "saturday")
        .env("PACE_TIME_ZONE", "America/New_York")
        .args(["config", "show", "--origin"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn shown_settings_read_back_the_same() -> TestResult {
    let sandbox = Sandbox::new()?;
    let shown = run(sandbox
        .pace()
        .env("PACE_TIME_ZONE", "Asia/Tokyo")
        .args(["config", "show"]))?;
    let toml = shown
        .split("----- stdout -----\n")
        .nth(1)
        .and_then(|rest| rest.split("\n----- stderr -----").next())
        .ok_or("no stdout")?;
    sandbox.write_config(toml)?;

    let again = run(sandbox.pace().args(["config", "show"]))?;

    assert_eq!(again, shown);
    assert!(again.contains("time-zone = \"Asia/Tokyo\""), "{again}");
    Ok(())
}

#[test]
fn an_unknown_key_names_the_line_and_the_closest_key() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write_config("version = 1\ntime-zon = \"Europe/Berlin\"\n")?;

    let output = run(sandbox.pace().args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn an_unknown_table_points_to_the_known_settings() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write_config("[storage]\nbackend = \"files\"\n")?;

    let output = run(sandbox.pace().args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn an_unknown_time_zone_suggests_the_closest() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write_config("time-zone = \"Europe/Berln\"\n")?;

    let output = run(sandbox.pace().args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn a_value_of_the_wrong_type_says_what_fits() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write_config("week-start = 1\n")?;

    let output = run(sandbox.pace().args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn a_wrong_value_in_the_environment_names_the_variable() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox
        .pace()
        .env("PACE_WEEK_START", "someday")
        .args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn broken_toml_points_at_the_spot() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write_config("time-zone = \"Europe/Berlin\nweek-start = \"monday\"\n")?;

    let output = run(sandbox.pace().args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn a_newer_file_version_is_refused() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write_config("version = 2\n")?;

    let output = run(sandbox.pace().args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn a_given_config_file_must_exist() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox
        .pace()
        .env("PACE_CONFIG", sandbox.path("config/typo.toml"))
        .args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn an_unknown_pace_variable_warns_and_goes_on() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox
        .pace()
        .env("PACE_TIMEZONE", "Europe/Berlin")
        .args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}
