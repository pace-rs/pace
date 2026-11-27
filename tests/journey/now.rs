//! `pace now` and bare `pace`: what is running, and choosing entries by
//! short ID.

use crate::support::{Sandbox, begin_apart, run};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The short ID that `pace now` shows first.
fn first_short_id(sandbox: &Sandbox, time: &str) -> Result<String, Box<dyn std::error::Error>> {
    let output = sandbox.pace_at(time).args(["now", "--json"]).output()?;
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    json["open"][0]["short"]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| "no open entry".into())
}

#[test]
fn now_with_nothing_running_says_so() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[("09:00", &["now"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn now_shows_the_open_entry_and_its_breaks() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/.pace.toml", "[project]\nkey = \"acme-webshop\"\n")?;

    let output = sandbox.journey(&[
        ("09:00", &["begin", "Write offer", "--tag", "offer"]),
        ("10:00", &["now"]),
        ("10:15", &["hold", "--outer", "--reason", "phone call"]),
        ("10:30", &["now"]),
        ("10:45", &["resume"]),
        ("12:00", &["pause", "--lunch"]),
        ("12:10", &["now"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn now_speaks_json_for_scripts() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.write("work/.pace.toml", "[project]\nkey = \"acme-webshop\"\n")?;

    let output = sandbox.journey(&[
        ("09:00", &["begin", "Write offer", "--tag", "offer"]),
        ("10:15", &["hold", "--outer", "--reason", "phone call"]),
        ("10:30", &["now", "--json"]),
    ])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn now_json_names_kind_and_file_of_path_mapping() -> TestResult {
    let sandbox = Sandbox::new()?;
    sandbox.mkdir("work/repo")?;
    sandbox.write_config(&format!(
        "[project-paths]\n'{}' = \"client-repo\"\n",
        sandbox.path("work/repo").display()
    ))?;

    let output = sandbox.pace_in("repo").args(["now", "--json"]).output()?;
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;

    let project = &json["project"];
    assert_eq!(project["key"], "client-repo");
    assert_eq!(project["kind"], "mapping");
    assert_eq!(
        project["path"],
        sandbox
            .path("config/pace.global.toml")
            .display()
            .to_string()
    );
    Ok(())
}

#[test]
fn now_json_leaves_out_path_for_flag_and_variable() -> TestResult {
    let sandbox = Sandbox::new()?;

    let flag = sandbox
        .pace()
        .args(["now", "--json", "--project", "acme"])
        .output()?;
    let variable = sandbox
        .pace()
        .env("PACE_PROJECT", "acme")
        .args(["now", "--json"])
        .output()?;

    for (output, kind) in [(flag, "flag"), (variable, "variable")] {
        let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        let project = &json["project"];
        assert_eq!(project["key"], "acme");
        assert_eq!(project["kind"], kind);
        assert!(
            project.get("path").is_none(),
            "{kind} names a path: {project}"
        );
    }
    Ok(())
}

#[test]
fn pace_without_arguments_shows_now_and_how_to_get_help() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.journey(&[("09:00", &["begin", "Write offer"]), ("09:30", &[])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn entries_open_on_two_devices_are_both_shown() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "on this machine"]))?;
    begin_apart(&sandbox, "09:30", "on the other machine")?;

    let output = sandbox.journey(&[("10:00", &["now"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn end_takes_short_id() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "on this machine"]))?;
    begin_apart(&sandbox, "09:30", "on the other machine")?;
    let first = first_short_id(&sandbox, "10:00")?;

    let output =
        sandbox.journey(&[("10:00", &["end", "deadbeef"]), ("10:00", &["end", &first])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn two_machines_first_used_at_the_same_moment_keep_their_entries() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "on this machine"]))?;
    begin_apart(&sandbox, "09:00", "on the other machine")?;

    let devices = std::fs::read_dir(sandbox.path("home/events"))?.count();
    let output = sandbox.pace_at("10:00").args(["now", "--json"]).output()?;
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let open = json["open"].as_array().ok_or("a list of open entries")?;
    let mut notes: Vec<&str> = open
        .iter()
        .filter_map(|entry| entry["note"].as_str())
        .collect();
    notes.sort_unstable();

    assert_eq!(devices, 2, "each machine has its own device directory");
    assert_eq!(notes, ["on the other machine", "on this machine"]);
    assert_ne!(open[0]["id"], open[1]["id"]);
    Ok(())
}

#[test]
fn now_names_overlap_and_how_to_change_it() -> TestResult {
    let sandbox = Sandbox::new()?;
    _ = run(sandbox.pace_at("09:00").args(["begin", "on this machine"]))?;
    begin_apart(&sandbox, "09:30", "on the other machine")?;
    let first = first_short_id(&sandbox, "10:00")?;

    let output = sandbox.journey(&[("10:00", &["end", &first]), ("10:01", &["now"])])?;
    sandbox.settings().bind(|| insta::assert_snapshot!(output));
    Ok(())
}

#[test]
fn now_on_first_run_writes_nothing() -> TestResult {
    let sandbox = Sandbox::new()?;

    _ = run(sandbox.pace_at("09:00").arg("now"))?;

    assert!(
        !sandbox.path("machine").exists(),
        "no device ID for a query"
    );
    assert!(!sandbox.path("home").exists());
    Ok(())
}

#[test]
fn now_into_closed_pipe_ends_quietly() -> TestResult {
    let sandbox = Sandbox::new()?;
    let (reader, writer) = std::io::pipe()?;
    drop(reader);

    let output = sandbox
        .pace_at("09:00")
        .arg("now")
        .stdout(writer)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(String::from_utf8(output.stderr)?, "");
    Ok(())
}
