//! `pace setup` guides through the settings, the roles, the clients and
//! the project of the working directory. It asks on a terminal only, and
//! without one it names the commands that do its parts.

use crate::support::{Sandbox, run};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn setup_without_terminal_exits_2_and_names_four_commands() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox.pace().arg("setup"))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn first_use_note_names_pace_setup() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[("09:00", &["begin", "Write offer"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn setup_help_names_start_over() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = run(sandbox.pace().args(["setup", "--help"]))?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}
