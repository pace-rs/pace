//! Pace Config

use std::fs;
use std::path::{Path, PathBuf};

use getset::{Getters, MutGetters};
use serde_derive::{Deserialize, Serialize};

use directories::ProjectDirs;

use crate::error::{PaceErrorKind, PaceResult};

/// The pace configuration file
///
/// The pace configuration file is a TOML file that contains the configuration for the pace application.
#[derive(Debug, Deserialize, Default, Serialize, Getters, Clone, MutGetters)]
#[serde(deny_unknown_fields)]
#[getset(get = "pub")]
pub struct PaceConfig {
    /// General configuration for the pace application
    #[getset(get = "pub", get_mut = "pub")]
    general: GeneralConfig,

    /// Review configuration for the pace application
    reviews: ReviewConfig,

    /// Export configuration for the pace application
    export: ExportConfig,

    /// Database configuration for the pace application
    database: Option<DatabaseConfig>, // Optional because it's only needed if log_storage is "database"

    /// Pomodoro configuration for the pace application
    pomodoro: PomodoroConfig,

    /// Inbox configuration for the pace application
    inbox: InboxConfig,

    /// Auto-archival configuration for the pace application
    auto_archival: AutoArchivalConfig,
}
impl PaceConfig {
    /// Create a new [`PaceConfig`] with the given path to an activity log file
    ///
    /// # Arguments
    ///
    /// `activity_log` - The path to the activity log file
    #[must_use]
    pub fn with_activity_log(self, activity_log: impl AsRef<Path>) -> Self {
        let mut new_config = self;
        new_config.general.activity_log_file_path =
            activity_log.as_ref().to_string_lossy().to_string();
        new_config
    }
}

/// The general configuration for the pace application
#[derive(Debug, Deserialize, Default, Serialize, Getters, MutGetters, Clone)]
#[getset(get = "pub")]
pub struct GeneralConfig {
    /// The storage type for the activity log
    log_storage: String,

    /// The path to the activity log file
    #[getset(get = "pub", get_mut = "pub")]
    activity_log_file_path: String,

    /// The format for the activity log
    log_format: String,

    /// If IDs should be autogenerated for activities
    autogenerate_ids: bool,

    /// The default category separator
    category_separator: String,

    /// The default category
    default_priority: String,
}

/// The review configuration for the pace application
#[derive(Debug, Deserialize, Default, Serialize, Getters, Clone)]
#[getset(get = "pub")]
pub struct ReviewConfig {
    /// The format for the review
    review_format: String,

    /// The directory to store the review files
    review_directory: String,
}

/// The export configuration for the pace application
#[derive(Debug, Deserialize, Default, Serialize, Getters, Clone)]
#[getset(get = "pub")]
pub struct ExportConfig {
    /// If the export should include tags
    export_include_tags: bool,

    /// If the export should include descriptions
    export_include_descriptions: bool,

    /// The time format within the export
    export_time_format: String,
}

/// The database configuration for the pace application
#[derive(Debug, Deserialize, Default, Serialize, Getters, Clone)]
#[getset(get = "pub")]
pub struct DatabaseConfig {
    /// The type of database
    #[serde(rename = "type")]
    db_type: String, // `type` is a reserved keyword in Rust

    /// The connection string for the database
    connection_string: String,
}

/// The pomodoro configuration for the pace application
#[derive(Debug, Deserialize, Default, Serialize, Getters, Clone, Copy)]
#[getset(get = "pub")]
pub struct PomodoroConfig {
    /// The duration of a work session in minutes
    work_duration_minutes: u32,

    /// The duration of a short break in minutes
    break_duration_minutes: u32,

    /// The duration of a long break in minutes
    long_break_duration_minutes: u32,

    /// The number of work sessions before a long break
    sessions_before_long_break: u32,
}

/// The inbox configuration for the pace application
#[derive(Debug, Deserialize, Default, Serialize, Getters, Clone)]
#[getset(get = "pub")]
pub struct InboxConfig {
    /// The maximum items the inbox should hold
    max_size: u32,

    /// The default category for items in the inbox
    default_priority: String,

    /// The default time to auto-archive items in the inbox (in days)
    auto_archive_after_days: u32,
}

/// The auto-archival configuration for the pace application
#[derive(Debug, Deserialize, Default, Serialize, Getters, Clone)]
#[getset(get = "pub")]
pub struct AutoArchivalConfig {
    /// If auto-archival is enabled
    enabled: bool,

    /// The default auto-archival time after which items should be archived (in days)
    archive_after_days: u32,

