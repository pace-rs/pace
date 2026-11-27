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
