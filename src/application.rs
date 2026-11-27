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

use crate::clock::TEST_NOW;
use crate::config::load_settings;
use crate::dirs::resolve_dirs;
use crate::editor::editor_variable;
use crate::project::discover_project;
use crate::prompt::Terminal;
use crate::session::device_name;
use crate::{
    commands::EntryPoint,
    config::Loaded,
    dirs::{Dirs, Inputs, Platform},
    project::Project,
};

/// The application state.
pub static PACE_APP: AppCell<PaceApp> = AppCell::new();

/// The pace application.
#[derive(Debug, Default)]
pub struct PaceApp {
    config: CfgCell<()>,
    state: application::State<Self>,
    context: Option<Context>,
    style: Style,
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
    /// The terminal pace asks on, if one is attached.
    pub terminal: Terminal,
    /// The editor the user set in `VISUAL` or `EDITOR`.
    pub editor: Option<String>,
    /// `PACE_TEST_NOW`, the moment that fixes the clock for tests.
    pub test_now: Option<OsString>,
    /// This device's readable name, from the hostname.
    pub host: String,
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
        self.register_components(command).map_err(|error| {
            PaceError::with_source(
                ErrorKind::Internal,
                "cannot register the application's components",
                error,
            )
            .ask_report()
        })?;

        let cwd = std::env::current_dir().map_err(|error| {
            PaceError::with_source(
                ErrorKind::Project,
                "cannot read the working directory",
                error,
            )
            .attach_hint("change into a directory that exists")
        })?;
        let executable = std::env::current_exe().ok();
        let inputs = Inputs::gather(
            command.config.as_deref(),
            command.home.as_deref(),
            |name: &str| -> Option<OsString> { std::env::var_os(name) },
            &cwd,
            executable.as_deref(),
        )?;
        let dirs = resolve_dirs(inputs, Platform::current)?;
        let home = directories::BaseDirs::new().map(|base| base.home_dir().to_path_buf());
        let settings = load_settings(
            &dirs.config_file,
            home.as_deref(),
            |name: &str| -> Option<OsString> { std::env::var_os(name) },
            std::env::vars_os().map(|(name, _)| name),
        )?;
        for warning in &settings.warnings {
            eprint!("{}", render(warning, self.style));
        }
        let project = discover_project(
            command.project.as_deref(),
            std::env::var_os("PACE_PROJECT"),
            &cwd,
            &settings.project_paths,
        )?;
        self.context = Some(Context {
            dirs,
            settings,
            project,
            terminal: Terminal::detect(),
            editor: editor_variable(|name: &str| -> Option<OsString> { std::env::var_os(name) }),
            test_now: std::env::var_os(TEST_NOW),
            host: device_name(&gethostname::gethostname()),
        });

        self.after_config(()).map_err(|error| {
            PaceError::with_source(
                ErrorKind::Internal,
                "the components refused the configuration",
                error,
            )
            .ask_report()
        })
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
        let components = self.framework_components(command)?;
        self.state.components_mut().register(components)
    }

    fn after_config(&mut self, config: Self::Cfg) -> Result<(), FrameworkError> {
        self.state.components_mut().after_config(&config)?;
        self.config.set_once(config);
        Ok(())
    }

    /// Abscissa's `fatal_error` would print an init error without its hint
    /// and always exit with 1, so `init` renders its errors itself.
    fn init(&mut self, command: &Self::Cmd) -> Result<(), FrameworkError> {
        self.style = Style::new(command.plain, |name: &str| -> Option<OsString> {
            std::env::var_os(name)
        });
        if let Err(error) = self.try_init(command) {
            eprint!("{}", render(&error, self.style));
            process::exit(error.exit_code().into());
        }
        Ok(())
    }
}

/// End the process with the error's exit code after rendering it, if there
/// is an error.
pub fn exit_on_error(result: PaceResult<()>) {
    if let Err(error) = result {
        print_error(&error);
        PACE_APP.shutdown_with_exitcode(Shutdown::Crash, error.exit_code().into());
    }
}

/// Print an error the way pace prints errors, and go on.
pub fn print_error(error: &PaceError) {
    eprint!("{}", render(error, PACE_APP.style));
}

/// How pace renders errors, read once at the start. `--plain` means ASCII
/// without colour; `NO_COLOR` turns off colour; `COLUMNS` sets the width,
/// as it does for clap's help. miette reads `NO_GRAPHICS` itself and
/// switches to its narrated output for screen readers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Style {
    plain: bool,
    no_color: bool,
    width: Option<usize>,
}

impl Style {
    fn new(plain: bool, env: impl Fn(&str) -> Option<OsString>) -> Self {
        Self {
            plain,
            no_color: plain || env("NO_COLOR").is_some_and(|value| !value.is_empty()),
            width: env("COLUMNS").and_then(|columns| columns.to_str()?.parse().ok()),
        }
    }
}

/// Render an error with its hint, details and snippet in `style`.
fn render(error: &PaceError, style: Style) -> String {
    let mut options = MietteHandlerOpts::new().unicode(!style.plain);
    if style.no_color {
        options = options.color(false);
    }
    if let Some(width) = style.width {
        options = options.width(width);
    }
    format!("{:?}\n", Rendered(&options.build(), error))
}

struct Rendered<'borrowed>(&'borrowed MietteHandler, &'borrowed PaceError);

impl fmt::Debug for Rendered<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.debug(self.1, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'borrowed>(
        vars: &'borrowed [(&str, &str)],
    ) -> impl Fn(&str) -> Option<OsString> + 'borrowed {
        move |name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        }
    }

    #[test]
    fn no_color_and_columns_shape_style() {
        let style = Style::new(false, env(&[("NO_COLOR", "1"), ("COLUMNS", "80")]));

        assert_eq!(
            style,
            Style {
                plain: false,
                no_color: true,
                width: Some(80),
            }
        );
    }

    #[test]
    fn empty_no_color_and_wrong_columns_change_nothing() {
        let style = Style::new(false, env(&[("NO_COLOR", ""), ("COLUMNS", "wide")]));

        assert_eq!(style, Style::default());
    }

    #[test]
    fn plain_turns_colour_off() {
        assert!(Style::new(true, env(&[])).no_color);
    }
}
