//! pace reads the roles, the clients and the project records of
//! `PACE_HOME` into the `Catalog`; each error points at its file and line.

use std::path::Path;

use jiff::{SignedDuration, civil::date};
use pace_core::{ProjectKey, RoleKind, RoundingMode, RoundingPer};
use pace_error::PaceError;
use pace_store::{
    catalog::{read_catalog, read_catalog_with, write_catalog_file},
    testing::scratch_dir,
};

use crate::TestResult;

const ROLE_ID: &str = "0193a7c4-0000-7000-8000-000000000001";
const PROJECT_ID: &str = "0193a7c4-0000-7000-8000-000000000002";

const ROLES: &str = r#"version = 1

[work]
id = "0193a7c4-0000-7000-8000-000000000001" # events refer to the ID
kind = "area"
name = "Work"
purpose = "Earn a living with good work"
weekly-target = "30h"
"#;

const CLIENTS: &str = r#"version = 1

[acme]
name = "ACME GmbH"
currency = "EUR"
rounding = { per = "entry", step = "15m", mode = "up" }

[[acme.rates]]
hourly = 95
from = 2025-01-01

[[acme.rates]]
hourly = 105
from = 2026-07-01

[[acme.projects.acme-webshop.rates]] # optional override for one project
hourly = 120
from = 2026-09-01
"#;

const PROJECT: &str = r#"version = 1
id = "0193a7c4-0000-7000-8000-000000000002"
key = "acme-webshop"
name = "Webshop relaunch"
client = "acme"
role = "work"
billable = true
budget = "120h"
starts-on = 2026-09-01
done-when = "the shop takes orders"
"#;

/// A `PACE_HOME` with the given files, by their path inside it.
fn home(files: &[(&str, &str)]) -> std::io::Result<tempfile::TempDir> {
    let dir = scratch_dir()?;
    for (name, text) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, text)?;
    }
    Ok(dir)
}

/// The three files of the plan.
fn plan_home() -> std::io::Result<tempfile::TempDir> {
    home(&[
        ("roles.toml", ROLES),
        ("clients.toml", CLIENTS),
        ("projects/acme-webshop.toml", PROJECT),
    ])
}

/// The error of reading the catalog in `home`.
fn read_error(home: &Path) -> Result<Box<PaceError>, &'static str> {
    read_catalog(home).err().ok_or("the catalog must not read")
}

/// The file and the line, counted from 1, that an error's snippet points
/// at.
fn blamed(error: &PaceError) -> Option<(String, usize)> {
    let label = miette::Diagnostic::labels(error)?.next()?;
    let source = miette::Diagnostic::source_code(error)?;
    let contents = source.read_span(label.inner(), 0, 0).ok()?;
    let name = contents.name()?.to_owned();
    let file = Path::new(&name).file_name()?.to_string_lossy().into_owned();
    Some((file, contents.line() + 1))
}

#[test]
fn plan_examples_read_into_catalog() -> TestResult {
    let dir = plan_home()?;

    let catalog = read_catalog(dir.path())?;

    let work = catalog.role(&"work".parse()?).ok_or("the role is there")?;
    assert_eq!(work.id, ROLE_ID.parse()?);
    assert_eq!(work.kind, RoleKind::Area);
    assert_eq!(work.name, "Work");
    assert_eq!(
        work.purpose.as_deref(),
        Some("Earn a living with good work")
    );
    assert_eq!(work.weekly_target, Some(SignedDuration::from_hours(30)));
    assert_eq!(work.archived, None);

    let acme = catalog
        .client(&"acme".parse()?)
        .ok_or("the client is there")?;
    assert_eq!(acme.name, "ACME GmbH");
    assert_eq!(acme.currency.code(), "EUR");
    assert_eq!(acme.rounding.per(), Some(RoundingPer::Entry));
    assert_eq!(acme.rounding.step(), Some(SignedDuration::from_mins(15)));
    assert_eq!(acme.rounding.mode(), Some(RoundingMode::Up));
    let rates: Vec<(i64, _)> = acme
        .rates
        .iter()
        .map(|rate| (rate.hourly.minor(), rate.from))
        .collect();
    assert_eq!(
        rates,
        [(9500, date(2025, 1, 1)), (10_500, date(2026, 7, 1))]
    );

    let key: ProjectKey = "acme-webshop".parse()?;
    let webshop = catalog.project(&key).ok_or("the project is there")?;
    assert_eq!(webshop.id, PROJECT_ID.parse()?);
    assert_eq!(webshop.name, "Webshop relaunch");
    assert_eq!(webshop.client, Some("acme".parse()?));
    assert_eq!(webshop.role, Some("work".parse()?));
    assert!(webshop.billable);
    assert_eq!(webshop.budget, Some(SignedDuration::from_hours(120)));
    assert_eq!(webshop.starts_on, Some(date(2026, 9, 1)));
    assert_eq!(webshop.ends_on, None);
    assert_eq!(webshop.done_when.as_deref(), Some("the shop takes orders"));
    assert!(!webshop.allow_overlap);

    let before = catalog
        .rate_on(webshop, date(2026, 8, 31))
        .ok_or("a rate")?;
    let after = catalog.rate_on(webshop, date(2026, 9, 1)).ok_or("a rate")?;
    assert_eq!(before.hourly.minor(), 10_500);
    assert_eq!(after.hourly.minor(), 12_000);
    Ok(())
}

