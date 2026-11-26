use crate::support::{pace, run};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn version_names_the_binary_and_its_version() -> TestResult {
    insta::assert_snapshot!(run(pace().arg("--version"))?);
    Ok(())
}

#[test]
fn help_describes_pace() -> TestResult {
    insta::assert_snapshot!(run(pace().arg("--help"))?);
    Ok(())
}

#[test]
fn pace_without_arguments_shows_the_help_as_a_usage_error() -> TestResult {
    insta::assert_snapshot!(run(&mut pace())?);
    Ok(())
}
