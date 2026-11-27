//! pace writes its TOML files field by field: comments and the order of a
//! file edited by hand survive.

use std::path::Path;

use jiff::civil::date;
use pace_error::ErrorKind;
use pace_store::{
    field_edit::{FieldEdit, FieldValue, apply_field_edits, table_text, write_field_edits},
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
fn remove_below_missing_table_or_value_changes_nothing() -> TestResult {
    let dir = scratch_dir()?;
    let clients = dir.path().join("clients.toml");
    let roles = dir.path().join("roles.toml");
    std::fs::write(&clients, CLIENTS)?;

    write_field_edits(vec![
        FieldEdit::remove(&clients, &["initech", "contact", "phone"]),
        FieldEdit::remove(&clients, &["acme", "name", "first"]),
        FieldEdit::remove(&roles, &["developer", "rates", "hourly"]),
    ])?;

    assert_eq!(read(&clients)?, CLIENTS);
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

/// The table `acme` of [`CLIENTS`], as [`table_text`] shows it alone.
const ACME: &str = r#"# Our oldest client.
[acme]
name = "ACME Inc." # as on the invoices
currency = "EUR"

[[acme.rates]]
from = 2026-01-01
hourly = 90.00
"#;

#[test]
fn table_text_shows_table_alone_with_comment_above_it() -> TestResult {
    let text = table_text(Path::new("clients.toml"), CLIENTS, &["acme"])?;

    assert_eq!(text.as_deref(), Some(ACME));
    assert_eq!(
        table_text(Path::new("clients.toml"), CLIENTS, &["initech"])?,
        None
    );
    Ok(())
}

#[test]
fn replace_table_with_its_own_text_changes_nothing() -> TestResult {
    let edit = FieldEdit::replace_table("clients.toml", &["acme"], ACME);

    let text = apply_field_edits(Path::new("clients.toml"), CLIENTS, &[edit])?;

    assert_eq!(text, CLIENTS);
    Ok(())
}

#[test]
fn replace_table_keeps_its_place_and_tables_of_others() -> TestResult {
    let edited = format!(
        "{}\n# From July.\n[[acme.rates]]\nfrom = 2026-07-01\nhourly = 95.50\n",
        ACME.replace("ACME Inc.", "ACME GmbH")
    );
    let edit = FieldEdit::replace_table("clients.toml", &["acme"], &edited);

    let text = apply_field_edits(Path::new("clients.toml"), CLIENTS, &[edit])?;

    assert_eq!(
        text,
        r#"version = 1

# Our oldest client.
[acme]
name = "ACME GmbH" # as on the invoices
currency = "EUR"

[[acme.rates]]
from = 2026-01-01
hourly = 90.00

# From July.
[[acme.rates]]
from = 2026-07-01
hourly = 95.50

# Since spring.
[globex]
name = "Globex"
"#
    );
    Ok(())
}

#[test]
fn replace_table_gathers_its_tables_where_the_first_stood() -> TestResult {
    let scattered = "version = 1\n\n[acme]\nname = \"ACME\"\n\n[globex]\nname = \"Globex\"\n\n[[acme.rates]]\nfrom = 2026-01-01\nhourly = 90\n";
    let edited = "[acme]\nname = \"ACME\"\n\n[[acme.rates]]\nfrom = 2026-01-01\nhourly = 95\n";
    let edit = FieldEdit::replace_table("clients.toml", &["acme"], edited);

    let text = apply_field_edits(Path::new("clients.toml"), scattered, &[edit])?;

    assert_eq!(
        text,
        "version = 1\n\n[acme]\nname = \"ACME\"\n\n[[acme.rates]]\nfrom = 2026-01-01\nhourly = 95\n\n[globex]\nname = \"Globex\"\n"
    );
    Ok(())
}

#[test]
fn replace_of_missing_table_adds_it_at_end_of_file() -> TestResult {
    let edit = FieldEdit::replace_table(
        "clients.toml",
        &["initech"],
        "[initech]\nname = \"Initech\"\n",
    );

    let text = apply_field_edits(Path::new("clients.toml"), CLIENTS, &[edit])?;

    assert_eq!(text, format!("{CLIENTS}\n[initech]\nname = \"Initech\"\n"));
    Ok(())
}

#[test]
fn replace_table_refuses_text_with_other_keys() {
    let edited = format!("version = 1\n{ACME}");
    let edit = FieldEdit::replace_table("clients.toml", &["acme"], &edited);

    let error = apply_field_edits(Path::new("clients.toml"), CLIENTS, &[edit]).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert_eq!(
        error.to_string(),
        "the text for the table `acme` holds `version`, outside of the table"
    );
    assert_eq!(
        error.hint(),
        Some("keep only `[acme]` and the tables below it, such as `[acme.…]`")
    );
}

#[test]
fn replace_table_refuses_text_without_table() {
    let edit = FieldEdit::replace_table("clients.toml", &["acme"], "# gone\n");

    let error = apply_field_edits(Path::new("clients.toml"), CLIENTS, &[edit]).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert_eq!(
        error.to_string(),
        "the text for the table `acme` holds no table `acme`"
    );
    assert_eq!(error.hint(), Some("keep the header `[acme]`"));
}

#[test]
fn replace_table_refuses_text_that_is_no_toml() {
    let edited = ACME.replace("currency = \"EUR\"", "currency = EUR");
    let edit = FieldEdit::replace_table("clients.toml", &["acme"], &edited);

    let error = apply_field_edits(Path::new("clients.toml"), CLIENTS, &[edit]).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert_eq!(
        error.to_string(),
        "the text for the table `acme` is not valid TOML"
    );
    assert_eq!(error.hint(), Some("fix the TOML at the marked spot"));
}
