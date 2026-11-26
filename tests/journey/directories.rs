//! Where pace keeps its files: config file, data (`PACE_HOME`) and the
//! machine directory.

use crate::support::{Sandbox, run};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn config_show_lists_the_three_places() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox.pace().args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn the_origin_says_where_each_place_comes_from() -> TestResult {
    let sandbox = Sandbox::new()?;
    std::fs::create_dir(sandbox.path("home"))?;

    let output = run(sandbox.pace().args(["config", "show", "--origin"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn flag_wins_over_the_environment() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox
        .pace()
        .arg("--home")
        .arg(sandbox.path("elsewhere"))
        .args(["config", "show", "--origin"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn relative_path_starts_in_the_working_directory() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox.pace().args(["--home", "data", "config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn an_empty_variable_counts_as_unset() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox
        .pace()
        .env("PACE_HOME", "")
        .arg("--home")
        .arg(sandbox.path("flag"))
        .args(["config", "show", "--origin"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn machine_directory_inside_the_data_is_refused() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox
        .pace()
        .env("PACE_MACHINE_DIR", sandbox.path("home/machine"))
        .args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn plain_errors_have_no_symbols() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox
        .pace()
        .env("PACE_MACHINE_DIR", sandbox.path("home"))
        .args(["--plain", "config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn linux_defaults_follow_xdg() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox
        .bare_pace()
        .env("HOME", sandbox.path("me"))
        .env("XDG_CONFIG_HOME", sandbox.path("me/.config"))
        .env("XDG_DATA_HOME", sandbox.path("me/.local/share"))
        .env("XDG_STATE_HOME", sandbox.path("me/.local/state"))
        .args(["config", "show", "--origin"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}
