//! The Abscissa application.

use abscissa_core::{
    Application, FrameworkError, StandardPaths,
    application::{self, AppCell},
    config::{self, CfgCell},
};

use crate::commands::EntryPoint;

/// The application state.
pub static PACE_APP: AppCell<PaceApp> = AppCell::new();

/// The pace application.
#[derive(Debug, Default)]
pub struct PaceApp {
    config: CfgCell<()>,
    state: application::State<Self>,
}

impl Application for PaceApp {
    type Cmd = EntryPoint;
    type Cfg = ();
    type Paths = StandardPaths;

    fn config(&self) -> config::Reader<()> {
        self.config.read()
    }

    fn state(&self) -> &application::State<Self> {
        &self.state
    }

    fn register_components(&mut self, command: &Self::Cmd) -> Result<(), FrameworkError> {
        let framework_components = self.framework_components(command)?;
        self.state.components_mut().register(framework_components)
    }

    fn after_config(&mut self, config: Self::Cfg) -> Result<(), FrameworkError> {
        self.state.components_mut().after_config(&config)?;
        self.config.set_once(config);
        Ok(())
    }
}
