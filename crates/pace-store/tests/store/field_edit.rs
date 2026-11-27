//! pace writes its TOML files field by field: comments and the order of a
//! file edited by hand survive.

use std::path::Path;

use jiff::civil::date;
use pace_store::{
    field_edit::{FieldEdit, FieldValue, write_field_edits},
    testing::scratch_dir,
};

use crate::TestResult;

const CLIENTS: &str = r#"version = 1

# Our oldest client.
[acme]
name = "ACME Inc." # as on the invoices
currency = "EUR"

[[acme.rates]]
from = 2026-01-01
hourly = 90.00

# Since spring.
[globex]
name = "Globex"
"#;

fn string(text: &str) -> FieldValue {
    FieldValue::String(text.to_owned())
}

fn read(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

#[test]
fn set_keeps_comments_and_order_of_other_keys() -> TestResult {
    let dir = scratch_dir()?;
    let clients = dir.path().join("clients.toml");
    std::fs::write(&clients, CLIENTS)?;

    write_field_edits(vec![
        FieldEdit::set(&clients, &["acme", "name"], string("ACME Corporation")),
        FieldEdit::set(&clients, &["acme", "street"], string("1 Road Runner Way")),
    ])?;

    insta::assert_snapshot!(read(&clients)?);
    Ok(())
}

#[test]
fn set_in_missing_table_adds_table_at_end_of_file() -> TestResult {
    let dir = scratch_dir()?;
    let clients = dir.path().join("clients.toml");
    std::fs::write(&clients, CLIENTS)?;

    write_field_edits(vec![
        FieldEdit::set(&clients, &["initech", "name"], string("Initech")),
        FieldEdit::set(
            &clients,
            &["initech", "tags"],
            FieldValue::Strings(vec!["software".to_owned(), "staplers".to_owned()]),
        ),
        FieldEdit::set(&clients, &["initech", "active"], FieldValue::Boolean(true)),
        FieldEdit::set(&clients, &["initech", "seats"], FieldValue::Integer(12)),
        FieldEdit::set(
            &clients,
            &["initech", "contact"],
            FieldValue::InlineTable(vec![
                ("name".to_owned(), string("Bill")),
                ("since".to_owned(), FieldValue::Date(date(2026, 7, 1))),
            ]),
        ),
    ])?;

    insta::assert_snapshot!(read(&clients)?);
    Ok(())
}

#[test]
fn remove_takes_out_key_and_keeps_the_rest() -> TestResult {
    let dir = scratch_dir()?;
    let clients = dir.path().join("clients.toml");
    std::fs::write(&clients, CLIENTS)?;

    write_field_edits(vec![FieldEdit::remove(&clients, &["acme", "currency"])])?;

    insta::assert_snapshot!(read(&clients)?);
    Ok(())
}

#[test]
fn append_table_goes_after_last_table_of_its_array() -> TestResult {
    let dir = scratch_dir()?;
    let clients = dir.path().join("clients.toml");
    std::fs::write(&clients, CLIENTS)?;

    write_field_edits(vec![FieldEdit::append_table(
        &clients,
        &["acme", "rates"],
        vec![
            ("from".to_owned(), FieldValue::Date(date(2026, 7, 1))),
            ("hourly".to_owned(), FieldValue::Decimal("95.50".to_owned())),
        ],
    )])?;

    insta::assert_snapshot!(read(&clients)?);
    Ok(())
}

#[test]
fn new_file_starts_with_version_in_new_directory() -> TestResult {
    let dir = scratch_dir()?;
    let project = dir.path().join("projects/website.toml");

    write_field_edits(vec![
        FieldEdit::set(&project, &["name"], string("Website relaunch")),
        FieldEdit::append_table(
            &project,
            &["rates"],
            vec![("hourly".to_owned(), FieldValue::Decimal("1200".to_owned()))],
        ),
    ])?;

    insta::assert_snapshot!(read(&project)?);
    Ok(())
}

#[test]
fn decimal_keeps_text_it_was_given() -> TestResult {
    let dir = scratch_dir()?;
    let roles = dir.path().join("roles.toml");

    write_field_edits(vec![FieldEdit::set(
        &roles,
        &["developer", "hourly"],
        FieldValue::Decimal("95.50".to_owned()),
    )])?;

    let text = read(&roles)?;
    assert!(text.contains("hourly = 95.50\n"), "{text}");
    Ok(())
}

#[test]
fn one_call_writes_two_files() -> TestResult {
    let dir = scratch_dir()?;
    let global = dir.path().join("pace.global.toml");
    let manifest = dir.path().join("project/.pace.toml");
    std::fs::write(
        &global,
        "version = 1\n\n# Where I live.\ntime-zone = \"Europe/Berlin\"\n",
    )?;

    write_field_edits(vec![
        FieldEdit::set(&global, &["time-zone"], string("Europe/Lisbon")),
        FieldEdit::set(&manifest, &["project"], string("website")),
        FieldEdit::set(&global, &["split"], string("month")),
    ])?;

    insta::assert_snapshot!(format!(
        "pace.global.toml:\n{}\n.pace.toml:\n{}",
        read(&global)?,
        read(&manifest)?
    ));
    Ok(())
}

#[test]
fn unchanged_file_is_not_written() -> TestResult {
    let dir = scratch_dir()?;
    let roles = dir.path().join("roles.toml");

    write_field_edits(vec![FieldEdit::remove(&roles, &["developer"])])?;

    assert!(!roles.exists(), "a remove of nothing writes no file");
    Ok(())
}

#[test]
fn value_on_path_of_table_is_refused() -> TestResult {
    let dir = scratch_dir()?;
    let clients = dir.path().join("clients.toml");
    std::fs::write(&clients, CLIENTS)?;

    let written = write_field_edits(vec![FieldEdit::set(
        &clients,
        &["acme", "name", "first"],
        string("ACME"),
    )]);

    let error = written.err().ok_or("acme.name is no table")?;
    assert_eq!(
        error.to_string(),
        format!(
            "{} holds acme.name as a value, not as a table",
            clients.display()
        )
    );
    assert_eq!(read(&clients)?, CLIENTS);
    Ok(())
}

#[test]
fn file_that_is_no_toml_is_refused_and_left_alone() -> TestResult {
    let dir = scratch_dir()?;
    let roles = dir.path().join("roles.toml");
    std::fs::write(&roles, "version = 1\n[developer\n")?;

    let written = write_field_edits(vec![FieldEdit::set(
        &roles,
        &["developer", "name"],
        string("Developer"),
    )]);

    let error = written.err().ok_or("the file is no valid TOML")?;
    assert_eq!(
        error.to_string(),
        format!("the file {} is not valid TOML", roles.display())
    );
    assert_eq!(read(&roles)?, "version = 1\n[developer\n");
    Ok(())
}

#[cfg(unix)]
#[test]
fn failed_write_leaves_every_file_whole() -> TestResult {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = scratch_dir()?;
    let global = dir.path().join("pace.global.toml");
    let locked = dir.path().join("locked");
    let clients = locked.join("clients.toml");
    std::fs::write(&global, "version = 1\ntime-zone = \"Europe/Berlin\"\n")?;
    std::fs::create_dir(&locked)?;
    std::fs::write(&clients, CLIENTS)?;
    let mode = |mode| std::fs::Permissions::from_mode(mode);
    // A read-only directory takes no temporary file.
    std::fs::set_permissions(&locked, mode(0o555))?;

    let written = write_field_edits(vec![
        FieldEdit::set(&global, &["time-zone"], string("Europe/Lisbon")),
        FieldEdit::set(&clients, &["acme", "name"], string("ACME Corporation")),
    ]);
    std::fs::set_permissions(&locked, mode(0o755))?;

    let error = written.err().ok_or("the read-only directory fails")?;
    assert_eq!(
        error.to_string(),
        format!("cannot create a temporary file in {}", locked.display())
    );
    assert_eq!(
        read(&global)?,
        "version = 1\ntime-zone = \"Europe/Berlin\"\n"
    );
    assert_eq!(read(&clients)?, CLIENTS);
    Ok(())
}
