//! A sandbox for running `pace` without touching the real user directories.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::TempDir;

/// The `pace` binary built for this test run.
pub fn pace() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pace"))
}

/// Run `command` and describe its outcome for a snapshot: exit status,
/// stdout and stderr. Unlike `insta_cmd`, nothing about the environment goes
/// into the snapshot, so sandbox paths never end up in the repository.
pub fn run(command: &mut Command) -> std::io::Result<String> {
    let output = command.output()?;
    Ok(format!(
        "success: {:?}\nexit_code: {}\n----- stdout -----\n{}\n----- stderr -----\n{}",
        output.status.success(),
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

/// A temporary directory with a working directory `work/` and an empty
/// `config/pace.global.toml` inside, and `PACE_CONFIG`, `PACE_HOME` and
/// `PACE_MACHINE_DIR` pointing into it.
pub struct Sandbox {
    root: TempDir,
}

impl Sandbox {
    pub fn new() -> std::io::Result<Self> {
        let root = tempfile::tempdir()?;
        std::fs::create_dir(root.path().join("work"))?;
        std::fs::create_dir(root.path().join("config"))?;
        std::fs::write(root.path().join("config/pace.global.toml"), "")?;
        Ok(Self { root })
    }

    /// Replace the content of the global config file.
    pub fn write_config(&self, content: &str) -> std::io::Result<()> {
        std::fs::write(self.path("config/pace.global.toml"), content)
    }

    pub fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.root.path().join(relative)
    }

    /// `pace` run in `work/`, with the three pace variables set and every
    /// other `PACE_*` variable of the caller removed.
    pub fn pace(&self) -> Command {
        let mut command = self.bare_pace();
        _ = command
            .env("PACE_CONFIG", self.path("config/pace.global.toml"))
            .env("PACE_HOME", self.path("home"))
            .env("PACE_MACHINE_DIR", self.path("machine"));
        command
    }

    /// `pace` run in `work/` without any `PACE_*` variable.
    pub fn bare_pace(&self) -> Command {
        let mut command = pace();
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("PACE_") {
                _ = command.env_remove(key);
            }
        }
        // Wide enough that long temporary paths never wrap, so snapshots are
        // the same on every OS.
        _ = command
            .env_remove("NO_COLOR")
            .env("COLUMNS", "500")
            .current_dir(self.path("work"));
        command
    }

    /// Snapshot settings that replace the sandbox path with `[SANDBOX]` and
    /// use `/` as the path separator on every OS.
    pub fn settings(&self) -> insta::Settings {
        let mut settings = insta::Settings::clone_current();
        let root = self.root.path().display().to_string();
        settings.add_filter(&regex_escape(&root), "[SANDBOX]");
        settings.add_filter(r"\\\\?", "/");
        settings
    }
}

fn regex_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        if "\\.+*?()|[]{}^$#&-~".contains(c) {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}
