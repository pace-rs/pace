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

use abscissa_core::{
    config::Override, status_err, status_warn, tracing::debug, Command, Configurable,
    FrameworkError, Runnable,
};
use clap::builder::{styling::AnsiColor, Styles};
use human_panic::setup_panic;
use std::path::PathBuf;

use pace_core::{
    constants::PACE_CONFIG_FILENAME, get_config_paths, ActivityLogFormatKind, PaceConfig,
};

/// Pace Subcommands
/// Subcommands need to be listed in an enum.
#[derive(clap::Parser, Command, Debug, Runnable)]
pub enum PaceCmd {
    /// 📝 Adjust the details of an activity, such as its category, description, or tags.
    #[clap(visible_alias = "a")]
    Adjust(adjust::AdjustCmd),

    /// ⌚ Starts tracking time for an activity.
    #[clap(visible_alias = "b")]
    Begin(begin::BeginCmd),

    /// ⏹️  Stops time tracking for the most recent or all activities.
    #[clap(visible_alias = "e")]
    End(end::EndCmd),

    /// ⏸️  Pauses the time tracking for the most recent active activity.
    #[clap(visible_alias = "h")]
    Hold(hold::HoldCmd),

    /// ⏲️  Shows you at a glance what you're currently tracking.
    #[clap(visible_alias = "n")]
    Now(now::NowCmd),

    /// ⏯️  Resumes a previously paused activity, allowing you to continue where you left off.
    #[clap(visible_alias = "r")]
    Resume(resume::ResumeCmd),

    /// 📈 Get sophisticated insights on your activities.
    #[clap(visible_alias = "rev")]
    Review(review::ReviewCmd),

    /// 🛠️  Set up a pace configuration, a new project, or generate shell completions.
    #[clap(visible_alias = "s")]
    Setup(setup::SetupCmd),

    /// 📚 Open the online documentation for pace.
    #[clap(visible_alias = "d")]
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
#[command(name="pace", author, about, styles=cli_colour_styles(), version, arg_required_else_help = true, propagate_version = true, )]
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
            debug!("Overriding activity log file with: {:?}", activity_log_file);

            // Handle not existing activity log file and parent directory
            match (activity_log_file.parent(), activity_log_file.exists()) {
                (Some(dir), false) if dir.exists() => {
                    std::fs::File::create(activity_log_file)?;
                }
                (Some(dir), false) if !dir.exists() => {
                    std::fs::create_dir_all(dir)?;
                    std::fs::File::create(activity_log_file)?;
                }
                _ => {}
            };

            *config.general_mut().activity_log_options_mut().path_mut() =
                activity_log_file.to_path_buf();

            // Set the activity log format to TOML
            // TODO: This should be configurable
            *config
                .general_mut()
                .activity_log_options_mut()
                .format_kind_mut() = Some(ActivityLogFormatKind::Toml);
        };

        debug!("Overridden config: {:?}", config);

        Ok(config)
    }
}

/// This trait allows you to define how application configuration is loaded.
impl Configurable<PaceConfig> for EntryPoint {
    /// Location of the configuration file
    fn config_path(&self) -> Option<PathBuf> {
        let automatically_determined = get_config_paths(PACE_CONFIG_FILENAME)
            .into_iter()
            .filter(|f| f.exists())
            .collect::<Vec<_>>();

        debug!(
            "Automatically determined config paths: {:?}",
            automatically_determined
        );

        if automatically_determined.is_empty() {
            status_err!(
                "No config file found in standard locations. Please run `pace setup config`."
            );
        }

        if automatically_determined.len() > 1 {
            status_warn!("Multiple config files found in standard locations, we will use the first one found: {:?}", automatically_determined);
        }

        // Get the first path that exists
        // TODO!: Let the user specify the config file location in case there are multiple existing ones
        let first_automatically_determined = automatically_determined.first();

        debug!(
            "First automatically determined config path: {:?}",
            first_automatically_determined
        );

        let user_specified = self.config.as_ref().and_then(|f| {
            if f.exists() {
                Some(f)
            } else {
                // If the parent directory doesn't exist, create it
                if let Some(parent) = f.parent() {
                    std::fs::create_dir_all(parent).ok()?;
                }

                // If the file doesn't exist, create it
                std::fs::File::create(f).ok()?;
                Some(f)
            }
        });

        // If the user has specified a config file, use that
        // otherwise, use the first config file found in specified
        // standard locations
        let config_path = match (user_specified, first_automatically_determined) {
            (Some(filename), _) => Some(filename.clone()),
            (None, Some(first_path)) => Some(first_path.clone()),
            _ => None,
        };

        debug!("Using config path: {:?}", config_path);

        config_path
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