    /// The path to the archive file
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
pub fn find_root_project_file(
    starting_directory: impl AsRef<Path>,
    file_name: &str,
) -> Option<PathBuf> {
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
pub fn find_root_config_file_path(
    current_dir: impl AsRef<Path>,
    file_name: &str,
) -> PaceResult<PathBuf> {
    find_root_project_file(&current_dir, file_name).ok_or_else(|| {
        PaceErrorKind::ConfigFileNotFound {
            current_dir: current_dir.as_ref().to_string_lossy().to_string(),
            file_name: file_name.to_string(),
        }
        .into()
    })
}

/// Get the paths to the activity log file
///
/// # Arguments
///
/// * `filename` - name of the config file
///
/// # Returns
///
/// A vector of [`PathBuf`]s to the activity log files
#[must_use]
pub fn get_activity_log_paths(filename: &str) -> Vec<PathBuf> {
    vec![
        ProjectDirs::from("org", "pace-rs", "pace").map(|project_dirs| {
            project_dirs
                .data_local_dir()
                .to_path_buf()
                .join("activities")
        }),
        // Fallback to the current directory
        Some(PathBuf::from(".")),
    ]
    .into_iter()
    .filter_map(|path| path.map(|p| p.join(filename)))
    .collect::<Vec<_>>()
}

/// Get the paths to the config file
///
/// # Arguments
///
/// * `filename` - name of the config file
///
/// # Returns
///
/// A vector of [`PathBuf`]s to the config files
#[must_use]
pub fn get_config_paths(filename: &str) -> Vec<PathBuf> {
    #[allow(unused_mut)]
    let mut paths = vec![
        get_home_config_path(),
        ProjectDirs::from("org", "pace-rs", "pace")
            .map(|project_dirs| project_dirs.config_dir().to_path_buf()),
        get_global_config_path(),
        // Fallback to the current directory
        Some(PathBuf::from(".")),
    ];

    #[cfg(target_os = "windows")]
    {
        if let Some(win_compatibility_paths) = get_windows_portability_config_directories() {
            paths.extend(win_compatibility_paths);
        };
    }

    paths
        .into_iter()
        .filter_map(|path| path.map(|p| p.join(filename)))
        .collect::<Vec<_>>()
}

/// Get the path to the home activity log directory.
///
/// # Returns
///
/// The path to the home activity log directory.
/// If the environment variable `PACE_HOME` is not set, `None` is returned.
pub fn get_home_activity_log_path() -> Option<PathBuf> {
    std::env::var_os("PACE_HOME").map(|home_dir| PathBuf::from(home_dir).join("activities"))
}

/// Get the path to the home config directory.
///
/// # Returns
///
/// The path to the home config directory.
/// If the environment variable `PACE_HOME` is not set, `None` is returned.
pub fn get_home_config_path() -> Option<PathBuf> {
    std::env::var_os("PACE_HOME").map(|home_dir| PathBuf::from(home_dir).join("config"))
}

/// Get the paths to the user profile config directories on Windows.
///
/// # Returns
///
/// A collection of possible paths to the user profile config directory on Windows.
///
/// # Note
///
/// If the environment variable `USERPROFILE` is not set, `None` is returned.
#[cfg(target_os = "windows")]
fn get_windows_portability_config_directories() -> Option<Vec<Option<PathBuf>>> {
    std::env::var_os("USERPROFILE").map(|path| {
        vec![
            Some(PathBuf::from(path.clone()).join(r".config\pace")),
            Some(PathBuf::from(path).join(".pace")),
        ]
    })
}

/// Get the path to the global config directory on Windows.
///
/// # Returns
///
/// The path to the global config directory on Windows.
/// If the environment variable `PROGRAMDATA` is not set, `None` is returned.
#[cfg(target_os = "windows")]
fn get_global_config_path() -> Option<PathBuf> {
    std::env::var_os("PROGRAMDATA")
        .map(|program_data| PathBuf::from(program_data).join(r"pace\config"))
}

/// Get the path to the global config directory on ios and wasm targets.
///
/// # Returns
///
/// `None` is returned.
#[cfg(any(target_os = "ios", target_arch = "wasm32"))]
fn get_global_config_path() -> Option<PathBuf> {
    None
}

/// Get the path to the global config directory on non-Windows,
/// non-iOS, non-wasm targets.
///
/// # Returns
///
/// "/etc/pace" is returned.
#[cfg(not(any(target_os = "windows", target_os = "ios", target_arch = "wasm32")))]
fn get_global_config_path() -> Option<PathBuf> {
    Some(PathBuf::from("/etc/pace"))
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
        #[files("../../config/projects.pace.toml")] config_path: PathBuf,
    ) -> TestResult<()> {
        let toml_string = fs::read_to_string(config_path)?;
        let _ = toml::from_str::<ProjectConfig>(&toml_string)?;

        Ok(())
    }

    #[rstest]
    fn test_parse_tasks_file_passes(
        #[files("../../config/tasks.pace.toml")] config_path: PathBuf,
    ) -> TestResult<()> {
        let toml_string = fs::read_to_string(config_path)?;
        let _ = toml::from_str::<TaskList>(&toml_string)?;

        Ok(())
    }
}
