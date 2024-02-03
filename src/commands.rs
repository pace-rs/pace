//! Pace Subcommands
//!
//! This is where you specify the subcommands of your application.
//!
//! The default application comes with two subcommands:
//!
//! - `start`: launches the application
//! - `--version`: print application version
//!
//! See the `impl Configurable` below for how to specify the path to the
//! application's configuration file.

mod begin;
mod end;
mod export;
// TODO: mod import;
mod hold;
mod now;
mod pomo;
mod report;
mod resume;
mod set;
mod tasks;

use crate::config::PaceConfig;
use abscissa_core::{config::Override, Command, Configurable, FrameworkError, Runnable};
use std::path::PathBuf;

/// Pace Configuration Filename
pub const CONFIG_FILE: &str = "pace.toml";

/// Pace Subcommands
/// Subcommands need to be listed in an enum.
#[derive(clap::Parser, Command, Debug, Runnable)]
pub enum PaceCmd {
    /// Starts tracking time for the specified task. You can
    /// optionally specify a category or project to help organize
    /// your tasks.
    Begin(begin::BeginCmd),

    /// Stops time tracking for the specified task, marking it as completed or finished for the day.
    End(end::EndCmd),

    /// Exports your tracked data and reports in JSON or CSV format, suitable for analysis or record-keeping.
    Export(export::ExportCmd),

    /// Pauses the time tracking for the specified task. This is
    /// useful for taking breaks without ending the task.
    Hold(hold::HoldCmd),

    /// Displays the currently running task, showing you at a glance what you're currently tracking.
    Now(now::NowCmd),

    /// Starts a Pomodoro session for the specified task, integrating the Pomodoro technique directly with your tasks.
    Pomo(pomo::PomoCmd),

    /// Generates a report for your tasks. You can specify the time frame for daily, weekly, or monthly reports.
    Report(report::ReportCmd),

    /// Resumes time tracking for a previously paused task, allowing you to continue where you left off.
    Resume(resume::ResumeCmd),

    /// Sets various application configurations, including Pomodoro lengths and preferred report formats.
    Set(set::SetCmd),

    /// Lists all tasks with optional filters. Use this to view active, completed, or today's tasks.
    Tasks(tasks::TasksCmd),
}

/// Entry point for the application. It needs to be a struct to allow using subcommands!
#[derive(clap::Parser, Command, Debug)]
#[command(author, about, version)]
pub struct EntryPoint {
    #[command(subcommand)]
    cmd: PaceCmd,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Use the specified config file
    #[arg(short, long)]
    pub config: Option<String>,
}

impl Runnable for EntryPoint {
    fn run(&self) {
        self.cmd.run()
    }
}

/// This trait allows you to define how application configuration is loaded.
impl Configurable<PaceConfig> for EntryPoint {
    /// Location of the configuration file
    fn config_path(&self) -> Option<PathBuf> {
        // Check if the config file exists, and if it does not, ignore it.
        // If you'd like for a missing configuration file to be a hard error
        // instead, always return `Some(CONFIG_FILE)` here.
        let filename = self
            .config
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| CONFIG_FILE.into());

        if filename.exists() {
            Some(filename)
        } else {
            None
        }
    }

    /// Apply changes to the config after it's been loaded, e.g. overriding
    /// values in a config file using command-line options.
    ///
    /// This can be safely deleted if you don't want to override config
    /// settings from command-line options.
    fn process_config(&self, config: PaceConfig) -> Result<PaceConfig, FrameworkError> {
        // match &self.cmd {
        // PaceCmd::Start(cmd) => cmd.override_config(config),
        //
        // If you don't need special overrides for some
        // subcommands, you can just use a catch all
        // _ => Ok(config),
        // }

        Ok(config)
    }
}
