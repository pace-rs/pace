use pace_core::config::find_config_file_path_from_current_dir;

use std::io::Write;
use std::{env, fs::File, path::Path};

use rstest::rstest;

#[rstest]
fn test_find_root_projects_file() -> Result<(), Box<dyn std::error::Error>> {
    let projects_config_name = "projects.pace.toml";

    // navigate to the test directory for the fixtures
    let root = Path::new(".\\tests\\fixtures\\project1\\subproject-a\\").canonicalize()?;
    assert!(env::set_current_dir(root).is_ok());

    // get the path to the projects config file
    let projects_config_path = find_config_file_path_from_current_dir(projects_config_name)?;

    assert_eq!(
        projects_config_path,
        Path::new(".\\tests\\fixtures\\project1\\projects.pace.toml").canonicalize()?
    );

    Ok(())
}
