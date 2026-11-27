//! The Abscissa application.
//!
//! Errors from loading the configuration and from the commands are rendered
//! with their hints and end the process with pace's
//! [`ExitCode`](pace_error::ExitCode)s.

use std::{
    ffi::OsString,
    fmt,
    path::{Path, PathBuf},
    process,
};

use abscissa_core::{
    Application, FrameworkError, Shutdown, StandardPaths,
    application::{self, AppCell},
    config::{CfgCell, Reader},
};
use jiff::civil::Date;
use miette::{MietteHandler, MietteHandlerOpts, ReportHandler};
use pace_core::Catalog;
use pace_error::{ErrorKind, PaceError, PaceResult};
use pace_store::catalog::{MendedCatalog, points_at_file, read_catalog, read_catalog_mending};

use crate::clock::{TEST_NOW, clock_for_run};
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
    project::{Project, Records},
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
    /// The roles, clients and project records in `PACE_HOME`, read once;
    /// empty for a command that mends a catalog that doesn't read.
    pub catalog: Catalog,
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
    /// The working directory pace runs in.
    pub cwd: PathBuf,
    /// `--plain`: output without colour and without symbols.
    pub plain: bool,
}

impl Context {
    /// Today in the configured time zone, as the clock of this run sees
    /// it.
    ///
    /// # Errors
    ///
    /// If `PACE_TEST_NOW` is set but no RFC 9557 time, or the system's zone
    /// is unknown.
    pub fn today(&self) -> PaceResult<Date> {
        let clock = clock_for_run(&self.settings.settings.time_zone, self.test_now.clone())?;
        Ok(clock.now().date())
    }
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
        // `config set` creates the file it writes.
        let creates_config_file = command.writes_config_file() && !dirs.config_file.path.exists();
        let settings = load_settings(
            (!creates_config_file).then_some(&dirs.config_file),
            home.as_deref(),
            |name: &str| -> Option<OsString> { std::env::var_os(name) },
            std::env::vars_os().map(|(name, _)| name),
        )?;
        for warning in &settings.warnings {
            eprint!("{}", render(warning, self.style));
        }
        let catalog = catalog_for(
            &dirs.data.path,
            command.mended_catalog_file(&dirs.data.path).as_deref(),
        )?;
        let project = discover_project(
            command.project.as_deref(),
            std::env::var_os("PACE_PROJECT"),
            &cwd,
            &settings.project_paths,
            Records {
                catalog: &catalog,
                home: &dirs.data.path,
            },
        )?;
        self.context = Some(Context {
            dirs,
            settings,
            catalog,
            project,
            terminal: Terminal::detect(),
            editor: editor_variable(|name: &str| -> Option<OsString> { std::env::var_os(name) }),
            test_now: std::env::var_os(TEST_NOW),
            host: device_name(&gethostname::gethostname()),
            cwd,
            plain: command.plain,
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

/// The catalog in `home`. A command that mends the catalog file `mended`,
/// a path in `home`, gets an empty catalog when a mistake in any file
/// keeps the catalog from reading, so long as `mended` reads alone or has
/// the mistake itself: it reads the file it mends itself. So each file
/// that reads alone can be mended while another file has a mistake.
///
/// # Errors
///
/// If a file can't be read, or every file reads and [`Catalog::new`]
/// refuses the records for a mistake in another file: the error points at
/// the line.
fn catalog_for(home: &Path, mended: Option<&Path>) -> PaceResult<Catalog> {
    let Some(mended) = mended else {
        return read_catalog(home);
    };
    match read_catalog_mending(home, mended, None) {
        Ok(MendedCatalog::Whole(catalog)) => Ok(catalog),
        Ok(MendedCatalog::Alone(..)) => Ok(Catalog::default()),
        Err(error) if points_at_file(&error, mended) => Ok(Catalog::default()),
        Err(error) => Err(error),
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

struct Rendered<'error>(&'error MietteHandler, &'error PaceError);

impl fmt::Debug for Rendered<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.debug(self.1, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'vars>(vars: &'vars [(&str, &str)]) -> impl Fn(&str) -> Option<OsString> + 'vars {
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

    fn home_with(file: &str, text: &str) -> tempfile::TempDir {
        let home = pace_store::testing::scratch_dir().unwrap();
        std::fs::write(home.path().join(file), text).unwrap();
        home
    }

    fn broken_home() -> tempfile::TempDir {
        home_with("roles.toml", "version = 1\n\n[work]\nkind = \"aera\"\n")
    }

    #[test]
    fn broken_catalog_stops_command_that_does_not_mend_it() {
        let home = broken_home();

        let error = catalog_for(home.path(), None).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::InvalidInput);
    }

    #[test]
    fn broken_catalog_is_empty_for_command_that_mends_broken_file() {
        let home = broken_home();

        assert_eq!(
            catalog_for(home.path(), Some(&home.path().join("roles.toml"))).unwrap(),
            Catalog::default()
        );
    }

    #[test]
    fn mistake_in_other_file_leaves_empty_catalog_to_command_that_mends_roles_file() {
        let home = home_with("clients.toml", "version = 1\n\n[acme]\nname = 1\n");

        let roles = catalog_for(home.path(), Some(&home.path().join("roles.toml")));

        assert_eq!(roles.unwrap(), Catalog::default());
    }

    #[test]
    fn mistake_of_mended_file_lets_command_run_after_mistake_of_earlier_file() {
        let home = home_with("clients.toml", "version = 1\n\n[acme]\nname = 1\n");
        std::fs::write(
            home.path().join("roles.toml"),
            "version = 1\n\n[work]\nkind = \"aera\"\n",
        )
        .unwrap();

        let clients = catalog_for(home.path(), Some(&home.path().join("clients.toml")));

        assert_eq!(clients.unwrap(), Catalog::default());
    }

    #[test]
    fn plain_turns_colour_off() {
        assert!(Style::new(true, env(&[])).no_color);
    }
}
