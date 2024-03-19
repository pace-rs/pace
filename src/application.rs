//! Pace Abscissa Application

use std::path::{Path, PathBuf};

use crate::commands::EntryPoint;
use abscissa_core::{
    application::{self, AppCell},
    config::{self, CfgCell},
    trace, Application, Configurable, FrameworkError, StandardPaths,
};

use pace_core::prelude::PaceConfig;

/// Application state
pub static PACE_APP: AppCell<PaceApp> = AppCell::new();

/// Pace Application
#[derive(Debug)]
pub struct PaceApp {
    /// Application configuration.
    config: CfgCell<PaceConfig>,

    /// Application state.
    state: application::State<Self>,

    /// Config file path
    config_path: PathBuf,
}

/// Initialize a new application instance.
///
/// By default no configuration is loaded, and the framework state is
/// initialized to a default, empty state (no components, threads, etc).
impl Default for PaceApp {
    fn default() -> Self {
        Self {
            config: CfgCell::default(),
            state: application::State::default(),
            config_path: PathBuf::default(),
        }
    }
}

impl Application for PaceApp {
    /// Entrypoint command for this application.
    type Cmd = EntryPoint;

    /// Application configuration.
    type Cfg = PaceConfig;

    /// Paths to resources within the application.
    type Paths = StandardPaths;

    /// Accessor for application configuration.
    fn config(&self) -> config::Reader<PaceConfig> {
        self.config.read()
    }

    /// Borrow the application state immutably.
    fn state(&self) -> &application::State<Self> {
        &self.state
    }

    /// Register all components used by this application.
    ///
    /// If you would like to add additional components to your application
    /// beyond the default ones provided by the framework, this is the place
    /// to do so.
    fn register_components(&mut self, command: &Self::Cmd) -> Result<(), FrameworkError> {
        let framework_components = self.framework_components(command)?;
        let mut app_components = self.state.components_mut();
        app_components.register(framework_components)
    }

    /// Post-configuration lifecycle callback.
    ///
    /// Called regardless of whether config is loaded to indicate this is the
    /// time in app lifecycle when configuration would be loaded if
    /// possible.
    fn after_config(&mut self, config: Self::Cfg) -> Result<(), FrameworkError> {
        // Configure components
        let mut components = self.state.components_mut();
        components.after_config(&config)?;
        self.config.set_once(config);
        Ok(())
    }

    /// Get tracing configuration from command-line options
    fn tracing_config(&self, command: &EntryPoint) -> trace::Config {
        if command.verbose {
            trace::Config::verbose()
        } else {
            trace::Config::default()
        }
    }

    fn init(&mut self, command: &Self::Cmd) -> Result<(), FrameworkError> {
        // Create and register components with the application.
        // We do this first to calculate a proper dependency ordering before
        // application configuration is processed
        self.register_components(command)?;

        // Load configuration
        let config = command
            .config_path()
            .map(|path| self.load_config(&path))
            .transpose()?
            .unwrap_or_default();

        // Set the config file path that was used to load the config
        // in the current application state
        self.set_config_path(command.config_path().unwrap_or_default());

        // Fire callback regardless of whether any config was loaded to
        // in order to signal state in the application lifecycle
        self.after_config(command.process_config(config)?)?;

        Ok(())
    }
}

impl PaceApp {
    /// Get the config file path
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Set the config file path
    pub fn set_config_path(&mut self, config_path: PathBuf) {
        self.config_path = config_path;
    }
}
