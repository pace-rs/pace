//! The latency guard of 3.1: answering a command takes less than 50 ms.
//!
//! Phase 1 measures `pace config show --origin`, which runs everything
//! every command runs first: directories, the global file, the environment
//! and the project discovery. Phase 2 adds `pace now` and `pace begin` on
//! three years of data.
//!
//! Run with `cargo bench --bench latency`; it fails if the median is over
//! the budget.

use std::{
    error::Error,
    fmt::Write as _,
    path::Path,
    process::{Command, ExitCode, Stdio},
    time::{Duration, Instant},
};

const BUDGET: Duration = Duration::from_millis(50);
const RUNS: usize = 50;

fn main() -> Result<ExitCode, Box<dyn Error>> {
    let sandbox = tempfile::tempdir()?;
    let root = sandbox.path();
    prepare(root)?;
    let pace = |args: &[&str]| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_pace"));
        _ = command
            .args(args)
            .current_dir(root.join("work/acme/src"))
            .env("PACE_CONFIG", root.join("config/pace.global.toml"))
            .env("PACE_HOME", root.join("home"))
            .env("PACE_MACHINE_DIR", root.join("machine"))
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command
    };

    // Warm up the file system cache, and check that the command works.
    if !pace(&["config", "show", "--origin"]).status()?.success() {
        return Err("`pace config show --origin` failed".into());
    }

    let mut times = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let start = Instant::now();
        let status = pace(&["config", "show", "--origin"]).status()?;
        times.push(start.elapsed());
        if !status.success() {
            return Err("`pace config show --origin` failed".into());
        }
    }
    times.sort();
    let median = times[RUNS / 2];
    let p95 = times[RUNS * 95 / 100];

    println!(
        "pace config show --origin: median {median:.1?}, p95 {p95:.1?} over {RUNS} runs \
         (budget {BUDGET:?})"
    );
    if median > BUDGET {
        eprintln!("the median is over the budget of {BUDGET:?}");
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}

fn prepare(root: &Path) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(root.join("config"))?;
    std::fs::create_dir_all(root.join("work/acme/src"))?;
    let mut mappings = String::from("[project-paths]\n");
    for client in 0..20 {
        writeln!(
            mappings,
            "'{}' = \"client-{client}\"",
            root.join(format!("clients/{client}")).display()
        )?;
    }
    std::fs::write(
        root.join("config/pace.global.toml"),
        format!(
            "version = 1\ntime-zone = \"Europe/Berlin\"\nweek-start = \"monday\"\n\n{mappings}"
        ),
    )?;
    std::fs::write(
        root.join("work/acme/.pace.toml"),
        "version = 1\n\n[project]\nkey = \"acme-webshop\"\nname = \"Webshop relaunch\"\n",
    )?;
    Ok(())
}
