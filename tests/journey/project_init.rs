//! `pace project init` makes the working directory a project: it writes
//! the record with a new ID and the manifest that links the directory to
//! it, asking only what it cannot guess.

use std::fmt::Write as _;

use crate::support::{Sandbox, run};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The transcript of `steps` in `work/<dir>`, then each of `files` as pace
/// left it, the IDs of the whole redacted together.
fn journey_and_files(
    sandbox: &Sandbox,
    dir: &str,
    steps: &[(&str, &[&str])],
    files: &[&str],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = sandbox.transcript_in(dir, steps)?;
    for file in files {
        let text = std::fs::read_to_string(sandbox.path(file))?;
        write!(output, "--- {file} ---\n{text}")?;
    }
    sandbox.redact_ids(&output)
}

/// The record of `webshop`, which a second checkout finds.
const WEBSHOP_RECORD: &str = "version = 1\nid = \"0193a7c4-0000-7000-8000-000000000001\"\n\
                              key = \"webshop\"\nname = \"Webshop relaunch\"\n";

#[test]
fn project_init_without_terminal_writes_record_and_manifest_then_begin_records_id() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.mkdir("work/webshop")?;

    let output = journey_and_files(
        &sandbox,
        "webshop",
        &[
            ("09:00", &["project", "init"]),
            ("09:01", &["begin", "Checkout"]),
            ("09:30", &["now", "--json"]),
        ],
        &["home/projects/webshop.toml", "work/webshop/.pace.toml"],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn project_init_in_second_checkout_writes_only_manifest() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("home/projects/webshop.toml", WEBSHOP_RECORD)?;
    sandbox.mkdir("work/laptop/webshop")?;

    let output = journey_and_files(
        &sandbox,
        "laptop/webshop",
        &[("09:00", &["project", "init"])],
        &[
            "home/projects/webshop.toml",
            "work/laptop/webshop/.pace.toml",
        ],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn project_init_after_second_checkout_follows_its_hint_to_another_project() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("home/projects/webshop.toml", WEBSHOP_RECORD)?;
    sandbox.mkdir("work/laptop/webshop")?;
    let first = sandbox
        .pace_in("laptop/webshop")
        .args(["project", "init"])
        .output()?;
    assert!(first.status.success());

    std::fs::remove_file(sandbox.path("work/laptop/webshop/.pace.toml"))?;
    let second = sandbox
        .pace_in("laptop/webshop")
        .args(["project", "init", "--key", "shop"])
        .output()?;

    assert!(second.status.success(), "{second:?}");
    let manifest = std::fs::read_to_string(sandbox.path("work/laptop/webshop/.pace.toml"))?;
    assert!(manifest.contains("key = \"shop\""), "{manifest}");
    assert!(sandbox.path("home/projects/shop.toml").exists());
    Ok(())
}

#[test]
fn project_init_gives_phase_two_manifest_its_id_and_takes_its_name() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write(
        "work/webshop/.pace.toml",
        "version = 1\n\n[project]\n# the shop of ACME\nkey = \"webshop\"\nname = \"Webshop relaunch\"\n\
         default-tags = [\"dev\"]\n",
    )?;

    let output = journey_and_files(
        &sandbox,
        "webshop",
        &[("09:00", &["project", "init"]), ("09:01", &["now"])],
        &["home/projects/webshop.toml", "work/webshop/.pace.toml"],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn project_init_visible_writes_pace_toml_with_flags_answering_questions() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.mkdir("work/webshop")?;

    let output = journey_and_files(
        &sandbox,
        "webshop",
        &[
            ("08:59", &["role", "add", "work", "--name", "Work"]),
            (
                "09:00",
                &[
                    "project",
                    "init",
                    "--visible",
                    "--key",
                    "acme-webshop",
                    "--name",
                    "Webshop relaunch",
                    "--role",
                    "work",
                    "--billable",
                ],
            ),
        ],
        &["home/projects/acme-webshop.toml", "work/webshop/pace.toml"],
    )?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn second_project_init_has_nothing_to_do() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.mkdir("work/webshop")?;
    _ = run(sandbox.pace_in("webshop").args(["project", "init"]))?;

    let output = sandbox.journey_in("webshop", &[("09:00", &["project", "init"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn project_init_refuses_what_it_cannot_do_and_names_flag() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("home/projects/webshop.toml", WEBSHOP_RECORD)?;
    sandbox.mkdir("work/webshop")?;
    sandbox.mkdir("work/___")?;

    let mut output = sandbox.journey_in("___", &[("09:00", &["project", "init"])])?;
    output.push_str(&sandbox.journey_in(
        "webshop",
        &[("09:00", &["project", "init", "--name", "Shop"])],
    )?);
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn project_init_refuses_manifest_with_id_of_other_project() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("home/projects/webshop.toml", WEBSHOP_RECORD)?;
    sandbox.write(
        "work/shop/.pace.toml",
        "version = 1\n\n[project]\nid = \"0193a7c4-0000-7000-8000-000000000001\"\nkey = \"shop\"\n",
    )?;

    let output = sandbox.journey_in("shop", &[("09:00", &["project", "init"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    assert!(!sandbox.path("home/projects/shop.toml").exists());
    Ok(())
}
