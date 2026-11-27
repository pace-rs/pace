//! `pace config`

use abscissa_core::{Command, Runnable};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_store::field_edit::{FieldEdit, FieldValue, write_field_edits};

use crate::{
    application::{Context, PACE_APP, exit_on_error},
    config::{Key, PROJECT_PATHS, setting_value},
    dirs::Located,
    origin::Origin,
    output::usage,
    print::print_lines,
    project::Project,
};

/// Look at pace's configuration, and change a setting.
#[derive(clap::Subcommand, Command, Debug, Runnable)]
pub enum ConfigCmd {
    /// Show where pace keeps its files, the project and the settings in effect.
    Show(ShowCmd),
    /// Print the value of one setting.
    Get(GetCmd),
    /// Write one setting into the global config file.
    Set(SetCmd),
}

/// `pace config get`
#[derive(clap::Args, Command, Debug)]
pub struct GetCmd {
    #[arg(
        value_name = "KEY",
        help = "The setting, such as `time-zone` or `holds.default-source`"
    )]
    key: String,

    /// Say where the value comes from.
    #[arg(long)]
    origin: bool,
}

impl Runnable for GetCmd {
    fn run(&self) {
        exit_on_error(PACE_APP.context().and_then(|context| self.execute(context)));
    }
}

impl GetCmd {
    fn execute(&self, context: &Context) -> PaceResult<()> {
        let key = setting_key(&self.key, "`pace config show` shows it")?;
        let (value, origin) = context.settings.entry(key);
        let shown = plain_value(&value);
        let line = if self.origin {
            format!("{shown}  # {origin}")
        } else {
            shown
        };
        print_lines([line])
    }
}

/// `pace config set`
#[derive(clap::Args, Command, Debug)]
pub struct SetCmd {
    #[arg(
        value_name = "KEY",
        help = "The setting, such as `time-zone` or `holds.default-source`"
    )]
    key: String,

    #[arg(value_name = "VALUE", help = "The new value, such as `Europe/Vienna`")]
    value: String,
}

impl Runnable for SetCmd {
    fn run(&self) {
        exit_on_error(PACE_APP.context().and_then(|context| self.execute(context)));
    }
}

impl SetCmd {
    fn execute(&self, context: &Context) -> PaceResult<()> {
        let path = &context.dirs.config_file.path;
        let file = path.display().to_string();
        let key = setting_key(&self.key, &format!("edit [project-paths] in {file}"))?;
        let value = setting_value(key, &self.value).map_err(usage)?;
        let shown = plain_value(&value);
        if context.settings.file_value(key) == Some(&value) {
            return Err(PaceError::new(
                ErrorKind::InvalidInput,
                "nothing to change: {file} sets {key} to {value} already",
            )
            .attach_context("file", file)
            .attach_context("key", key.name())
            .attach_context("value", shown)
            .attach_exit_code(ExitCode::NothingToDo));
        }
        let parts: Vec<&str> = key.name().split('.').collect();
        write_field_edits(vec![FieldEdit::set(path, &parts, field_value(value)?)])?;
        let overridden = match context.settings.entry(key).1 {
            Origin::Env(variable) => format!("; {variable} overrides it"),
            _ => String::new(),
        };
        print_lines([format!(
            "Set {} to {shown} in {file}{overridden}.",
            key.name()
        )])
    }
}

/// A setting's value as the field writer writes it.
///
/// # Errors
///
/// If the value is neither text nor a whole number, which is a bug.
fn field_value(value: toml::Value) -> PaceResult<FieldValue> {
    match value {
        toml::Value::String(text) => Ok(FieldValue::String(text)),
        toml::Value::Integer(number) => Ok(FieldValue::Integer(number)),
        other => Err(PaceError::new(
            ErrorKind::Internal,
            "a setting has the value {value}, which is neither text nor a whole number",
        )
        .attach_context("value", other.to_string())
        .ask_report()),
    }
}

/// The setting `name` of the command line; `table_hint` says where to go
/// for `[project-paths]`, which is a table, not one setting.
///
/// # Errors
///
/// If `name` is `project-paths` or no setting pace knows: the error exits
/// with 2.
fn setting_key(name: &str, table_hint: &str) -> PaceResult<Key> {
    if name == PROJECT_PATHS {
        return Err(PaceError::new(
            ErrorKind::InvalidInput,
            "`{key}` is a table of directories, not one setting",
        )
        .attach_context("key", name)
        .attach_hint(table_hint)
        .attach_exit_code(ExitCode::Usage));
    }
    Key::named(name)
}

