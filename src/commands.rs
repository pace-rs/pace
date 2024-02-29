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

pub mod adjust;
pub mod begin;
pub mod docs;
pub mod end;
pub mod hold;
pub mod now;
pub mod resume;
pub mod review;
pub mod setup;

use abscissa_core::{config::Override, Command, Configurable, FrameworkError, Runnable};
use clap::builder::{styling::AnsiColor, Styles};
use human_panic::setup_panic;
use std::path::PathBuf;

use pace_core::{constants::PACE_CONFIG_FILENAME, get_config_paths, PaceConfig};

/// Pace Subcommands
/// Subcommands need to be listed in an enum.
#[derive(clap::Parser, Command, Debug, Runnable)]
pub enum PaceCmd {
    /// 📝 Adjust the details of an activity, such as its category, description, or tags.
    /// [alias: a]
    #[clap(alias = "a")]
    Adjust(adjust::AdjustCmd),

    /// ⌚ Starts tracking time for an activity.
    /// [alias: b]
    #[clap(alias = "b")]
    Begin(begin::BeginCmd),

    /// ⏹️  Stops time tracking for the most recent or all activities.
    /// [alias: e]
    #[clap(alias = "e")]
    End(end::EndCmd),

    /// ⏸️  Pauses the time tracking for the most recent active activity.
    /// [alias: h]
    #[clap(alias = "h")]
    Hold(hold::HoldCmd),

    /// ⏲️  Shows you at a glance what you're currently tracking.
    /// [alias: n]
    #[clap(alias = "n")]
    Now(now::NowCmd),

    /// ⏯️  Resumes a previously paused activity, allowing you to continue where you left off.
    /// [alias: r]
    #[clap(alias = "r")]
    Resume(resume::ResumeCmd),

    /// 📈 Get sophisticated insights on your activities.
    /// [alias: rev]
    #[clap(alias = "rev")]
    Review(review::ReviewCmd),

    /// 🛠️  Set up a pace configuration, a new project, or generate shell completions.
    /// [alias: s]
    #[clap(alias = "s")]
    Setup(setup::SetupCmd),

    /// 📚 Open the online documentation for pace.
    /// [alias: d]
    #[clap(alias = "d")]
    Docs(docs::DocsCmd),
    // /// Exports your tracked data and reviews in JSON or CSV format, suitable for analysis or record-keeping.
    // Export(export::ExportCmd),

    // /// Starts a Pomodoro session for the specified task, integrating the Pomodoro technique directly with your tasks.
    // Pomo(pomo::PomoCmd),
    // /// Sets various application configurations, including Pomodoro lengths and preferred review formats.
    // Set(set::SetCmd),

    // /// Lists all tasks with optional filters. Use this to view active, completed, or today's tasks.
    // Tasks(tasks::TasksCmd),
}

/// Define CLI colour styles for the application
const fn cli_colour_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::BrightBlue.on_default())
        .usage(AnsiColor::BrightYellow.on_default())
        .literal(AnsiColor::BrightGreen.on_default())
        .placeholder(AnsiColor::Magenta.on_default())
}

/// Entry point for the application. It needs to be a struct to allow using subcommands!
#[derive(clap::Parser, Command, Debug)]
#[command(name="pace", author, about, styles=cli_colour_styles(), version, arg_required_else_help = true)]
pub struct EntryPoint {
    #[command(subcommand)]
    cmd: PaceCmd,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Use the specified config file
    #[arg(long, env = "PACE_CONFIG_FILE", value_hint = clap::ValueHint::FilePath)]
    pub config: Option<PathBuf>,

    /// Use the specified activity log file
    #[arg(long, env = "PACE_ACTIVITY_LOG_FILE", value_hint = clap::ValueHint::FilePath)]
    pub activity_log_file: Option<PathBuf>,

    /// Pace Home Directory
    #[arg(long, env = "PACE_HOME", value_hint = clap::ValueHint::DirPath)]
    pub home: Option<PathBuf>,
}

impl Runnable for EntryPoint {
    fn run(&self) {
        setup_panic!();
        self.cmd.run();
    }
}

impl Override<PaceConfig> for EntryPoint {
    fn override_config(&self, mut config: PaceConfig) -> Result<PaceConfig, FrameworkError> {
        // Override the activity log file if it's set
        if let Some(activity_log_file) = &self.activity_log_file {
            if activity_log_file.exists() {
                *config
                    .general_mut()
                    .activity_log_options_mut()
                    .activity_log_path_mut() = activity_log_file.to_path_buf();
            }
        };

        Ok(config)
    }
}

/// This trait allows you to define how application configuration is loaded.
impl Configurable<PaceConfig> for EntryPoint {
    /// Location of the configuration file
    fn config_path(&self) -> Option<PathBuf> {
        let config_paths = get_config_paths(PACE_CONFIG_FILENAME)
            .into_iter()
            .filter(|f| f.exists())
            .collect::<Vec<_>>();

        // Get the first path that exists
        // FIXME: This feels hacky, is this sensible?
        let path = config_paths.first();

        let filename = self
            .config
            .as_ref()
            .and_then(|f| if f.exists() { Some(f) } else { None });

        // If the user has specified a config file, use that
        // otherwise, use the first config file found in specified
        // standard locations
        match (filename, path) {
            (Some(filename), _) => Some(filename.clone()),
            (None, Some(first_path)) => Some(first_path.clone()),
            _ => None,
        }
    }

    /// Apply changes to the config after it's been loaded, e.g. overriding
    /// values in a config file using command-line options.
    ///
    /// This can be safely deleted if you don't want to override config
    /// settings from command-line options.
    fn process_config(&self, config: PaceConfig) -> Result<PaceConfig, FrameworkError> {
        // Override the config file with options from CLI arguments globally
        let config = self.override_config(config)?;

        // You can also override settings based on the subcommand
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
