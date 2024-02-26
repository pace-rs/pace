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

use pace_core::{get_activity_log_paths, get_config_paths, toml, ActivityLog, PaceConfig};

use crate::prompt::{prompt_activity_log_path, prompt_config_file_path};

/// Final paths for the configuration and activity log files
///
/// This struct is used to store the final paths for the configuration and activity log files
#[derive(Debug, TypedBuilder, Getters, MutGetters)]
pub(crate) struct FinalSetupPaths {
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
pub(crate) fn env_knowledge_loop(term: &Term, config_root: &Path) -> Result<()> {
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
pub(crate) fn write_config(
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
pub(crate) fn write_activity_log(final_paths: &FinalSetupPaths) -> Result<()> {
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
pub(crate) fn print_intro(term: &Term) -> Result<()> {
    // Font name: Font Name: Georgia11
    // Source: https://patorjk.com/software/taag/#p=display&f=Georgia11&t=PACE
    let logo = style(
        r#"
`7MM"""Mq.   db       .g8"""bgd `7MM"""YMM  
  MM   `MM. ;MM:    .dP'     `M   MM    `7  
  MM   ,M9 ,V^MM.   dM'       `   MM   d    
  MMmmdM9 ,M  `MM   MM            MMmmMM    
  MM      AbmmmqMA  MM.           MM   Y  , 
  MM     A'     VML `Mb.     ,'   MM     ,M 
.JMML. .AMA.   .AMMA. `"bmmmd'  .JMMmmmmMMM 
    "#,
    )
    .italic()
    .green()
    .bold();

    let assistant_headline = style("Setup Assistant")
        .white()
        .on_black()
        .bold()
        .underlined();

    term.clear_screen()?;
    println!("{logo}");

    println!("{assistant_headline}");

    let intro_text = r"
Welcome to pace, your time tracking tool!

Whether you're diving in for the first time or keen on refining your setup,
this assistant is here to seamlessly tailor your environment to your preferences.

Ready to shape your experience? Here’s how:

- Glide through options with UP and DOWN arrows.
- Hit ENTER to use the default choice (or use y/n) and stride to the next prompt.

Worried about commitment? Don’t be. We’re only locking in your preferences at the
journey’s end, giving you the freedom to experiment. And if you decide to bow out early,
no sweat — Q, ESC, or Ctrl-C will let you exit gracefully without a trace of change.

Let’s embark on this customization adventure together—press ENTER when you’re ready
to elevate your productivity with pace.";

    println!("{intro_text}\n");

    let confirmation = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to continue?")
        .default(true)
        .interact()?;

    if !confirmation {
        eyre::bail!("Exiting setup assistant.");
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
pub(crate) fn confirmation_or_break(prompt: &str) -> Result<()> {
    let confirmation = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(true)
        .interact()?;

    if !confirmation {
        eyre::bail!("Exiting setup assistant. No changes were made.");
    }

    Ok(())
}

/// The `craft setup` commands interior for the pace application
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
pub fn craft_setup(term: &Term) -> Result<()> {
    let mut config = PaceConfig::default();

    let config_paths = get_config_paths("pace.toml")
        .into_iter()
        .map(|f| f.to_string_lossy().to_string())
        .collect::<Vec<String>>();

    let current_month_year = chrono::Local::now().format("%Y-%m").to_string();

    let activity_log_filename = format!("activity_{current_month_year}.pace.toml");

    let activity_log_paths = get_activity_log_paths(&activity_log_filename)
        .into_iter()
        .map(|f| f.to_string_lossy().to_string())
        .collect::<Vec<String>>();

    print_intro(term)?;

    term.clear_screen()?;

    let final_paths = prompt_activity_log_path(&activity_log_paths)?;

    config.add_activity_log_path(final_paths.activity_log_path());

    let final_paths = prompt_config_file_path(final_paths, config_paths.as_slice())?;

    let prompt = "Do you want the files to be written?";

    confirmation_or_break(prompt)?;

    term.clear_screen()?;

    write_activity_log(&final_paths)?;

    let Some(config_root) = final_paths.config_root() else {
        eyre::bail!("No config root. Exiting setup assistant.");
    };

    let Some(config_path) = final_paths.config_path() else {
        eyre::bail!("No config path. Exiting setup assistant.");
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

    Ok(())
}