#[test]
fn hourly_95_50_reads_as_9550_minor_units() -> TestResult {
    let clients = "version = 1\n[acme]\nname = \"ACME\"\ncurrency = \"EUR\"\n\
                   [[acme.rates]]\nhourly = 95.50\nfrom = 2026-01-01\n";
    let dir = home(&[("clients.toml", clients)])?;

    let catalog = read_catalog(dir.path())?;

    let acme = catalog.client(&"acme".parse()?).ok_or("the client")?;
    let rate = acme.rates.iter().next().ok_or("one rate")?;
    assert_eq!(rate.hourly.minor(), 9550);
    Ok(())
}

#[test]
fn missing_files_give_empty_catalog() -> TestResult {
    let dir = scratch_dir()?;

    let catalog = read_catalog(dir.path())?;

    assert_eq!(catalog.roles().count(), 0);
    assert_eq!(catalog.clients().count(), 0);
    assert_eq!(catalog.projects().count(), 0);
    Ok(())
}

#[test]
fn archived_role_reads_its_day_and_reason() -> TestResult {
    let roles = ROLES.replace(
        "weekly-target = \"30h\"",
        "archived = { on = 2027-03-01, reason = \"ended\" }",
    );
    let dir = home(&[("roles.toml", &roles)])?;

    let catalog = read_catalog(dir.path())?;

    let work = catalog.role(&"work".parse()?).ok_or("the role")?;
    let archived = work.archived.ok_or("the role is archived")?;
    assert_eq!(archived.on, date(2027, 3, 1));
    assert_eq!(archived.reason.as_str(), "ended");
    Ok(())
}

#[test]
fn unknown_key_names_closest_and_its_line() -> TestResult {
    let roles = ROLES.replace("purpose =", "purpse =");
    let dir = home(&[("roles.toml", &roles)])?;

    let error = read_error(dir.path())?;

    assert_eq!(error.to_string(), "`purpse` is not a key pace knows");
    assert_eq!(error.hint(), Some("did you mean `purpose`?"));
    assert_eq!(blamed(&error), Some(("roles.toml".to_owned(), 7)));
    Ok(())
}

#[test]
fn remotes_are_unknown_until_phase_5() -> TestResult {
    let project = format!("{PROJECT}\n[remotes]\nkimai = {{ project = 12 }}\n");
    let dir = home(&[("projects/acme-webshop.toml", &project)])?;

    let error = read_error(dir.path())?;

    assert_eq!(error.to_string(), "`remotes` is not a key pace knows");
    Ok(())
}

#[test]
fn project_with_unknown_client_names_file_line_and_closest_client() -> TestResult {
    let dir = plan_home()?;
    let project = PROJECT.replace("client = \"acme\"", "client = \"acmee\"");
    std::fs::write(dir.path().join("projects/acme-webshop.toml"), project)?;

    let error = read_error(dir.path())?;

    assert_eq!(
        error.to_string(),
        "the project `acme-webshop` names the client `acmee`, which pace doesn't know"
    );
    assert_eq!(error.hint(), Some("did you mean `acme`?"));
    assert_eq!(blamed(&error), Some(("acme-webshop.toml".to_owned(), 5)));
    Ok(())
}

#[test]
fn project_with_unknown_role_names_its_line() -> TestResult {
    let dir = plan_home()?;
    let project = PROJECT.replace("role = \"work\"", "role = \"wrok\"");
    std::fs::write(dir.path().join("projects/acme-webshop.toml"), project)?;

    let error = read_error(dir.path())?;

    assert!(error.is_code("pace::core::unknown-role"), "{error:?}");
    assert_eq!(blamed(&error), Some(("acme-webshop.toml".to_owned(), 6)));
    Ok(())
}

