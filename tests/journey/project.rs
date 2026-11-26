//! The working directory selects the project: `--project`, then
//! `PACE_PROJECT`, then the nearest manifest, then a path mapping, else none.

use crate::support::{Sandbox, run};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ACME: &str = "[project]\nkey = \"acme-webshop\"\nname = \"Webshop relaunch\"\n";

#[test]
fn without_a_project_pace_says_what_it_looked_for() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox.pace().args(["config", "show", "--origin"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn the_nearest_manifest_above_the_working_directory_wins() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/.pace.toml", "[project]\nkey = \"everything\"\n")?;
    sandbox.write("work/acme/pace.toml", ACME)?;
    sandbox.mkdir("work/acme/src/checkout")?;

    let output = run(sandbox
        .pace_in("acme/src/checkout")
        .args(["config", "show", "--origin"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn config_show_names_the_project() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/.pace.toml", ACME)?;

    let output = run(sandbox.pace().args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn both_manifest_names_in_one_directory_are_refused() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/pace.toml", ACME)?;
    sandbox.write("work/.pace.toml", ACME)?;

    let output = run(sandbox.pace().args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn the_flag_wins_over_everything() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/.pace.toml", ACME)?;

    let output = run(sandbox.pace().env("PACE_PROJECT", "garden").args([
        "--project",
        "taxes",
        "config",
        "show",
        "--origin",
    ]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn the_variable_wins_over_the_manifest() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/.pace.toml", ACME)?;

    let output = run(sandbox
        .pace()
        .env("PACE_PROJECT", "garden")
        .args(["config", "show", "--origin"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn a_path_mapping_covers_a_directory_without_a_manifest() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.mkdir("work/clients/repo/src")?;
    sandbox.write_config(&format!(
        "[project-paths]\n'{}' = \"clients\"\n'{}' = \"client-repo\"\n",
        sandbox.path("work/clients").display(),
        sandbox.path("work/clients/repo").display()
    ))?;

    let output = run(sandbox
        .pace_in("clients/repo/src")
        .args(["config", "show", "--origin"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn a_manifest_wins_over_a_path_mapping() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/acme/.pace.toml", ACME)?;
    sandbox.write_config(&format!(
        "[project-paths]\n'{}' = \"client-repo\"\n",
        sandbox.path("work").display()
    ))?;

    let output = run(sandbox.pace_in("acme").args(["config", "show", "--origin"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_path_mapping_may_start_at_home() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.mkdir("work/repo")?;
    sandbox.write_config("[project-paths]\n\"~/work/repo\" = \"client-repo\"\n")?;

    let output = run(sandbox
        .pace_in("repo")
        .env("HOME", sandbox.path(""))
        .args(["config", "show", "--origin"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn a_relative_path_mapping_is_refused() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write_config("[project-paths]\n\"work/repo\" = \"client-repo\"\n")?;

    let output = run(sandbox.pace().args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn an_unknown_key_in_the_manifest_names_the_line() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write(
        "work/.pace.toml",
        "version = 1\n\n[project]\nkey = \"acme-webshop\"\nclient = \"acme\"\n",
    )?;

    let output = run(sandbox.pace().args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn a_manifest_key_must_be_a_project_key() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/.pace.toml", "[project]\nkey = \"Acme Webshop\"\n")?;

    let output = run(sandbox.pace().args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn a_manifest_without_a_key_is_refused() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/.pace.toml", "[project]\nname = \"Webshop\"\n")?;

    let output = run(sandbox.pace().args(["config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn a_flag_that_is_no_project_key_is_a_usage_error() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox
        .pace()
        .args(["--project", "Acme Webshop", "config", "show"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}
