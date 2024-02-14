//! `config` subcommand

use std::path::PathBuf;

use abscissa_core::{status_warn, tracing::debug, Application, Command, Runnable, Shutdown};
use clap::Parser;
use dialoguer::{
    console::{style, Term},
    Select,
};
use eyre::Result;
use pace_cli::setup::{
    confirmation_or_break, env_knowledge_loop, print_intro, write_activity_log, write_config,
    FinalSetupPaths,
};
use pace_core::config::{get_activity_log_paths, get_config_paths, PaceConfig};

use crate::prelude::PACE_APP;

/// `config` subcommand
#[derive(Command, Debug, Parser)]
pub struct SetupSubCmd {
    /// Path to the configuration file
    config_path: Option<PathBuf>,

    /// Path to the activity log file
    activity_log: Option<PathBuf>,
}

impl Runnable for SetupSubCmd {
    /// Start the application.
    fn run(&self) {
        let term = Term::stdout();
        if let Err(err) = self.inner_run(&term) {
            // Do nothing, and let the error be, we are already panicking anyway
            _ = term.clear_screen();

            status_warn!("{}", err);
            PACE_APP.shutdown(Shutdown::Graceful);
        };
    }
}

impl SetupSubCmd {
    pub fn inner_run(&self, term: &Term) -> Result<()> {
        let default_config_content = PaceConfig::default();

        let config_paths = get_config_paths("pace.toml")
            .into_iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect::<Vec<String>>();

        let current_month_year = chrono::Local::now().format("%Y-%m").to_string();

        let activity_log_filename = format!("pace_{}.toml", current_month_year);

        let activity_log_paths = get_activity_log_paths(&activity_log_filename)
            .into_iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect::<Vec<String>>();

        print_intro(term)?;

        term.clear_screen()?;

        let final_paths = self.get_activity_log_path(&activity_log_paths)?;

        let config = default_config_content.with_activity_log(final_paths.activity_log_path());

        let final_paths = self.get_config_file_path(final_paths, config_paths.as_slice())?;

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

        write_config(config, config_root, config_path)?;

        println!(
            "To prioritize this configuration, set the `{}` environment variable to '{}'.",
            style("PACE_HOME").bold().red(),
            style(config_root.display()).bold().green()
        );

        env_knowledge_loop(term, config_root)?;

        term.clear_screen()?;

        println!(
            "{}",
            style("Configuration assistant completed successfully, here are the final paths:")
                .green()
        );

        println!();

        println!(
            "Environment variable: PACE_HOME=\"{}\".",
            style(config_root.display()).cyan()
        );

        println!("{final_paths}");

        Ok(())
    }

    fn get_activity_log_path(&self, activity_log_paths: &[String]) -> Result<FinalSetupPaths> {
        let activity_log_path_select_text = r#"Please select the location for your activity log"#;

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

        let parent = match selected_activity_log_path.parent() {
            Some(p) => {
                debug!("Parent directory created.");
                p
            }
            None => {
                debug!("No parent directory for activity log file.");
                eyre::bail!("Exiting setup assistant. No changes were made.");
            }
        };

        Ok(FinalSetupPaths::builder()
            .activity_log_path(selected_activity_log_path.to_path_buf())
            .activity_log_root(parent.to_path_buf())
            .build())
    }

    fn get_config_file_path(
        &self,
        mut final_paths: FinalSetupPaths,
        config_paths: &[String],
    ) -> Result<FinalSetupPaths> {
        let config_path_select_text = r#"Please select the location for your configuration"#;

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

        let parent = match selected_config_path.parent() {
            Some(p) => {
                debug!("Parent directory created.");
                p
            }
            None => {
                debug!("No parent directory for config file.");
                eyre::bail!("Exiting setup assistant. No changes were made.");
            }
        };

        final_paths
            .config_path_mut()
            .replace(selected_config_path.to_path_buf());
        final_paths.config_root_mut().replace(parent.to_path_buf());

        Ok(final_paths)
    }
}
