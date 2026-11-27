//! The manifest links a directory to its project record in
//! `PACE_HOME/projects/<key>.toml`: entries go by the record's ID, and
//! entries recorded under the key before the record existed count for it.

use crate::support::{Sandbox, run};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The ID of the record of `acme-webshop`.
const RECORD_ID: &str = "0193a7c4-0000-7000-8000-000000000001";

/// Another ID, which no record has.
const OTHER_ID: &str = "0193a7c4-0000-7000-8000-0000000000ff";

/// Write the record of the project `acme-webshop` into the sandbox's
/// `PACE_HOME`.
fn write_webshop_record(sandbox: &Sandbox) -> std::io::Result<()> {
    sandbox.write(
        "home/projects/acme-webshop.toml",
        &format!(
            "version = 1\nid = \"{RECORD_ID}\"\nkey = \"acme-webshop\"\nname = \"Webshop relaunch\"\n"
        ),
    )
}

#[test]
fn manifest_key_with_record_makes_begin_record_record_id() -> TestResult {
    let sandbox = Sandbox::new()?;
    write_webshop_record(&sandbox)?;
    sandbox.write("work/.pace.toml", "[project]\nkey = \"acme-webshop\"\n")?;

    let output = sandbox.journey(&[
        ("09:00", &["begin", "Write offer"]),
        ("09:30", &["now", "--json"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn manifest_id_other_than_record_id_fails_naming_both_files() -> TestResult {
    let sandbox = Sandbox::new()?;
    write_webshop_record(&sandbox)?;
    sandbox.write(
        "work/.pace.toml",
        &format!("[project]\nid = \"{OTHER_ID}\"\nkey = \"acme-webshop\"\n"),
    )?;

    let output = sandbox.journey(&[("09:00", &["begin", "Write offer"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

/// The manifest of `acme-webshop` with the default tag `dev`.
const WITH_DEFAULT_TAGS: &str = "[project]\nkey = \"acme-webshop\"\ndefault-tags = [\"dev\"]\n";

#[test]
fn default_tags_join_tags_of_begun_entry() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/.pace.toml", WITH_DEFAULT_TAGS)?;

    let output = sandbox.journey(&[
        ("09:00", &["begin", "Write offer", "--tag", "offer"]),
        ("09:30", &["now"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

/// `begin --again` takes the tags of the logged entry, and adds no default
/// tags itself: so `now` shows the tags the log recorded.
#[test]
fn default_tags_join_tags_of_logged_entry() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/.pace.toml", WITH_DEFAULT_TAGS)?;

    let output = sandbox.journey(&[
        (
            "09:00",
            &["log", "Standup", "--from", "08:45", "--to", "09:00"],
        ),
        ("09:00", &["begin", "--again"]),
        ("09:30", &["now"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn entry_recorded_under_key_shows_under_project_once_its_record_exists() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/.pace.toml", "[project]\nkey = \"acme-webshop\"\n")?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "Write offer"]))?;
    write_webshop_record(&sandbox)?;

    let output = sandbox.journey(&[("09:30", &["now"]), ("09:30", &["now", "--json"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

/// The ID of the record of `gamma`.
#[cfg(unix)]
const GAMMA_ID: &str = "0193a7c4-0000-7000-8000-000000000002";

/// `adjust` names the project it moved the entry from and the one it moved
/// it to by their records, although the entry was recorded under the key;
/// `edit` names the record in the header of the file it opens.
#[cfg(unix)]
#[test]
fn adjust_and_edit_name_projects_by_their_records() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/.pace.toml", "[project]\nkey = \"acme-webshop\"\n")?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "Write offer"]))?;
    _ = run(sandbox.pace_at("10:00").arg("end"))?;
    write_webshop_record(&sandbox)?;
    sandbox.write(
        "home/projects/gamma.toml",
        &format!("version = 1\nid = \"{GAMMA_ID}\"\nkey = \"gamma\"\nname = \"Gamma relaunch\"\n"),
    )?;

    let adjusted = sandbox.journey(&[("10:05", &["adjust", "--last", "--project", "gamma"])])?;
    let opened = run(sandbox
        .pace_at("10:06")
        .env("VISUAL", "head -n 1")
        .args(["edit", "--last"]))?;

    let transcript = format!(
        "{adjusted}\n$ pace edit --last   # VISUAL=head -n 1\n{}",
        sandbox.redact_ids(&opened)?
    );
    sandbox
        .settings()
        .bind(|| insta::assert_snapshot!(transcript));
    Ok(())
}

#[test]
fn manifest_with_name_warns_with_notes_of_command() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write(
        "work/.pace.toml",
        "[project]\nkey = \"acme-webshop\"\nname = \"Webshop relaunch\"\n",
    )?;

    let output = sandbox.journey(&[("09:00", &["begin", "Write offer"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}
