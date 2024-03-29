use std::{
    fmt::Display,
    fs::create_dir_all,
    path::{Path, PathBuf},
};

use dialoguer::{
    console::{style, Term},
    theme::ColorfulTheme,
    Confirm,
};
use eyre::Result;
use getset::{Getters, MutGetters};
use tracing::{debug, info};
use typed_builder::TypedBuilder;

use pace_core::{
    constants::PACE_ACTIVITY_LOG_FILENAME,
    constants::PACE_CONFIG_FILENAME,
    prelude::{get_activity_log_paths, get_config_paths, ActivityLog, PaceConfig},
    toml,
};

use crate::{
    prompt::{prompt_activity_log_path, prompt_config_file_path},
    prompt_time_zone, PACE_ART,
};

#[derive(Debug, TypedBuilder, Getters)]
pub struct PathOptions {
    /// Path to the activity log file
    #[getset(get = "pub")]
    activity_log: Option<PathBuf>,
}

/// Final paths for the configuration and activity log files
///
/// This struct is used to store the final paths for the configuration and activity log files
#[derive(Debug, TypedBuilder, Getters, MutGetters)]
pub struct FinalSetupPaths {
    /// The path to the activity log file
    #[builder(default)]
    #[getset(get = "pub")]
    activity_log_path: PathBuf,

    /// The root directory for the activity log file
    #[builder(default)]
    #[getset(get = "pub")]
    activity_log_root: PathBuf,

    /// The path to the configuration file
    #[builder(default, setter(strip_option))]
    #[getset(get = "pub", get_mut = "pub")]
    config_path: Option<PathBuf>,

    /// The root directory for the configuration file
    #[builder(default, setter(strip_option))]
    #[getset(get = "pub", get_mut = "pub")]
    config_root: Option<PathBuf>,
}

impl Display for FinalSetupPaths {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let config_root = self
            .config_root
            .clone()
            .map_or_else(PathBuf::new, |config_root| config_root);

        let config_path = self
            .config_path
            .clone()
            .map_or_else(PathBuf::new, |config_path| config_path);

        writeln!(f, "Configuration root: {:?}", style(config_root).cyan())?;
        writeln!(f, "Configuration: {:?}", style(config_path).cyan())?;
        writeln!(
            f,
            "Activity log root: {:?}",
            style(&self.activity_log_root).cyan()
        )?;
        writeln!(
            f,
            "Activity log: {:?}",
            style(&self.activity_log_path).cyan()
        )
    }
}

/// Asks the user if they know how to set environment variables
/// and provides a guide if they don't
///
/// # Arguments
///
/// * `term` - The terminal to use for the prompt
/// * `config_root` - The root directory for the configuration file
///
/// # Errors
///
/// Returns an error if the prompt fails
///
/// # Returns
///
/// Returns `Ok(())` if the prompt succeeds
pub fn env_knowledge_loop(term: &Term, config_root: &Path) -> Result<()> {
    let env_var_knowledge = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you know how to set environment variables?")
        .default(true)
        .interact()?;

    println!();

    'env: loop {
        term.clear_screen()?;

        if env_var_knowledge {
            break 'env;
        }

        println!(
            "To prioritize this configuration, set the `{}` environment variable to '{}'.",
            style("PACE_HOME").bold().red(),
            style(config_root.display()).bold().green()
        );
        println!(
                "You can check out this guide: {}",
                style("https://web.archive.org/web/20240110123209/https://www3.ntu.edu.sg/home/ehchua/programming/howto/Environment_Variables.html")
                    .bold()
                    .blue()
            );

        let ready_to_continue = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Are you ready to continue?")
            .default(false)
            .interact()?;

        if !ready_to_continue {
            break 'env;
        }
    }

    Ok(())
}

/// Writes the configuration to the file system
///
/// # Arguments
///
/// * `config` - The configuration to write
/// * `config_root` - The root directory for the configuration file
/// * `config_path` - The path to the configuration file
///
/// # Errors
///
/// Returns an error if the configuration cannot be written to the file system
///
/// # Returns
///
/// Returns `Ok(())` if the configuration is written successfully
pub fn write_config(
    config: &PaceConfig,
    config_root: &PathBuf,
    config_path: &PathBuf,
) -> Result<()> {
    let config_content = toml::to_string_pretty(&config)?;

    create_dir_all(config_root)?;

    if config_root.exists() {
        // Create a backup before writing the new configuration
        if config_path.exists() {
            info!("A configuration already exists, creating a backup next to the existing one.");
            let backup_path = config_path.with_extension("toml.bak");

            _ = std::fs::copy(config_path, backup_path)?;
        }

        // Write the pace.toml file
        std::fs::write(config_path, config_content.as_bytes())?;

        debug!("Configuration written successfully.");
    }

    Ok(())
}

