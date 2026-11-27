//! A sandbox for running `pace` without touching the real user directories.

use std::{
    collections::BTreeSet,
    fmt::Write as _,
    path::{Path, PathBuf},
    process::Command,
};

use pace_rs::project::MANIFEST_FILES;
use pace_store::testing::scratch_dir;
use tempfile::TempDir;
use uuid::Uuid;

/// The `pace` binary built for this test run.
pub(crate) fn pace() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pace"))
}

/// Run `command` and describe its outcome for a snapshot: exit status,
/// stdout and stderr. Unlike `insta_cmd`, nothing about the environment goes
/// into the snapshot, so sandbox paths never end up in the repository.
pub(crate) fn run(command: &mut Command) -> std::io::Result<String> {
    let output = command.output()?;
    Ok(format!(
        "success: {:?}\nexit_code: {}\n----- stdout -----\n{}\n----- stderr -----\n{}",
        output.status.success(),
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

/// The short ID that follows `prefix` at the start of a line of `output`,
/// such as `ad5e` after `Began ` in `Began ad5e "Write offer" at 09:00.`
pub(crate) fn short_id_after(output: &str, prefix: &str) -> Result<String, String> {
    output
        .lines()
        .find_map(|line| line.strip_prefix(prefix))
        .and_then(|rest| rest.split_whitespace().next())
        .map(str::to_owned)
        .ok_or_else(|| format!("no line starts with {prefix:?} and a short ID"))
}

/// All event files of the sandbox, as one text; empty before the first event.
pub(crate) fn event_text(sandbox: &Sandbox) -> Result<String, Box<dyn std::error::Error>> {
    let mut text = String::new();
    let events = sandbox.path("home/events");
    if !events.is_dir() {
        return Ok(text);
    }
    for device in std::fs::read_dir(events)? {
        let file = device?.path().join("2026/2026-11.toml");
        if file.is_file() {
            text.push_str(&std::fs::read_to_string(file)?);
        }
    }
    Ok(text)
}

/// The transcript of `steps`, then the event files as pace left them.
pub(crate) fn journey_and_events(
    sandbox: &Sandbox,
    steps: &[(&str, &[&str])],
) -> Result<String, Box<dyn std::error::Error>> {
    let transcript = sandbox.journey(steps)?;
    let events = event_text(sandbox)?;
    Ok(format!(
        "{transcript}--- home/events ---\n{}",
        sandbox.redact_ids(&events)?
    ))
}

/// An entry begun on another machine, apart: it has its own data
/// directory, which is then synced into this one.
pub(crate) fn begin_apart(
    sandbox: &Sandbox,
    time: &str,
    note: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    _ = run(sandbox
        .pace_at(time)
        .env("PACE_HOME", sandbox.path("other-home"))
        .env("PACE_MACHINE_DIR", sandbox.path("other-machine"))
        .args(["begin", note]))?;
    sandbox.sync_events_from("other-home")?;
    Ok(())
}

/// Stop at a manifest in a folder above the sandbox `root`. pace searches
/// upward for the nearest manifest, so every test would take its project, and
/// about 30 snapshots would fail without saying why.
fn refuse_manifest_above(root: &Path) -> std::io::Result<()> {
    for dir in root.ancestors().skip(1) {
        for name in MANIFEST_FILES {
            let manifest = dir.join(name);
            if manifest.is_file() {
                return Err(std::io::Error::other(format!(
                    "the sandbox {} lies below the manifest {}, which pace would find in \
                     every test: move the manifest, or set the temporary folder (TMPDIR, or \
                     TMP on Windows) to a folder outside it",
                    root.display(),
                    manifest.display()
                )));
            }
        }
    }
    Ok(())
}

/// A temporary directory with a working directory `work/`, an empty
/// `config/pace.global.toml` and the temporary directory `tmp/` inside, and
/// `PACE_CONFIG`, `PACE_HOME`, `PACE_MACHINE_DIR` and `TMPDIR` pointing into
/// it.
pub(crate) struct Sandbox {
    root: TempDir,
}

impl Sandbox {
    pub(crate) fn new() -> std::io::Result<Self> {
        let root = scratch_dir()?;
        refuse_manifest_above(root.path())?;
        std::fs::create_dir(root.path().join("work"))?;
        std::fs::create_dir(root.path().join("config"))?;
        std::fs::create_dir(root.path().join("tmp"))?;
        std::fs::write(root.path().join("config/pace.global.toml"), "")?;
        Ok(Self { root })
    }

    /// Create a directory and its parents inside the sandbox.
    pub(crate) fn mkdir(&self, relative: &str) -> std::io::Result<()> {
        std::fs::create_dir_all(self.path(relative))
    }

    /// Write a file inside the sandbox, creating its directory.
    pub(crate) fn write(&self, relative: &str, content: &str) -> std::io::Result<()> {
        let path = self.path(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)
    }

    /// Copy the device directories of another data directory in the sandbox
    /// into `home/events`, as a file sync would.
    pub(crate) fn sync_events_from(&self, other_home: &str) -> std::io::Result<()> {
        copy_dir(
            &self.path(other_home).join("events"),
            &self.path("home/events"),
        )
    }

    /// Copy the device directories of `home/events` into another data
    /// directory in the sandbox, as a file sync would.
    pub(crate) fn sync_events_to(&self, other_home: &str) -> std::io::Result<()> {
        copy_dir(
            &self.path("home/events"),
            &self.path(other_home).join("events"),
        )
    }

    /// Replace the content of the global config file.
    pub(crate) fn write_config(&self, content: &str) -> std::io::Result<()> {
        std::fs::write(self.path("config/pace.global.toml"), content)
    }

    pub(crate) fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.root.path().join(relative)
    }

    /// `pace` run in `work/`, with the three pace variables set and every
    /// other `PACE_*` variable of the caller removed.
    pub(crate) fn pace(&self) -> Command {
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
    pub(crate) fn pace_at(&self, time: &str) -> Command {
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
    /// outcome, with the IDs redacted by [`Sandbox::redact_ids`].
    pub(crate) fn journey(
        &self,
        steps: &[(&str, &[&str])],
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.journey_in("", steps)
    }

    /// [`Sandbox::journey`] with each step run in `work/<relative>`; the
    /// line of a step names the directory, unless it is `work/` itself.
    pub(crate) fn journey_in(
        &self,
        relative: &str,
        steps: &[(&str, &[&str])],
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.redact_ids(&self.transcript_in(relative, steps)?)
    }

    /// The transcript of [`Sandbox::journey_in`] before its IDs are
    /// redacted, for a test that adds files to it and redacts the whole.
    pub(crate) fn transcript_in(
        &self,
        relative: &str,
        steps: &[(&str, &[&str])],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut transcript = String::new();
        let place = if relative.is_empty() {
            String::new()
        } else {
            format!(" in work/{relative}")
        };
        for (time, args) in steps {
            writeln!(transcript, "$ pace {}   # at {time}{place}", args.join(" "))?;
            let mut command = self.pace_at(time);
            _ = command.current_dir(self.path("work").join(relative));
            transcript.push_str(&run(command.args(*args))?);
            transcript.push('\n');
        }
        Ok(transcript)
    }

    /// `text` with every ID that pace made in this sandbox replaced by
    /// `[ID-1]`, `[ID-2]`, …, numbered in the order they first appear.
    /// pace's IDs are random, so a snapshot shows that an ID is there and
    /// which ones are the same, not its value. An entry's short ID (four or
    /// more of its last hex characters) gets the number of its full ID.
    ///
    /// The IDs come from the event files in the sandbox: device IDs and the
    /// IDs of events, batches and entries. An ID that the test wrote itself,
    /// such as a project ID in a manifest, stays.
    pub(crate) fn redact_ids(&self, text: &str) -> Result<String, Box<dyn std::error::Error>> {
        let made = MadeIds::find(self.root.path())?;
        let mut numbered: Vec<Uuid> = Vec::new();
        let mut redacted = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(start) = rest.find(is_word_char) {
            redacted.push_str(&rest[..start]);
            rest = &rest[start..];
            let end = rest.find(|c| !is_word_char(c)).unwrap_or(rest.len());
            let word = &rest[..end];
            if let Some(id) = made.resolve(word)? {
                let number = numbered
                    .iter()
                    .position(|known| *known == id)
                    .unwrap_or_else(|| {
                        numbered.push(id);
                        numbered.len() - 1
                    });
                write!(redacted, "[ID-{}]", number + 1)?;
            } else {
                redacted.push_str(word);
            }
            rest = &rest[end..];
        }
        redacted.push_str(rest);
        Ok(redacted)
    }

    /// [`Sandbox::pace`] run in `work/<relative>`.
    pub(crate) fn pace_in(&self, relative: &str) -> Command {
        let mut command = self.pace();
        _ = command.current_dir(self.path("work").join(relative));
        command
    }

    /// `pace` run in `work/` without any `PACE_*` variable, and without the
    /// variables by which the caller's shell sets colour, graphics or width.
    pub(crate) fn bare_pace(&self) -> Command {
        self.bare(pace())
    }

    /// Copy the `pace` binary into the sandbox directory `relative`, for
    /// portable mode, and return the path of the copy.
    pub(crate) fn copy_pace_to(&self, relative: &str) -> std::io::Result<PathBuf> {
        let binary = Path::new(env!("CARGO_BIN_EXE_pace"));
        let name = binary.file_name().ok_or_else(|| {
            std::io::Error::other(format!("{} has no file name", binary.display()))
        })?;
        let copy = self.path(relative).join(name);
        std::fs::create_dir_all(self.path(relative))?;
        _ = std::fs::copy(binary, &copy)?;
        Ok(copy)
    }

    /// The copy `binary` of [`Sandbox::copy_pace_to`], run as
    /// [`Sandbox::bare_pace`] runs, with `HOME` and the XDG directories in the
    /// sandbox's `fallback-home/`. So if portable mode breaks, pace falls
    /// back to the sandbox, not to the caller's real directories.
    pub(crate) fn portable_pace(&self, binary: &Path) -> Command {
        let home = self.path("fallback-home");
        let mut command = self.bare(Command::new(binary));
        _ = command
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", home.join(".config"))
            .env("XDG_DATA_HOME", home.join(".local/share"))
            .env("XDG_STATE_HOME", home.join(".local/state"));
        command
    }

    /// `command` without the caller's `PACE_*` and output variables, in
    /// `work/`.
    fn bare(&self, mut command: Command) -> Command {
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("PACE_") {
                _ = command.env_remove(key);
            }
        }
        for key in HOST_OUTPUT_VARIABLES {
            _ = command.env_remove(key);
        }
        // The editor's file and what a scripted editor leaves beside it, such
        // as the backup of `sed -i.bak`, go into the sandbox, not the
        // system's temporary directory.
        for key in ["TMPDIR", "TMP", "TEMP"] {
            _ = command.env(key, self.path("tmp"));
        }
        // Wide enough that long temporary paths never wrap, so snapshots are
        // the same on every OS.
        _ = command.env("COLUMNS", "500").current_dir(self.path("work"));
        command
    }

    /// Snapshot settings that replace the sandbox path with `[SANDBOX]` and
    /// use `/` as the path separator on every OS.
    pub(crate) fn settings(&self) -> insta::Settings {
        let mut settings = insta::Settings::clone_current();
        // The working directory of a child is the canonical path: on macOS
        // /private/var/… for /var/…, on Windows with a \\?\ prefix.
        if let Ok(canonical) = std::fs::canonicalize(self.root.path()) {
            let canonical = canonical.display().to_string();
            let plain = canonical.trim_start_matches(r"\\?\").to_owned();
            add_sandbox_filter(&mut settings, &canonical);
            add_sandbox_filter(&mut settings, &plain);
        }
        add_sandbox_filter(&mut settings, &self.root.path().display().to_string());
        settings.add_filter(r"\\\\?", "/");
        settings
    }
}

/// Replaces `path` with `[SANDBOX]`, also as JSON writes it: on Windows JSON
/// doubles each `\` of the path.
fn add_sandbox_filter(settings: &mut insta::Settings, path: &str) {
    let escaped = path.replace('\\', r"\\");
    if escaped != path {
        settings.add_filter(&regex_escape(&escaped), "[SANDBOX]");
    }
    settings.add_filter(&regex_escape(path), "[SANDBOX]");
}

/// The variables by which a shell turns colour or miette's graphics on or
/// off. A journey test removes them, so its snapshot is the same in every
/// shell.
const HOST_OUTPUT_VARIABLES: [&str; 5] = [
    "NO_COLOR",
    "CLICOLOR",
    "CLICOLOR_FORCE",
    "FORCE_COLOR",
    "NO_GRAPHICS",
];

/// A word that may be an ID: letters, digits and hyphens.
const fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-'
}

/// The fields of an event that hold an ID pace made.
const ID_FIELDS: [&str; 4] = ["id", "batch", "entry", "undoes"];

/// The IDs pace made in a sandbox, read from every `events` directory in it,
/// from every roles file, `roles.toml`, and from every project record,
/// `projects/<key>.toml`.
#[derive(Default)]
struct MadeIds {
    all: BTreeSet<Uuid>,
    entries: BTreeSet<Uuid>,
}

impl MadeIds {
    /// The IDs in the event files, the roles files and the project records
    /// below `root`. Other TOML files, such as manifests, hold IDs a test
    /// wrote, so they are not read.
    fn find(root: &Path) -> std::io::Result<Self> {
        let mut made = Self::default();
        // Each directory, and whether it is an `events` directory or below one.
        let mut pending = vec![(root.to_path_buf(), false)];
        while let Some((dir, below_events)) = pending.pop() {
            let is_events = dir.file_name().is_some_and(|name| name == "events");
            for child in std::fs::read_dir(&dir)? {
                let path = child?.path();
                if path.is_dir() {
                    if is_events
                        && let Some(device) = path
                            .file_name()
                            .and_then(|name| Uuid::parse_str(&name.to_string_lossy()).ok())
                    {
                        _ = made.all.insert(device);
                    }
                    pending.push((path, below_events || is_events));
                } else if (below_events
                    && path
                        .extension()
                        .is_some_and(|extension| extension == "toml"))
                    || path.file_name().is_some_and(|name| name == "roles.toml")
                    || is_project_record(&path)
                {
                    made.read(&std::fs::read_to_string(&path)?);
                }
            }
        }
        Ok(made)
    }

    /// The IDs in the lines `field = "…"` of an event file, a roles file
    /// or a project record.
    fn read(&mut self, text: &str) {
        for line in text.lines() {
            let Some((field, value)) = line.split_once(" = ") else {
                continue;
            };
            let Ok(id) = Uuid::parse_str(value.trim_matches('"')) else {
                continue;
            };
            if ID_FIELDS.contains(&field) {
                _ = self.all.insert(id);
                if field == "entry" {
                    _ = self.entries.insert(id);
                }
            }
        }
    }

    /// The ID that `word` names: a full ID pace made, or the last four or
    /// more hex characters of exactly one entry ID.
    fn resolve(&self, word: &str) -> Result<Option<Uuid>, String> {
        if let Ok(id) = Uuid::parse_str(word) {
            return Ok(self.all.contains(&id).then_some(id));
        }
        if word.len() < 4
            || !word
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Ok(None);
        }
        let fitting: Vec<&Uuid> = self
            .entries
            .iter()
            .filter(|id| id.simple().to_string().ends_with(word))
            .collect();
        match fitting.as_slice() {
            [] => Ok(None),
            [id] => Ok(Some(**id)),
            several => Err(format!(
                "`{word}` ends {} entry IDs, so the snapshot can't tell them apart; \
                 IDs are random, run the test again",
                several.len()
            )),
        }
    }
}

/// Whether `path` is a project record: a TOML file in a `projects`
/// directory.
fn is_project_record(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "toml")
        && path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == "projects")
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

#[cfg(test)]
mod tests {
    use pace_store::testing::scratch_dir;

    use crate::support::{Sandbox, refuse_manifest_above, short_id_after};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn manifest_above_sandbox_is_refused() -> TestResult {
        let outside = scratch_dir()?;
        let manifest = outside.path().join(".pace.toml");
        std::fs::write(&manifest, "version = 1\n")?;
        let root = outside.path().join("sandbox");
        std::fs::create_dir(&root)?;

        let refused = refuse_manifest_above(&root)
            .err()
            .ok_or("the sandbox took the manifest above it")?;

        assert!(
            refused
                .to_string()
                .contains(&manifest.display().to_string()),
            "{refused}"
        );
        Ok(())
    }

    const DEVICE: &str = "01a216de-5800-7000-9ee1-77b5f4719b59";
    const OTHER_DEVICE: &str = "01a21715-4680-7000-8000-000000000002";
    const PROJECT: &str = "0193a7c4-0000-7000-8000-000000000001";

    /// An event file with one `entry.begun` event.
    fn begun(id: &str, batch: &str, entry: &str) -> String {
        format!(
            "version = 1\n\n[[event]]\nid = \"{id}\"\ntype = \"entry.begun\"\nv = 1\n\
             batch = \"{batch}\"\nentry = \"{entry}\"\nproject = \"{PROJECT}\"\n"
        )
    }

    /// A sandbox with an entry `…c4ad5e` on this device and `…2568b8` on
    /// another one.
    fn two_entries() -> Result<Sandbox, Box<dyn std::error::Error>> {
        let sandbox = Sandbox::new()?;
        sandbox.write(
            &format!("home/events/{DEVICE}/2026/2026-11.toml"),
            &begun(
                "01a216de-5800-7000-af18-a6bec2f07109",
                "01a216de-5800-7000-84ce-745b930f365c",
                "01a216de-5800-7000-ae6b-c7a4e9c4ad5e",
            ),
        )?;
        sandbox.write(
            &format!("other-home/events/{OTHER_DEVICE}/2026/2026-11.toml"),
            &begun(
                "01a21715-4680-7000-882f-64e308b79f7c",
                "01a21715-4680-7000-af46-8620765dda3a",
                "01a21715-4680-7000-9b84-3c0d772568b8",
            ),
        )?;
        Ok(sandbox)
    }

    #[test]
    fn ids_become_numbers_in_the_order_they_first_appear() -> TestResult {
        let sandbox = two_entries()?;
        let text = format!(
            "Began 68b8 \"Call\" at 10:00.\n\
             Ended ad5e \"Write offer\"; also 568b8 and c4ad5e.\n\
             {{\"id\": \"01a216de-5800-7000-ae6b-c7a4e9c4ad5e\", \"short\": \"ad5e\"}}\n\
             batch 01a216de-5800-7000-84ce-745b930f365c in home/events/{DEVICE}/2026\n"
        );

        assert_eq!(
            sandbox.redact_ids(&text)?,
            "Began [ID-1] \"Call\" at 10:00.\n\
             Ended [ID-2] \"Write offer\"; also [ID-1] and [ID-2].\n\
             {\"id\": \"[ID-2]\", \"short\": \"[ID-2]\"}\n\
             batch [ID-3] in home/events/[ID-4]/2026\n"
        );
        Ok(())
    }

    #[test]
    fn other_words_and_the_ids_test_wrote_stay() -> TestResult {
        let sandbox = two_entries()?;
        sandbox.write(
            "work/.pace.toml",
            &format!("[project]\nid = \"{PROJECT}\"\nkey = \"acme\"\n"),
        )?;
        let text = format!(
            "pace knows no entry deadbeef; 1800 seconds in 2026/2026-11.toml, \
             project {PROJECT}, Ad5e\n"
        );

        assert_eq!(sandbox.redact_ids(&text)?, text);
        Ok(())
    }

    #[test]
    fn short_id_that_fits_two_entries_is_refused() -> TestResult {
        let sandbox = two_entries()?;
        sandbox.write(
            &format!("home/events/{DEVICE}/2026/2026-10.toml"),
            &begun(
                "01a1c000-0000-7000-8000-000000000001",
                "01a1c000-0000-7000-8000-000000000002",
                "01a1c000-0000-7000-8000-00000000ad5e",
            ),
        )?;

        let error = sandbox.redact_ids("Began ad5e.").err().ok_or("an error")?;

        assert!(error.to_string().contains("`ad5e`"), "{error}");
        Ok(())
    }

    #[test]
    fn short_id_is_read_after_its_prefix() -> TestResult {
        let output = "success: true\n----- stdout -----\nBegan ad5e \"Write offer\" at 09:00.\n";

        assert_eq!(short_id_after(output, "Began ")?, "ad5e");
        assert!(short_id_after(output, "Ended ").is_err());
        Ok(())
    }
}
