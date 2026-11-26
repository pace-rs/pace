//! The latency guard of 3.1: answering a command takes less than 50 ms.
//!
//! It measures, on three years of one device's history:
//!
//! - `pace config show --origin`, which runs everything every command runs
//!   first: directories, the global file, the environment and the project
//!   discovery;
//! - `pace now`, which reads today's entries through the index;
//! - `pace begin`, which ends the running entry and appends to this
//!   month's file.
//!
//! The first `pace now` builds the index from every file; it is reported,
//! but not held to the budget, since it happens once per machine and after
//! large syncs.
//!
//! Run with `cargo bench --bench latency`; it fails if a median is over the
//! budget. In GitHub Actions, the results also go into the job summary.

use std::{
    collections::BTreeMap,
    error::Error,
    fmt::Write as _,
    path::Path,
    process::{Command, ExitCode, Stdio},
    time::{Duration, Instant},
};

use jiff::{ToSpan, ZonedDateTime, civil::Weekday};
use uuid::Uuid;

const BUDGET: Duration = Duration::from_millis(50);
const RUNS: usize = 50;

/// The moment the benchmark runs at, by `PACE_TEST_NOW`.
const NOW: &str = "2026-11-26T12:00:00+01:00[Europe/Berlin]";

fn main() -> Result<ExitCode, Box<dyn Error>> {
    let sandbox = tempfile::tempdir()?;
    let root = sandbox.path();
    prepare(root)?;
    let now: ZonedDateTime = NOW.parse()?;
    let events = history(root, &now)?;
    println!("three years of history: {events} events in monthly files");

    let pace = |args: &[&str], at: &ZonedDateTime| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_pace"));
        _ = command
            .args(args)
            .current_dir(root.join("work/acme/src"))
            .env("PACE_CONFIG", root.join("config/pace.global.toml"))
            .env("PACE_HOME", root.join("home"))
            .env("PACE_MACHINE_DIR", root.join("machine"))
            .env("PACE_TEST_NOW", at.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command
    };

    let start = Instant::now();
    let first = pace(&["now"], &now).stdout(Stdio::piped()).output()?;
    let elapsed = start.elapsed();
    let shown = String::from_utf8_lossy(&first.stdout);
    // The history is only worth measuring if pace reads it as intended.
    if !first.status.success() || !shown.contains("Running:") || !shown.contains("Task 3") {
        return Err(format!("the first `pace now` didn't show the running entry:\n{shown}").into());
    }
    println!("pace now, building the index: {elapsed:.1?} (not held to the budget)");

    let results = [
        measure("config show --origin", |_| {
            Ok(pace(&["config", "show", "--origin"], &now))
        })?,
        measure("now", |_| Ok(pace(&["now"], &now)))?,
        measure("begin", |run| {
            let minutes = i64::try_from(run)? + 1;
            Ok(pace(
                &["begin", "Benchmark"],
                &now.checked_add(minutes.minutes())?,
            ))
        })?,
    ];

    if let Some(summary) = std::env::var_os("GITHUB_STEP_SUMMARY") {
        let mut table = format!(
            "### Latency on {}\n\n{events} events of three years; budget {BUDGET:?} for the \
             median.\n\n| Command | Median | p95 |\n| --- | --- | --- |\n\
             | `pace now`, building the index | {elapsed:.1?} | |\n",
            std::env::consts::OS
        );
        for result in &results {
            writeln!(
                table,
                "| `pace {}` | {:.1?} | {:.1?} |",
                result.name, result.median, result.p95
            )?;
        }
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(summary)?;
        std::io::Write::write_all(&mut file, table.as_bytes())?;
    }

    if results.iter().any(|result| result.median > BUDGET) {
        eprintln!("a median is over the budget of {BUDGET:?}");
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}

/// How long a command took.
struct Measured {
    name: &'static str,
    median: Duration,
    p95: Duration,
}

/// Run the command `RUNS` times, and print its median and 95th percentile.
fn measure(
    name: &'static str,
    mut command: impl FnMut(usize) -> Result<Command, Box<dyn Error>>,
) -> Result<Measured, Box<dyn Error>> {
    let mut times = Vec::with_capacity(RUNS);
    for run in 0..RUNS {
        let mut command = command(run)?;
        let start = Instant::now();
        let status = command.status()?;
        times.push(start.elapsed());
        if !status.success() {
            return Err(format!("`pace {name}` failed").into());
        }
    }
    times.sort();
    let median = times[RUNS / 2];
    let p95 = times[RUNS * 95 / 100];
    println!(
        "pace {name}: median {median:.1?}, p95 {p95:.1?} over {RUNS} runs (budget {BUDGET:?})"
    );
    Ok(Measured { name, median, p95 })
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

/// Three years of this machine's history up to `now`, written as pace
/// writes it: on each weekday eight 50-minute entries from 08:00, every
/// second one held for five minutes, one caught thought a day; the entry
/// that began within the last hour still runs. Returns the number of
/// events.
fn history(root: &Path, now: &ZonedDateTime) -> Result<usize, Box<dyn Error>> {
    let device = Uuid::now_v7();
    std::fs::create_dir_all(root.join("machine"))?;
    std::fs::write(
        root.join("machine/device.toml"),
        format!("version = 1\nid = \"{device}\"\n"),
    )?;

    let zone = now.time_zone().clone();
    let first = now.date().checked_sub(3.years())?;
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    let mut count = 0;
    let mut day = first;
    while day <= now.date() {
        if matches!(day.weekday(), Weekday::Saturday | Weekday::Sunday) {
            day = day.tomorrow()?;
            continue;
        }
        let file = files
            .entry(format!(
                "{}/{}-{:02}.toml",
                day.year(),
                day.year(),
                day.month()
            ))
            .or_insert_with(|| "version = 1\n".to_owned());
        for slot in 0..8_i8 {
            let start = day.at(8 + slot, 0, 0, 0).to_zoned_datetime(zone.clone())?;
            if start >= *now {
                break;
            }
            let entry = Uuid::now_v7();
            let id = format!("entry = \"{entry}\"\n");
            event(
                file,
                "entry.begun",
                &start,
                &start,
                &format!(
                    "{id}project-key = \"acme-webshop\"\nnote = \"Task {slot}\"\ntags = [\"dev\"]\n"
                ),
            )?;
            count += 1;
            if slot % 2 == 1 {
                let held = start.checked_add(20.minutes())?;
                let resumed = start.checked_add(25.minutes())?;
                event(
                    file,
                    "entry.held",
                    &held,
                    &start,
                    &format!("{id}source = \"outer\"\nreason = \"phone call\"\n"),
                )?;
                event(file, "entry.resumed", &resumed, &start, &id)?;
                count += 2;
            }
            if slot == 3 {
                let caught = start.checked_add(30.minutes())?;
                event(
                    file,
                    "catch.noted",
                    &caught,
                    &start,
                    &format!("{id}text = \"call the tax office\"\n"),
                )?;
                count += 1;
            }
            let end = start.checked_add(50.minutes())?;
            if start.checked_add(1.hour())? < *now {
                event(file, "entry.ended", &end, &start, &id)?;
                count += 1;
            }
        }
        day = day.tomorrow()?;
    }

    let dir = root.join("home/events").join(device.to_string());
    for (name, text) in files {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, text)?;
    }
    std::fs::write(
        dir.join("device.toml"),
        format!(
            "version = 1\nname = \"bench\"\ncreated = \"{}\"\nsplit = \"month\"\n",
            first.at(8, 0, 0, 0).to_zoned_datetime(zone)?
        ),
    )?;
    Ok(count)
}

/// Append one event of `kind` to `file`, in pace's format.
fn event(
    file: &mut String,
    kind: &str,
    at: &ZonedDateTime,
    about: &ZonedDateTime,
    fields: &str,
) -> Result<(), Box<dyn Error>> {
    writeln!(
        file,
        "\n[[event]]\nid = \"{}\"\ntype = \"{kind}\"\nv = 1\nbatch = \"{}\"\nat = \"{at}\"",
        Uuid::now_v7(),
        Uuid::now_v7()
    )?;
    if about != at {
        writeln!(file, "about = \"{about}\"")?;
    }
    writeln!(file, "hlc = \"{}/0\"", at.timestamp())?;
    file.push_str(fields);
    Ok(())
}
