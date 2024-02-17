use std::path::PathBuf;

use dialoguer::Select;
use eyre::Result;
use tracing::debug;

use crate::setup::FinalSetupPaths;

/// Get the activity log path from the user
///
/// # Arguments
///
/// * `activity_log_paths` - The list of activity log paths
///
/// # Errors
///
/// Returns an error if the user exits the setup assistant
///
/// # Returns
///
/// Returns the selected activity log path
pub(crate) fn prompt_activity_log_path(activity_log_paths: &[String]) -> Result<FinalSetupPaths> {
    let activity_log_path_select_text = r"Please select the location for your activity log";

    let selection = Select::new()
        .with_prompt(activity_log_path_select_text)
        .clear(true)
        .items(activity_log_paths)
        .interact_opt()?;

    let selected_activity_log_path = match selection {
        Some(index) => PathBuf::from(activity_log_paths[index].clone()),
        None => {
            eyre::bail!("Exiting setup assistant.");
        }
    };

    let parent = if let Some(p) = selected_activity_log_path.parent() {
        debug!("Parent directory created.");
        p
    } else {
        debug!("No parent directory for activity log file.");
        eyre::bail!("Exiting setup assistant. No changes were made.");
    };

    Ok(FinalSetupPaths::builder()
        .activity_log_path(selected_activity_log_path.clone())
        .activity_log_root(parent.to_path_buf())
        .build())
}

/// Get the configuration file path from the user
///
/// # Arguments
///
/// * `final_paths` - The current state of the setup paths
/// * `config_paths` - The list of configuration file paths
///
/// # Errors
///
/// Returns an error if the user exits the setup assistant
///
/// # Returns
///
/// Returns the updated setup paths
pub(crate) fn prompt_config_file_path(
    mut final_paths: FinalSetupPaths,
    config_paths: &[String],
) -> Result<FinalSetupPaths> {
    let config_path_select_text = r"Please select the location for your configuration";

    let selection = Select::new()
        .with_prompt(config_path_select_text)
        .clear(true)
        .items(config_paths)
        .interact_opt()?;

    let selected_config_path = match selection {
        Some(index) => PathBuf::from(config_paths[index].clone()),
        None => {
            eyre::bail!("Exiting setup assistant.");
        }
    };

    let parent = if let Some(p) = selected_config_path.parent() {
        debug!("Parent directory created.");
        p
    } else {
        debug!("No parent directory for config file.");
        eyre::bail!("Exiting setup assistant. No changes were made.");
    };

    _ = final_paths
        .config_path_mut()
        .replace(selected_config_path.clone());
    _ = final_paths.config_root_mut().replace(parent.to_path_buf());

    Ok(final_paths)
}