/// Writes the activity log to the file system
///
/// # Arguments
///
/// * `final_paths` - The final paths for the activity log file
///
/// # Errors
///
/// Returns an error if the activity log cannot be written to the file system
///
/// # Returns
///
/// Returns `Ok(())` if the activity log is written successfully
pub fn write_activity_log(final_paths: &FinalSetupPaths) -> Result<()> {
    let activity_log = ActivityLog::default();

    let activity_log_content = toml::to_string_pretty(&activity_log)?;

    create_dir_all(&final_paths.activity_log_root)?;

    if final_paths.activity_log_root.exists() {
        // Create a backup before writing the new activity log
        if final_paths.activity_log_path.exists() {
            info!("An activity log already exists, creating a backup next to the existing one.");
            let backup_path = &final_paths.activity_log_path.with_extension("toml.bak");

            _ = std::fs::copy(&final_paths.activity_log_path, backup_path)?;
        }

        // Write the activity log file
        std::fs::write(
            &final_paths.activity_log_path,
            activity_log_content.as_bytes(),
        )?;
        debug!("Activity log written successfully.");
    }

    Ok(())
}

/// Prints the introduction to the setup assistant
///
/// # Arguments
///
/// * `term` - The terminal to use for the prompt
///
/// # Errors
///
/// Returns an error if the prompt fails
///
/// # Returns
///
/// Returns `Ok(())` if the prompt succeeds
pub fn print_intro(term: &Term) -> Result<()> {
    // Font name: Font Name: Georgia11
    // Source: https://patorjk.com/software/taag/#p=display&f=Georgia11&t=PACE

    let logo = style(PACE_ART.to_string()).italic().green().bold();

    let assistant_headline = style("Setup Assistant")
        .white()
        .on_black()
        .bold()
        .underlined();

    term.clear_screen()?;

    println!("{logo}");

    println!("{assistant_headline}");

    let intro_text = r"
Keep the pace on your command line. Time tracking and management.

Use this assistant to setup your pace environment and preferences.

- Use UP / Down arrows to choose options
- or Enter for default choice when applicable

Preferences will only be saved if you complete the setup.
Use Q, ESC, or Ctrl-C to exit gracefully at any time.";

    println!("{intro_text}\n");

    let confirmation = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Ready to start?")
        .default(true)
        .interact()?;

    if !confirmation {
        eyre::bail!("Setup exited without changes.");
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
pub fn confirmation_or_break(prompt: &str) -> Result<()> {
    let confirmation = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(true)
        .interact()?;

    if !confirmation {
        eyre::bail!("Setup exited without changes. No changes were made.");
    }

    Ok(())
}

/// The `setup` commands interior for the pace application
///
/// # Arguments
///
/// * `term` - The terminal to use for the prompt
///
/// # Errors
///
/// Returns an error if the setup assistant fails
///
/// # Returns
///
/// Returns `Ok(())` if the setup assistant succeeds
pub fn setup_config(term: &Term, path_opts: &PathOptions) -> Result<()> {
    let mut config = PaceConfig::default();

    let config_paths = get_config_paths(PACE_CONFIG_FILENAME)
        .into_iter()
        .map(|f| f.to_string_lossy().to_string())
        .collect::<Vec<String>>();

    let mut activity_log_paths = get_activity_log_paths(PACE_ACTIVITY_LOG_FILENAME)
        .into_iter()
        .map(|f| f.to_string_lossy().to_string())
        .collect::<Vec<String>>();

    // Add the custom path from the cli input to the activity log paths
    if let Some(mut custom_path) = path_opts.activity_log().clone() {
        custom_path.push(PACE_ACTIVITY_LOG_FILENAME);
        activity_log_paths.push(custom_path.to_string_lossy().to_string());
    }

    print_intro(term)?;

    term.clear_screen()?;

    let time_zone = prompt_time_zone()?;

    term.clear_screen()?;

    let final_paths = prompt_activity_log_path(&activity_log_paths)?;

    config.set_activity_log_path(final_paths.activity_log_path());
    config.set_time_zone(time_zone);

    let final_paths = prompt_config_file_path(final_paths, config_paths.as_slice())?;

    let prompt = "Do you want the files to be written?";

    confirmation_or_break(prompt)?;

    term.clear_screen()?;

    write_activity_log(&final_paths)?;

    let Some(config_root) = final_paths.config_root() else {
        eyre::bail!("No config root. Setup exited without changes.");
    };

    let Some(config_path) = final_paths.config_path() else {
        eyre::bail!("No config path. Setup exited without changes.");
    };

    write_config(&config, config_root, config_path)?;

    println!(
        "To prioritize this configuration, set the `{}` environment variable to '{}'.",
        style("PACE_HOME").bold().red(),
        style(config_root.display()).bold().green()
    );

    env_knowledge_loop(term, config_root)?;

    term.clear_screen()?;

    println!(
        "{}",
        style("Configuration assistant completed successfully, here are the final paths:").green()
    );

    println!();

    println!(
        "Environment variable: PACE_HOME=\"{}\".",
        style(config_root.display()).cyan()
    );

    println!("{final_paths}");

    println!(
        "For optimal user experience, it's essential to read our Getting Started guide here: {}",
        style("https://pace.cli.rs/docs/getting_started.html")
            .bold()
            .red()
    );

    Ok(())
}