#[test]
fn rates_for_unknown_project_point_at_clients_file() -> TestResult {
    let dir = home(&[("clients.toml", CLIENTS)])?;

    let error = read_error(dir.path())?;

    assert!(error.is_code("pace::core::unknown-project"), "{error:?}");
    assert_eq!(blamed(&error), Some(("clients.toml".to_owned(), 16)));
    Ok(())
}

#[test]
fn project_file_whose_key_differs_from_its_name_names_both() -> TestResult {
    let dir = home(&[("projects/webshop.toml", PROJECT)])?;

    let error = read_error(dir.path())?;

    assert_eq!(
        error.to_string(),
        "the project file webshop.toml holds the key `acme-webshop`"
    );
    assert_eq!(
        error.hint(),
        Some("rename the file to acme-webshop.toml, or write `key = \"webshop\"`")
    );
    assert_eq!(blamed(&error), Some(("webshop.toml".to_owned(), 3)));
    Ok(())
}

#[test]
fn version_2_says_newer_pace_wrote_it() -> TestResult {
    let clients = CLIENTS.replace("version = 1", "version = 2");
    let dir = home(&[("clients.toml", &clients)])?;

    let error = read_error(dir.path())?;

    assert_eq!(
        error.to_string(),
        "this pace reads version 1 of the clients file, not 2"
    );
    assert_eq!(
        error.hint(),
        Some("a newer pace wrote this file; update pace")
    );
    Ok(())
}

#[test]
fn value_that_does_not_fit_names_what_fits() -> TestResult {
    let roles = ROLES.replace("kind = \"area\"", "kind = \"aera\"");
    let dir = home(&[("roles.toml", &roles)])?;

    let error = read_error(dir.path())?;

    assert_eq!(error.to_string(), "`aera` is not a role kind");
    assert_eq!(
        error.hint(),
        Some("a role kind is one of `area`, `area-project`, `meta`")
    );
    assert_eq!(blamed(&error), Some(("roles.toml".to_owned(), 5)));
    Ok(())
}

#[test]
fn rate_as_text_says_to_write_number() -> TestResult {
    let clients = CLIENTS.replace("hourly = 95\n", "hourly = \"95\"\n");
    let dir = home(&[("clients.toml", &clients)])?;

    let error = read_error(dir.path())?;

    assert_eq!(error.to_string(), "`hourly` takes an amount such as 95.50");
    assert_eq!(blamed(&error), Some(("clients.toml".to_owned(), 9)));
    Ok(())
}

#[test]
fn rate_with_more_decimals_than_currency_points_at_it() -> TestResult {
    let clients = CLIENTS.replace("hourly = 95\n", "hourly = 95.505\n");
    let dir = home(&[("clients.toml", &clients)])?;

    let error = read_error(dir.path())?;

    assert!(error.is_code("pace::clients::invalid-value"), "{error:?}");
    assert_eq!(blamed(&error), Some(("clients.toml".to_owned(), 9)));
    Ok(())
}

#[test]
fn role_without_name_says_what_to_add() -> TestResult {
    let roles = ROLES.replace("name = \"Work\"\n", "");
    let dir = home(&[("roles.toml", &roles)])?;

    let error = read_error(dir.path())?;

    assert_eq!(error.to_string(), "the role `work` has no `name`");
    assert_eq!(blamed(&error), Some(("roles.toml".to_owned(), 3)));
    Ok(())
}

#[test]
fn second_project_with_same_id_points_at_its_id() -> TestResult {
    let copy = PROJECT.replace("acme-webshop", "acme-shop");
    let dir = home(&[
        ("projects/acme-webshop.toml", PROJECT),
        ("projects/acme-shop.toml", &copy),
    ])?;

    let error = read_error(dir.path())?;

    assert!(error.is_code("pace::core::same-id"), "{error:?}");
    assert_eq!(blamed(&error), Some(("acme-webshop.toml".to_owned(), 2)));
    Ok(())
}

#[test]
fn rates_for_project_of_other_client_point_at_them() -> TestResult {
    let clients = format!("{CLIENTS}\n[globex]\nname = \"Globex\"\ncurrency = \"USD\"\n");
    let project = PROJECT.replace("client = \"acme\"", "client = \"globex\"");
    let dir = home(&[
        ("roles.toml", ROLES),
        ("clients.toml", &clients),
        ("projects/acme-webshop.toml", &project),
    ])?;

    let error = read_error(dir.path())?;

    assert!(
        error.is_code("pace::core::rate-project-client"),
        "{error:?}"
    );
    assert_eq!(blamed(&error), Some(("clients.toml".to_owned(), 16)));
    Ok(())
}

