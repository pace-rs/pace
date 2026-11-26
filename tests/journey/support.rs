//! A sandbox for running `pace` without touching the real user directories.

use std::{
    fmt::Write as _,
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

    /// Create a directory and its parents inside the sandbox.
    pub fn mkdir(&self, relative: &str) -> std::io::Result<()> {
        std::fs::create_dir_all(self.path(relative))
    }

    /// Write a file inside the sandbox, creating its directory.
    pub fn write(&self, relative: &str, content: &str) -> std::io::Result<()> {
        let path = self.path(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)
    }

    /// Copy the device directories of another data directory in the sandbox
    /// into `home/events`, as a file sync would.
    pub fn sync_events_from(&self, other_home: &str) -> std::io::Result<()> {
        copy_dir(
            &self.path(other_home).join("events"),
            &self.path("home/events"),
        )
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

    /// [`Sandbox::pace`] with the clock fixed at `time` (`HH:MM` on
    /// 2026-11-26 in Berlin, or a full RFC 9557 time) and Berlin as the
    /// configured time zone.
    pub fn pace_at(&self, time: &str) -> Command {
        let now = if time.contains('[') {
            time.to_owned()
        } else {
            format!("2026-11-26T{time}[Europe/Berlin]")
        };
        let mut command = self.pace();
        _ = command
            .env("PACE_TEST_NOW", now)
            .env("PACE_TIME_ZONE", "Europe/Berlin");
        command
    }

    /// Run `steps` one after another, each a time as for
    /// [`Sandbox::pace_at`] and the arguments, and describe them for a
    /// snapshot: a line with the command and its time, then [`run`]'s
    /// outcome.
    pub fn journey(&self, steps: &[(&str, &[&str])]) -> Result<String, Box<dyn std::error::Error>> {
        let mut transcript = String::new();
        for (time, args) in steps {
            writeln!(transcript, "$ pace {}   # at {time}", args.join(" "))?;
            transcript.push_str(&run(self.pace_at(time).args(*args))?);
            transcript.push('\n');
        }
        Ok(transcript)
    }

    /// [`Sandbox::pace`] run in `work/<relative>`.
    pub fn pace_in(&self, relative: &str) -> Command {
        let mut command = self.pace();
        _ = command.current_dir(self.path("work").join(relative));
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
        // The working directory of a child is the canonical path: on macOS
        // /private/var/… for /var/…, on Windows with a \\?\ prefix.
        if let Ok(canonical) = std::fs::canonicalize(self.root.path()) {
            let canonical = canonical.display().to_string();
            let plain = canonical.trim_start_matches(r"\\?\").to_owned();
            settings.add_filter(&regex_escape(&canonical), "[SANDBOX]");
            settings.add_filter(&regex_escape(&plain), "[SANDBOX]");
        }
        let root = self.root.path().display().to_string();
        settings.add_filter(&regex_escape(&root), "[SANDBOX]");
        settings.add_filter(r"\\\\?", "/");
        settings
    }
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let path = entry?.path();
        let Some(name) = path.file_name() else {
            continue;
        };
        if path.is_dir() {
            copy_dir(&path, &to.join(name))?;
        } else {
            _ = std::fs::copy(&path, to.join(name))?;
        }
    }
    Ok(())
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
