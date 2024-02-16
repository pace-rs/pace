use std::{
    fmt::Display,
    fs::create_dir_all,
    path::{Path, PathBuf},
};

use dialoguer::{
    console::{style, Term},
    Confirm,
};
use eyre::Result;
use getset::{Getters, MutGetters};
use tracing::debug;
use typed_builder::TypedBuilder;

use pace_core::{config::PaceConfig, domain::activity_log::ActivityLog, toml};

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
        let config_root = if let Some(config_root) = self.config_root.clone() {
            config_root
        } else {
            PathBuf::new()
        };

        let config_path = if let Some(config_path) = self.config_path.clone() {
            config_path
        } else {
            PathBuf::new()
        };

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

pub fn env_knowledge_loop(term: &Term, config_root: &Path) -> Result<()> {
    let env_var_knowledge = Confirm::new()
        .with_prompt("Do you know how to set environment variables?")
        .default(true)
        .interact_opt()?;

    println!();

    'env: loop {
        term.clear_screen()?;

        if let Some(true) = env_var_knowledge {
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

        let ready_to_continue = Confirm::new()
            .with_prompt("Are you ready to continue?")
            .default(false)
            .interact_opt()?;

        if let Some(true) = ready_to_continue {
            break 'env;
        }
    }

    Ok(())
}

pub fn write_config(
    config: PaceConfig,
    config_root: &PathBuf,
    config_path: &PathBuf,
) -> Result<()> {
    let config_content = toml::to_string_pretty(&config)?;

    create_dir_all(config_root)?;

    if config_root.exists() {
        // Write the pace.toml file
        std::fs::write(config_path, config_content.as_bytes())?;

        debug!("Configuration written successfully.");
    }

    Ok(())
}

pub fn write_activity_log(final_paths: &FinalSetupPaths) -> Result<()> {
    let activity_log = ActivityLog::default();

    let activity_log_content = toml::to_string_pretty(&activity_log)?;

    create_dir_all(&final_paths.activity_log_root)?;

    if final_paths.activity_log_root.exists() {
        // Write the activity log file
        std::fs::write(
            &final_paths.activity_log_path,
            activity_log_content.as_bytes(),
        )?;
        debug!("Activity log written successfully.");
    }

    Ok(())
}

pub fn print_intro(term: &Term) -> Result<()> {
    let assistant_headline = style("Pace Setup Assistant\n")
        .white()
        .on_black()
        .bold()
        .underlined();

    term.clear_screen()?;

    println!("{assistant_headline}");

    let intro_text = r#"Welcome to pace, your time tracking tool!

Whether you're diving in for the first time or keen on refining your setup,
this assistant is here to seamlessly tailor your environment to your preferences.

Ready to shape your experience? Here’s how:

- Glide through options with UP and DOWN arrows.
- Hit ENTER to use the default choice (or use y/n) and stride to the next prompt.

Worried about commitment? Don’t be. We’re only locking in your preferences at the
journey’s end, giving you the freedom to experiment. And if you decide to bow out early,
no sweat — Q, ESC, or Ctrl-C will let you exit gracefully without a trace of change.

Let’s embark on this customization adventure together—press ENTER when you’re ready
to elevate your productivity with pace."#;

    println!("{intro_text}\n");

    let confirmation = Confirm::new()
        .with_prompt("Do you want to continue?")
        .default(true)
        .interact_opt()?;

    if let Some(false) = confirmation {
        eyre::bail!("Exiting setup assistant.");
    }

    Ok(())
}

pub fn confirmation_or_break(prompt: &str) -> Result<bool> {
    let confirmation = Confirm::new()
        .with_prompt(prompt)
        .default(true)
        .interact_opt()?;

    if let Some(false) = confirmation {
        eyre::bail!("Exiting setup assistant. No changes were made.");
    } else {
        Ok(true)
    }
}