#[test]
fn hidden_files_and_other_files_in_projects_are_no_records() -> TestResult {
    let dir = home(&[
        ("projects/.pace-a1b2c3", "half a write"),
        ("projects/.acme-webshop.toml", "version = 9"),
        ("projects/notes.md", "# notes"),
    ])?;

    let catalog = read_catalog(dir.path())?;

    assert_eq!(catalog.projects().count(), 0);
    Ok(())
}

#[test]
fn archived_project_reads_every_optional_key() -> TestResult {
    let project = format!(
        "{PROJECT}ends-on = 2026-12-31\nallow-overlap = true\n\
         archived = {{ on = 2027-01-05, reason = \"completed\" }}\n"
    );
    let dir = home(&[
        ("roles.toml", ROLES),
        ("clients.toml", CLIENTS),
        ("projects/acme-webshop.toml", &project),
    ])?;

    let catalog = read_catalog(dir.path())?;

    let webshop = catalog
        .project(&"acme-webshop".parse()?)
        .ok_or("the project")?;
    assert_eq!(webshop.ends_on, Some(date(2026, 12, 31)));
    assert!(webshop.allow_overlap);
    let archived = webshop.archived.ok_or("the project is archived")?;
    assert_eq!(archived.on, date(2027, 1, 5));
    Ok(())
}

#[test]
fn rounding_without_step_names_it() -> TestResult {
    let clients = CLIENTS.replace(", step = \"15m\"", "");
    let dir = home(&[("clients.toml", &clients)])?;

    let error = read_error(dir.path())?;

    assert_eq!(
        error.to_string(),
        "the client `acme` has no `rounding.step`"
    );
    assert_eq!(blamed(&error), Some(("clients.toml".to_owned(), 6)));
    Ok(())
}

#[test]
fn two_rates_on_one_day_are_refused() -> TestResult {
    let clients = CLIENTS.replace("from = 2026-07-01", "from = 2025-01-01");
    let dir = home(&[("clients.toml", &clients)])?;

    let error = read_error(dir.path())?;

    assert_eq!(
        error.to_string(),
        "two rates begin on 2025-01-01: 95.00 and 105.00"
    );
    assert!(error.is_code("pace::clients::invalid-value"), "{error:?}");
    Ok(())
}

#[test]
fn rate_without_from_names_its_client() -> TestResult {
    let clients = CLIENTS.replace("from = 2025-01-01\n", "");
    let dir = home(&[("clients.toml", &clients)])?;

    let error = read_error(dir.path())?;

    assert_eq!(
        error.to_string(),
        "the rate of the client `acme` has no `from`"
    );
    Ok(())
}

#[test]
fn project_rate_without_from_names_its_project() -> TestResult {
    let clients = CLIENTS.replace("from = 2026-09-01\n", "");
    let dir = home(&[("clients.toml", &clients)])?;

    let error = read_error(dir.path())?;

    assert_eq!(
        error.to_string(),
        "the rate of the project `acme-webshop` has no `from`"
    );
    assert_eq!(blamed(&error), Some(("clients.toml".to_owned(), 16)));
    Ok(())
}

/// A clients file with the client `acme` and `line` below its name and
/// currency.
fn acme_with(line: &str) -> String {
    format!("version = 1\n\n[acme]\nname = \"ACME GmbH\"\ncurrency = \"EUR\"\n{line}\n")
}

#[test]
fn nested_rates_that_are_no_list_say_to_write_their_dotted_header() -> TestResult {
    let dir = home(&[("clients.toml", &acme_with("rates = 95"))])?;

    let error = read_error(dir.path())?;

    assert_eq!(error.to_string(), "`acme.rates` is a list of tables");
    assert_eq!(
        error.hint(),
        Some("write each as `[[acme.rates]]` with its keys below")
    );
    assert_eq!(blamed(&error), Some(("clients.toml".to_owned(), 6)));
    Ok(())
}

#[test]
fn nested_rounding_that_is_no_table_says_to_write_its_dotted_header() -> TestResult {
    let dir = home(&[("clients.toml", &acme_with("rounding = 5"))])?;

    let error = read_error(dir.path())?;

    assert_eq!(error.to_string(), "`acme.rounding` is a table");
    assert_eq!(
        error.hint(),
        Some("write it as `[acme.rounding]` with its keys below")
    );
    Ok(())
}

