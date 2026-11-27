//! `pace doctor` reads the catalog's files with every mistake named: each
//! record of each file, with its file and line.

use std::path::{Path, PathBuf};

use pace_error::PaceError;
use pace_store::{
    catalog::{catalog_file_paths, check_catalog_files},
    testing::scratch_dir,
};

use crate::TestResult;

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

/// The catalog files of `home` with their texts, as the app gathers them:
/// a file that doesn't exist is left out.
fn gathered(home: &Path) -> Result<Vec<(PathBuf, String)>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    for path in catalog_file_paths(home)? {
        if path.is_file() {
            let text = std::fs::read_to_string(&path)?;
            files.push((path, text));
        }
    }
    Ok(files)
}

/// Each mistake as its file name, line and message.
fn named(mistakes: &[Box<PaceError>]) -> Vec<(String, Option<usize>, String)> {
    mistakes
        .iter()
        .map(|mistake| {
            let file = mistake
                .snippet_name()
                .and_then(|name| Path::new(name).file_name())
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            (file, mistake.snippet_line(), mistake.to_string())
        })
        .collect()
}

const TWO_BROKEN_ROLES: &str = r#"version = 1

[work]
id = "0193a7c4-0000-7000-8000-000000000001"
kind = "aera"
name = "Work"

[health]
id = "0193a7c4-0000-7000-8000-000000000003"
kind = "area"
name = "Health"
colour = "green"
"#;

const BROKEN_CLIENT: &str = r#"version = 1

[acme]
name = "ACME GmbH"
"#;

const PROJECT_OF_UNKNOWN_CLIENT: &str = r#"version = 1
id = "0193a7c4-0000-7000-8000-000000000002"
key = "acme-webshop"
name = "Webshop relaunch"
client = "acme"
"#;

#[test]
fn every_mistake_of_every_catalog_file_is_named_with_its_line() -> TestResult {
    let dir = home(&[
        ("roles.toml", TWO_BROKEN_ROLES),
        ("clients.toml", BROKEN_CLIENT),
        ("projects/notes.toml", "version = 1\nkey = \n"),
    ])?;

    let mistakes = check_catalog_files(dir.path(), gathered(dir.path())?);

    let found: Vec<(String, Option<usize>)> = named(&mistakes)
        .into_iter()
        .map(|(file, line, _)| (file, line))
        .collect();
    assert_eq!(
        found,
        [
            ("roles.toml".to_owned(), Some(5)),
            ("roles.toml".to_owned(), Some(12)),
            ("clients.toml".to_owned(), Some(3)),
            ("notes.toml".to_owned(), Some(2)),
        ]
    );
    Ok(())
}

#[test]
fn records_that_read_but_disagree_give_the_catalog_check() -> TestResult {
    let dir = home(&[("projects/acme-webshop.toml", PROJECT_OF_UNKNOWN_CLIENT)])?;

    let mistakes = check_catalog_files(dir.path(), gathered(dir.path())?);

    let found = named(&mistakes);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "acme-webshop.toml");
    assert_eq!(found[0].1, Some(5));
    Ok(())
}

#[test]
fn catalog_files_that_read_have_no_mistake() -> TestResult {
    let dir = home(&[])?;

    assert_eq!(catalog_file_paths(dir.path())?.len(), 2);
    assert_eq!(
        check_catalog_files(dir.path(), gathered(dir.path())?).len(),
        0
    );
    Ok(())
}
