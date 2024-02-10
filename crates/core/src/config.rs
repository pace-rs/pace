//! Pace Config

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDateTime;
use derive_getters::Getters;
use serde_derive::{Deserialize, Serialize};

use crate::{
    domain::category::Category,
    error::{PaceErrorKind, PaceResult},
};

#[derive(Debug, Deserialize, Default, Serialize, Getters)]
#[serde(deny_unknown_fields)]
pub struct PaceConfig {
    general: GeneralConfig,
    reporting: ReportingConfig,
    export: ExportConfig,
    database: Option<DatabaseConfig>, // Optional because it's only needed if log_storage is "database"
    pomodoro: PomodoroConfig,
    inbox: InboxConfig,
    auto_archival: AutoArchivalConfig,
}

#[derive(Debug, Deserialize, Default, Serialize, Getters)]
pub struct GeneralConfig {
    log_storage: String,
    activity_log_file_path: String,
    log_format: String,
    autogenerate_ids: bool,
    category_separator: String,
    default_priority: String,
}

#[derive(Debug, Deserialize, Default, Serialize, Getters)]
pub struct ReportingConfig {
    report_format: String,
    report_directory: String,
}

#[derive(Debug, Deserialize, Default, Serialize, Getters)]
pub struct ExportConfig {
    export_include_tags: bool,
    export_include_descriptions: bool,
    export_time_format: String,
}

#[derive(Debug, Deserialize, Default, Serialize, Getters)]
pub struct DatabaseConfig {
    #[serde(rename = "type")]
    db_type: String, // `type` is a reserved keyword in Rust
    connection_string: String,
}

#[derive(Debug, Deserialize, Default, Serialize, Getters)]
pub struct PomodoroConfig {
    work_duration_minutes: u32,
    break_duration_minutes: u32,
    long_break_duration_minutes: u32,
    sessions_before_long_break: u32,
}

#[derive(Debug, Deserialize, Default, Serialize, Getters)]
pub struct InboxConfig {
    max_size: u32,
    default_priority: String,
    auto_archive_after_days: u32,
}

#[derive(Debug, Deserialize, Default, Serialize, Getters)]
pub struct AutoArchivalConfig {
    enabled: bool,
    archive_after_days: u32,
    archive_path: String,
}

/// Get the current directory and then search upwards in the directory hierarchy for a file name
///
/// # Arguments
///
/// * `starting_directory` - The directory to start searching from
/// * `file_name` - The name of the file to search for
///
/// # Returns
///
/// The path to the file if found, otherwise None
pub fn find_config_file(starting_directory: impl AsRef<Path>, file_name: &str) -> Option<PathBuf> {
    let mut current_dir = starting_directory.as_ref();

    loop {
        let config_path = current_dir.join(file_name);

        // Check if the config file exists in the current directory
        if fs::metadata(&config_path).is_ok() {
            return Some(config_path);
        }

        // Attempt to move up to the parent directory
        match current_dir.parent() {
            Some(parent) => current_dir = parent,
            None => break, // No more parent directories, stop searching
        }
    }

    None // No config file found
}

/// Find a config file in the current directory and upwards in the directory hierarchy and return the path
///
/// # Arguments
///
/// * `file_name` - The name of the file to search for
///
/// # Errors
///
/// [`PaceErrorKind::ConfigFileNotFound`] - If the current directory value is invalid
/// [`std::io::Error`] - If there is an error accessing the current directory (e.g. insufficient permissions)
/// or the current directory does not exist
///
/// # Returns
///
/// The path to the file if found
pub fn find_config_file_path_from_current_dir(file_name: &str) -> PaceResult<PathBuf> {
    let current_dir = env::current_dir()?;
    find_config_file(&current_dir, file_name).ok_or(
        PaceErrorKind::ConfigFileNotFound {
            current_dir: current_dir.clone().to_string_lossy().to_string(),
            file_name: file_name.to_string(),
        }
        .into(),
    )
}

#[cfg(test)]
mod tests {

    use crate::{domain::project::ProjectConfig, domain::task::TaskList, error::TestResult};

    use super::*;
    use rstest::*;
    use std::{fs, path::PathBuf};

    #[rstest]
    fn test_parse_pace_config_passes(
        #[files("../../config/pace.toml")] config_path: PathBuf,
    ) -> TestResult<()> {
        let toml_string = fs::read_to_string(config_path)?;
        let _ = toml::from_str::<PaceConfig>(&toml_string)?;

        Ok(())
    }

    #[rstest]
    fn test_parse_project_file_passes(
        #[files("../../config/project.toml")] config_path: PathBuf,
    ) -> TestResult<()> {
        let toml_string = fs::read_to_string(config_path)?;
        let _ = toml::from_str::<ProjectConfig>(&toml_string)?;

        Ok(())
    }

    #[rstest]
    fn test_parse_tasks_file_passes(
        #[files("../../config/tasks.toml")] config_path: PathBuf,
    ) -> TestResult<()> {
        let toml_string = fs::read_to_string(config_path)?;
        let _ = toml::from_str::<TaskList>(&toml_string)?;

        Ok(())
    }
}