#[test]
fn project_rates_that_are_no_table_say_to_write_their_dotted_header() -> TestResult {
    let dir = home(&[("clients.toml", &acme_with("projects = { webshop = 5 }"))])?;

    let error = read_error(dir.path())?;

    assert_eq!(
        error.hint(),
        Some("write it as `[acme.projects.webshop]` with its keys below")
    );
    Ok(())
}

#[test]
fn archive_of_role_that_is_no_table_says_to_write_its_dotted_header() -> TestResult {
    let roles = format!("{ROLES}archived = \"ended\"\n");
    let dir = home(&[("roles.toml", &roles)])?;

    let error = read_error(dir.path())?;

    assert_eq!(
        error.hint(),
        Some("write it as `[work.archived]` with its keys below")
    );
    Ok(())
}

#[test]
fn archive_of_project_that_is_no_table_says_to_write_its_header() -> TestResult {
    let project = format!("{PROJECT}archived = \"ended\"\n");
    let dir = home(&[("projects/acme-webshop.toml", &project)])?;

    let error = read_error(dir.path())?;

    assert_eq!(
        error.hint(),
        Some("write it as `[archived]` with its keys below")
    );
    Ok(())
}

#[test]
fn version_typo_in_roles_file_names_version() -> TestResult {
    let roles = ROLES.replace("version = 1", "verison = 1");
    let dir = home(&[("roles.toml", &roles)])?;

    let error = read_error(dir.path())?;

    assert_eq!(error.to_string(), "`verison` is not a key pace knows");
    assert_eq!(error.hint(), Some("did you mean `version`?"));
    assert_eq!(blamed(&error), Some(("roles.toml".to_owned(), 1)));
    Ok(())
}

#[test]
fn version_typo_in_clients_file_names_version() -> TestResult {
    let clients = CLIENTS.replace("version = 1", "verison = 1");
    let dir = home(&[("clients.toml", &clients)])?;

    let error = read_error(dir.path())?;

    assert_eq!(error.hint(), Some("did you mean `version`?"));
    assert!(error.is_code("pace::clients::unknown-key"), "{error:?}");
    Ok(())
}

#[test]
fn edited_roles_text_reads_in_place_of_roles_file() -> TestResult {
    let dir = plan_home()?;
    let roles_file = dir.path().join("roles.toml");
    let edited = ROLES.replace("name = \"Work\"", "name = \"Paid work\"");

    let catalog = read_catalog_with(dir.path(), &roles_file, &edited)?;

    let names: Vec<&str> = catalog.roles().map(|role| role.name.as_str()).collect();
    assert_eq!(names, ["Paid work"]);
    assert_eq!(std::fs::read_to_string(&roles_file)?, ROLES);
    Ok(())
}

#[test]
fn edited_roles_text_that_drops_project_role_points_at_project() -> TestResult {
    let dir = plan_home()?;
    let roles_file = dir.path().join("roles.toml");

    let error = read_catalog_with(dir.path(), &roles_file, "version = 1\n")
        .err()
        .ok_or("the catalog must not read")?;

    assert!(error.is_code("pace::core::unknown-role"), "{error:?}");
    assert_eq!(blamed(&error), Some(("acme-webshop.toml".to_owned(), 6)));
    Ok(())
}

#[test]
fn edited_roles_text_for_missing_roles_file_reads() -> TestResult {
    let dir = home(&[])?;
    let roles_file = dir.path().join("roles.toml");

    let catalog = read_catalog_with(dir.path(), &roles_file, ROLES)?;

    assert_eq!(catalog.roles().count(), 1);
    Ok(())
}

#[test]
fn catalog_file_is_written_whole() -> TestResult {
    let dir = home(&[])?;
    let roles_file = dir.path().join("roles.toml");

    write_catalog_file(&roles_file, ROLES)?;

    assert_eq!(std::fs::read_to_string(&roles_file)?, ROLES);
    Ok(())
}

#[test]
fn edited_text_of_new_project_record_reads() -> TestResult {
    let dir = home(&[("roles.toml", ROLES), ("clients.toml", CLIENTS)])?;
    let record = dir.path().join("projects/acme-webshop.toml");

    let catalog = read_catalog_with(dir.path(), &record, PROJECT)?;

    assert_eq!(catalog.projects().count(), 1);
    Ok(())
}
