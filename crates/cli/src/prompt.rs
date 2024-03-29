use std::path::PathBuf;

use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Select};
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
pub fn prompt_activity_log_path(activity_log_paths: &[String]) -> Result<FinalSetupPaths> {
    let activity_log_path_select_text = r"Please select the location for your activity log";

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(activity_log_path_select_text)
        .clear(true)
        .items(activity_log_paths)
        .interact_opt()?;

    let selected_activity_log_path = match selection {
        Some(index) => PathBuf::from(activity_log_paths[index].clone()),
        None => {
            eyre::bail!("Setup exited without changes.");
        }
    };

    let parent = if let Some(p) = selected_activity_log_path.parent() {
        debug!("Parent directory created.");
        p
    } else {
        debug!("No parent directory for activity log file.");
        eyre::bail!("Setup exited without changes. No changes were made.");
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
pub fn prompt_config_file_path(
    mut final_paths: FinalSetupPaths,
    config_paths: &[String],
) -> Result<FinalSetupPaths> {
    let config_path_select_text = r"Please select the location for your configuration";

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(config_path_select_text)
        .clear(true)
        .items(config_paths)
        .interact_opt()?;

    let selected_config_path = match selection {
        Some(index) => PathBuf::from(config_paths[index].clone()),
        None => {
            eyre::bail!("Setup exited without changes.");
        }
    };

    let parent = if let Some(p) = selected_config_path.parent() {
        debug!("Parent directory created.");
        p
    } else {
        debug!("No parent directory for config file.");
        eyre::bail!("Setup exited without changes. No changes were made.");
    };

    _ = final_paths
        .config_path_mut()
        .replace(selected_config_path.clone());
    _ = final_paths.config_root_mut().replace(parent.to_path_buf());

    Ok(final_paths)
}

/// Prompts the user to confirm their choices or break
///
/// # Arguments
///
/// * `prompt` - The prompt to display to the user
///
/// # Errors
///
/// Returns an error if the wants to break or if the prompt fails
///
/// # Returns
///
/// Returns `Ok(())` if the user confirms their choices
pub fn confirmation_or_break_default_false(prompt: &str) -> Result<()> {
    let confirmation = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(false)
        .interact()?;

    if !confirmation {
        eyre::bail!("Exiting. No changes were made.");
    }

    Ok(())
}

/// Prompts the user to confirm their choices or break the setup assistant
///
/// # Arguments
///
/// * `prompt` - The prompt to display to the user
///
/// # Errors
///
/// Returns an error if the wants to break the setup assistant or
/// if the prompt fails
///
/// # Returns
///
/// Returns `Ok(())` if the user confirms their choices
pub fn confirmation_or_break_default_true(prompt: &str) -> Result<()> {
    let confirmation = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(true)
        .interact()?;

    if !confirmation {
        eyre::bail!("Setup exited without changes. No changes were made.");
    }

    Ok(())
}

/// Prompts the user to select an activity to resume
///
/// # Arguments
///
/// * `string_repr` - The list of activities represented as a String to resume
///
/// # Errors
///
/// Returns an error if the prompt fails
///
/// # Returns
///
/// Returns the index of the selected activity
pub fn prompt_resume_activity(string_repr: &[String]) -> Result<usize, dialoguer::Error> {
    FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Which activity do you want to continue?")
        .items(string_repr)
        .interact()
}

/// Prompts the user to select a time zone
///
/// # Errors
///
/// Returns an error if the prompt fails
///
/// # Returns
///
/// Returns the selected time zone
pub fn prompt_time_zone() -> Result<chrono_tz::Tz> {
    let time_zones_iter = chrono_tz::TZ_VARIANTS.into_iter();

    let time_zones_lookup = time_zones_iter.clone().collect::<Vec<chrono_tz::Tz>>();

    let time_zones = time_zones_iter
        .map(|tz| format!("{tz}"))
        .collect::<Vec<String>>();

    // TODO: Search through timezones and mark the one we determined to be the first in the current range as the default

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Please select your time zone:")
        .clear(true)
        .items(&time_zones)
        .interact()?;

    Ok(time_zones_lookup[selection])
}
