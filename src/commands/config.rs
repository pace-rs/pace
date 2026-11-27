//! `pace config`

use abscissa_core::{Command, Runnable};
use pace_error::PaceResult;

use crate::{
    application::{PACE_APP, exit_on_error},
    dirs::Located,
    print::print_lines,
    project::Project,
};

/// Look at pace's configuration.
#[derive(clap::Subcommand, Command, Debug, Runnable)]
pub enum ConfigCmd {
    /// Show where pace keeps its files, the project and the settings in effect.
    Show(ShowCmd),
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
                    mapping.origin.to_string(),
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
