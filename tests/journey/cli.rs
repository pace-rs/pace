use crate::support::{Sandbox, run};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn version_names_the_binary_and_its_version() -> TestResult {
    let sandbox = Sandbox::new()?;
    insta::assert_snapshot!(run(sandbox.pace().arg("--version"))?);
    Ok(())
}

#[test]
fn help_describes_pace() -> TestResult {
    let sandbox = Sandbox::new()?;
    insta::assert_snapshot!(run(sandbox.pace().arg("--help"))?);
    Ok(())
}
