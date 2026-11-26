//! The Abscissa application.
//!
//! Errors from loading the configuration and from the commands are rendered
//! with their hints and end the process with pace's
//! [`ExitCode`](pace_error::ExitCode)s.

use std::{ffi::OsString, fmt, process};

use abscissa_core::{
    Application, FrameworkError, Shutdown, StandardPaths,
    application::{self, AppCell},
    config::{CfgCell, Reader},
};
use miette::{MietteHandler, MietteHandlerOpts, ReportHandler};
use pace_error::{ErrorKind, PaceError, PaceResult};

use crate::{
    commands::EntryPoint,
    config::{self, Loaded},
    dirs::{self, Dirs, Inputs, Platform},
    project::{self, Project},
};

/// The application state.
pub static PACE_APP: AppCell<PaceApp> = AppCell::new();

/// The pace application.
#[derive(Debug, Default)]
pub struct PaceApp {
    config: CfgCell<()>,
    state: application::State<Self>,
    context: Option<Context>,
    plain: bool,
}

/// What pace found out before running a command.
#[derive(Debug)]
pub struct Context {
    /// Where pace keeps its files.
    pub dirs: Dirs,
    /// The settings and where each comes from.
    pub settings: Loaded,
    /// The project of the working directory, if any.
    pub project: Option<Project>,
}

impl PaceApp {
    /// What pace found out before running the command.
    ///
    /// # Errors
    ///
    /// Only if called before the application is initialised, which is a bug.
    pub fn context(&self) -> PaceResult<&Context> {
        self.context.as_ref().ok_or_else(|| {
            PaceError::new(
                ErrorKind::Internal,
                "the context was used before the application was initialised",
            )
            .ask_report()
        })
    }

    fn try_init(&mut self, command: &EntryPoint) -> PaceResult<()> {
        self.plain = command.plain;
        self.register_components(command)
            .map_err(|error| PaceError::from(ErrorKind::Internal, error).ask_report())?;

        let inputs = Inputs::gather(
            command.config.as_deref(),
            command.home.as_deref(),
            |name: &str| -> Option<OsString> { std::env::var_os(name) },
        )?;
        let dirs = dirs::resolve(inputs, Platform::current)?;
        let home = directories::BaseDirs::new().map(|base| base.home_dir().to_path_buf());
        let settings = config::load(
            &dirs.config_file,
            home.as_deref(),
            |name: &str| -> Option<OsString> { std::env::var_os(name) },
            std::env::vars_os().map(|(name, _)| name),
        )?;
        for warning in &settings.warnings {
            eprint!("{}", render(warning, self.plain));
        }
        let cwd = std::env::current_dir().map_err(|error| {
            PaceError::with_source(
                ErrorKind::Project,
                "cannot read the working directory",
                error,
            )
            .attach_hint("change into a directory that exists")
        })?;
        let project = project::discover(
            command.project.as_deref(),
            std::env::var_os("PACE_PROJECT"),
            &cwd,
            &settings.project_paths,
        )?;
        self.context = Some(Context {
            dirs,
            settings,
            project,
        });

        self.after_config(())
            .map_err(|error| PaceError::from(ErrorKind::Internal, error).ask_report())
    }
}

impl Application for PaceApp {
    type Cmd = EntryPoint;
    type Cfg = ();
    type Paths = StandardPaths;

    fn config(&self) -> Reader<()> {
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

    /// Abscissa's `fatal_error` would print an init error without its hint
    /// and always exit with 1, so `init` renders its errors itself.
    fn init(&mut self, command: &Self::Cmd) -> Result<(), FrameworkError> {
        if let Err(error) = self.try_init(command) {
            eprint!("{}", render(&error, command.plain));
            process::exit(error.exit_code().into());
        }
        Ok(())
    }
}

/// End the process with the error's exit code after rendering it, if there
/// is an error.
pub fn exit_on_error(result: PaceResult<()>) {
    if let Err(error) = result {
        eprint!("{}", render(&error, PACE_APP.plain));
        PACE_APP.shutdown_with_exitcode(Shutdown::Crash, error.exit_code().into());
    }
}

/// Render an error with its hint, details and snippet. `--plain` means ASCII
/// without colour; `NO_COLOR` turns off colour; `NO_GRAPHICS` switches to
/// miette's narrated output for screen readers. `COLUMNS` sets the width,
/// as it does for clap's help.
fn render(error: &PaceError, plain: bool) -> String {
    let no_color = plain || std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
    let mut options = MietteHandlerOpts::new().unicode(!plain);
    if no_color {
        options = options.color(false);
    }
    if let Some(width) = std::env::var("COLUMNS")
        .ok()
        .and_then(|columns| columns.parse().ok())
    {
        options = options.width(width);
    }
    format!("{:?}\n", Rendered(&options.build(), error))
}

struct Rendered<'a>(&'a MietteHandler, &'a PaceError);

impl fmt::Debug for Rendered<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.debug(self.1, f)
    }
}
