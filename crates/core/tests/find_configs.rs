use pace_core::{find_root_config_file_path, TestResult};
use similar_asserts::assert_eq;
use std::env;

use rstest::rstest;

#[rstest]
fn test_find_root_projects_file() -> TestResult<()> {
    let current_dir = env::current_dir()?;
    let projects_config_name = "projects.pace.toml";

    // navigate to the test directory for the fixtures
    let root = current_dir
        .join("tests/fixtures/project1/subproject-a/")
        .canonicalize()?;

    // get the path to the projects config file
    let projects_config_path = find_root_config_file_path(root, projects_config_name)?;

    assert_eq!(
        projects_config_path,
        current_dir
            .join("tests/fixtures/project1/projects.pace.toml")
            .canonicalize()?
    );

    Ok(())
}