/// A setting's value as a script wants it: text without quotes.
fn plain_value(value: &toml::Value) -> String {
    match value {
        toml::Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// `pace config show`
#[derive(clap::Args, Command, Debug)]
pub struct ShowCmd {
    /// Say where each value comes from.
    #[arg(long)]
    origin: bool,
}

impl Runnable for ShowCmd {
    fn run(&self) {
        exit_on_error(self.execute());
    }
}

impl ShowCmd {
    fn execute(&self) -> PaceResult<()> {
        let context = PACE_APP.context()?;
        let dirs = &context.dirs;
        let places = [
            (
                "config file",
                &dirs.config_file,
                dirs.config_file.path.is_file(),
            ),
            ("data", &dirs.data, dirs.data.path.is_dir()),
            ("machine", &dirs.machine, dirs.machine.path.is_dir()),
        ];
        let mut lines: Vec<String> = places
            .into_iter()
            .map(|(label, located, exists)| self.describe_place(label, located, exists))
            .collect();
        lines.push(self.describe_project(context.project.as_ref()));

        let loaded = &context.settings;
        lines.push(String::new());
        // Top-level keys first, then each table: TOML needs that order.
        let entries = loaded.entries();
        let mut tables: Vec<&str> = Vec::new();
        for (key, _, _) in &entries {
            if let Some((table, _)) = key.split_once('.')
                && !tables.contains(&table)
            {
                tables.push(table);
            }
        }
        lines.extend(
            self.toml_lines(
                entries
                    .iter()
                    .filter(|(key, _, _)| !key.contains('.'))
                    .map(|(key, value, origin)| (format!("{key} = {value}"), origin.to_string())),
            ),
        );
        for table in tables {
            lines.extend([String::new(), format!("[{table}]")]);
            lines.extend(
                self.toml_lines(entries.iter().filter_map(|(key, value, origin)| {
                    let inner = key.strip_prefix(table)?.strip_prefix('.')?;
                    Some((format!("{inner} = {value}"), origin.to_string()))
                })),
            );
        }

        if !loaded.project_paths.is_empty() {
            lines.extend([String::new(), "[project-paths]".to_owned()]);
            lines.extend(self.toml_lines(loaded.project_paths.iter().map(|mapping| {
                (
                    format!(
                        "{} = {}",
                        toml_key(&mapping.written),
                        toml::Value::String(mapping.key.to_string())
                    ),
                    mapping.origin().to_string(),
                )
            })));
        }
        print_lines(lines)
    }

    fn describe_place(&self, label: &str, located: &Located, exists: bool) -> String {
        let place = format!("# {:<12} {}", format!("{label}:"), located.path.display());
        if !self.origin {
            return place;
        }
        let presence = if exists { "" } else { ", not there yet" };
        format!("{place} ({}{presence})", located.origin)
    }

    fn describe_project(&self, project: Option<&Project>) -> String {
        let Some(project) = project else {
            let reason = if self.origin {
                " (no --project, PACE_PROJECT, manifest or path mapping)"
            } else {
                ""
            };
            return format!("# {:<12} none{reason}", "project:");
        };
        let name = project
            .name
            .as_ref()
            .map_or_else(String::new, |name| format!(": {name}"));
        let reason = if self.origin {
            format!(" ({})", project.reason)
        } else {
            String::new()
        };
        format!("# {:<12} {}{name}{reason}", "project:", project.key)
    }

    /// TOML lines; with `--origin`, aligned with their origin.
    fn toml_lines(&self, lines: impl Iterator<Item = (String, String)>) -> Vec<String> {
        let lines: Vec<_> = lines.collect();
        let width = lines.iter().map(|(line, _)| line.len()).max().unwrap_or(0);
        lines
            .into_iter()
            .map(|(line, origin)| {
                if self.origin {
                    format!("{line:<width$}  # {origin}")
                } else {
                    line
                }
            })
            .collect()
    }
}

/// A key as TOML: a literal string `'…'` if possible, so paths need no
/// escaped backslashes on Windows.
fn toml_key(key: &str) -> String {
    if key.contains(['\'', '\n', '\r']) {
        toml::Value::String(key.to_owned()).to_string()
    } else {
        format!("'{key}'")
    }
}
