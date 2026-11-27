//! The project context: which project the working directory belongs to.
//!
//! The first hit wins: `--project`, then `PACE_PROJECT`, then the nearest
//! `pace.toml` or `.pace.toml` above the working directory (like git), then
//! a path mapping in `[project-paths]` of the global file. Without any,
//! there is no project.
//!
//! The manifest only links its directory to a project: its `id`, its `key`
//! and the `default-tags` of the directory. The rest of the project is its
//! record in `PACE_HOME/projects/<key>.toml`. A project whose key has a
//! record goes by the record's ID; without a record, entries record the key.

use std::{
    collections::BTreeSet,
    ffi::OsString,
    fmt,
    ops::Range,
    path::{Path, PathBuf},
};

use pace_core::{Catalog, ProjectId, ProjectKey, ProjectRecord, ProjectRef, Tag};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};

use pace_store::catalog::project_record_path;
use pace_store::toml_file::{TomlFile, TomlTable, TomlValue, read_hint};

use crate::config::ProjectPath;

/// The two names of a project manifest: visible and hidden.
pub const MANIFEST_FILES: [&str; 2] = ["pace.toml", ".pace.toml"];

/// The version of the manifest format this pace reads and writes.
const MANIFEST_VERSION: i64 = 1;

/// The keys of a manifest's `[project]`; `name` still reads, with a
/// warning, until `pace project init` moves it into the record.
const MANIFEST_PROJECT_KEYS: [&str; 3] = ["id", "key", "default-tags"];

/// The project the working directory belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// Its ID: the record's, else the manifest's, if either gives one.
    pub id: Option<ProjectId>,
    /// Its key.
    pub key: ProjectKey,
    /// Its readable name: the record's, else the one an old manifest
    /// gives.
    pub name: Option<String>,
    /// The tags of the manifest that join each entry begun or logged in its
    /// directory.
    pub default_tags: BTreeSet<Tag>,
    /// Why this project was picked.
    pub reason: Reason,
    /// What the command prints with its notes, such as a manifest that
    /// still holds the project's name.
    pub warnings: Vec<String>,
}

/// The project records a project is resolved through: the catalog read from
/// `home`, the data directory.
#[derive(Debug, Clone, Copy)]
pub struct Records<'records> {
    /// The roles, clients and project records.
    pub catalog: &'records Catalog,
    /// `PACE_HOME`, where the records are.
    pub home: &'records Path,
}

impl Project {
    /// The project `key`, picked for `reason`, before its record is looked
    /// up.
    const fn keyed(key: ProjectKey, reason: Reason) -> Self {
        Self {
            id: None,
            key,
            name: None,
            default_tags: BTreeSet::new(),
            reason,
            warnings: Vec::new(),
        }
    }

    /// This project with the ID and the name of its record in `catalog`, if
    /// its key has one.
    fn linked(mut self, catalog: &Catalog) -> Self {
        if let Some(record) = catalog.project(&self.key) {
            self.id = Some(record.id);
            self.name = Some(record.name.clone());
        }
        self
    }

    /// How entries refer to the project: by its ID, else by its key.
    #[must_use]
    pub fn reference(&self) -> ProjectRef {
        self.id
            .map_or_else(|| ProjectRef::Key(self.key.clone()), ProjectRef::Id)
    }
}

/// Why a project was picked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// `--project`
    Flag,
    /// `PACE_PROJECT`
    Variable,
    /// The nearest manifest above the working directory.
    Manifest(PathBuf),
    /// A directory in `[project-paths]`.
    Mapping {
        /// The directory as written.
        written: String,
        /// The file the mapping is written in.
        file: PathBuf,
        /// The line it is written on, counted from 1.
        line: usize,
    },
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flag => f.write_str("from --project"),
            Self::Variable => f.write_str("from PACE_PROJECT"),
            Self::Manifest(path) => write!(f, "from {}, the nearest manifest", path.display()),
            Self::Mapping {
                written,
                file,
                line,
            } => write!(
                f,
                "from `{written}` in [project-paths] of {}:{line}",
                file.display()
            ),
        }
    }
}

/// Find the project for the working directory `cwd`, and resolve it
/// through the project `records`.
///
/// # Errors
///
/// If `--project` or `PACE_PROJECT` is no project key, a directory has both
/// manifest names, or the nearest manifest cannot be read, has an unknown
/// key or a value that doesn't fit, or gives another ID than the record of
/// its key.
pub fn discover_project(
    flag: Option<&str>,
    variable: Option<OsString>,
    cwd: &Path,
    project_paths: &[ProjectPath],
    records: Records<'_>,
) -> PaceResult<Option<Project>> {
    if let Some(key) = flag {
        let key = parse_key(key, "--project")?;
        return Ok(Some(
            Project::keyed(key, Reason::Flag).linked(records.catalog),
        ));
    }

    if let Some(key) = variable.filter(|value| !value.is_empty()) {
        let key = parse_key(&key.to_string_lossy(), "PACE_PROJECT")
            .map_err(|error| error.attach_exit_code(ExitCode::Error))?;
        return Ok(Some(
            Project::keyed(key, Reason::Variable).linked(records.catalog),
        ));
    }

    if let Some(path) = nearest_manifest(cwd)? {
        return read_manifest(&path, records).map(Some);
    }

    Ok(mapped(cwd, project_paths).map(|mapping| {
        let reason = Reason::Mapping {
            written: mapping.written.clone(),
            file: mapping.file.clone(),
            line: mapping.line,
        };
        Project::keyed(mapping.key.clone(), reason).linked(records.catalog)
    }))
}

/// The project key `key`, which `given_by` gave; its error gets a first
/// line that names `given_by`.
fn parse_key(key: &str, given_by: &str) -> PaceResult<ProjectKey> {
    key.parse().map_err(|error: Box<PaceError>| {
        error
            .prepend_guidance_line("the project given by {given_by} is not valid")
            .attach_context("given_by", given_by)
            .attach_exit_code(ExitCode::Usage)
    })
}

fn nearest_manifest(cwd: &Path) -> PaceResult<Option<PathBuf>> {
    for dir in cwd.ancestors() {
        if let Some(manifest) = manifest_file_in(dir)? {
            return Ok(Some(manifest));
        }
    }
    Ok(None)
}

/// The manifest file in `dir` itself; `None` if it has none.
///
/// # Errors
///
/// If `dir` has both manifest names.
fn manifest_file_in(dir: &Path) -> PaceResult<Option<PathBuf>> {
    let found: Vec<PathBuf> = MANIFEST_FILES
        .iter()
        .map(|name| dir.join(name))
        .filter(|path| path.is_file())
        .collect();
    match found.as_slice() {
        [] => Ok(None),
        [manifest] => Ok(Some(manifest.clone())),
        _ => Err(PaceError::new(
            ErrorKind::Project,
            "the directory {dir} has both a pace.toml and a .pace.toml",
        )
        .attach_context("dir", dir.display().to_string())
        .attach_hint("keep one of them: pace reads either name, but not both")
        .attach_error_code("pace::project::two-manifests")),
    }
}

fn read_manifest(path: &Path, records: Records<'_>) -> PaceResult<Project> {
    let text = read_manifest_text(path)?;
    parse_manifest(path, &text, records)
}

/// The text of the manifest at `path`.
fn read_manifest_text(path: &Path) -> PaceResult<String> {
    std::fs::read_to_string(path).map_err(|error| {
        let hint = read_hint(&error);
        PaceError::with_source(ErrorKind::Project, "cannot read the manifest {path}", error)
            .attach_context("path", path.display().to_string())
            .attach_hint(hint)
    })
}

/// A manifest's fields as written: the link of its directory to a
/// project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// The manifest's file.
    pub path: PathBuf,
    /// The project's ID, if the manifest gives one.
    pub id: Option<ProjectId>,
    /// The project's key.
    pub key: ProjectKey,
    /// The project's name, which a manifest of phase 2 still holds.
    pub name: Option<String>,
    /// The tags that join each entry begun or logged in its directory.
    pub default_tags: BTreeSet<Tag>,
}

/// The manifest in `dir` itself, not above it, checked against `records`;
/// `None` if `dir` has none.
///
/// # Errors
///
/// As [`discover_project`] for a manifest: if `dir` has both manifest
/// names, or the manifest cannot be read, has an unknown key or a value
/// that doesn't fit, or gives another ID than the record of its key.
pub fn manifest_in(dir: &Path, records: Records<'_>) -> PaceResult<Option<Manifest>> {
    let Some(path) = manifest_file_in(dir)? else {
        return Ok(None);
    };
    let text = read_manifest_text(&path)?;
    parse_manifest_fields(&path, &text, records).map(Some)
}

/// The manifest `text` of the file at `path`, as the project it links its
/// directory to, resolved through `records`.
fn parse_manifest(path: &Path, text: &str, records: Records<'_>) -> PaceResult<Project> {
    let manifest = parse_manifest_fields(path, text, records)?;
    let warnings = manifest
        .name
        .is_some()
        .then(|| {
            format!(
                "The manifest {} still holds the project's name; `pace project init` moves it \
                 into the project's record.",
                path.display()
            )
        })
        .into_iter()
        .collect();
    let project = Project {
        id: manifest.id,
        name: manifest.name,
        default_tags: manifest.default_tags,
        warnings,
        ..Project::keyed(manifest.key, Reason::Manifest(manifest.path))
    };
    Ok(project.linked(records.catalog))
}

/// The manifest `text` of the file at `path` as written, its ID checked
/// against the record of its key in `records`.
fn parse_manifest_fields(path: &Path, text: &str, records: Records<'_>) -> PaceResult<Manifest> {
    let file = TomlFile {
        path,
        text,
        what: "the project manifest",
        noun: "key",
        code: "pace::project",
        kind: ErrorKind::Project,
        version: MANIFEST_VERSION,
    };
    let document = file.parse()?;
    let table = document.table();
    file.check_version(table)?;

    let mut project = None;
    for (key, value) in table.entries() {
        match key.name() {
            "version" => {}
            "project" => project = Some(file.table("project", value)?),
            name => return Err(file.unknown_key(name, key.span(), &["version", "project"])),
        }
    }

    let mut id = None;
    let mut project_key = None;
    let mut name = None;
    let mut default_tags = BTreeSet::new();
    for (key, value) in project.map_or_else(Vec::new, TomlTable::entries) {
        match key.name() {
            "key" => project_key = Some(manifest_key(&file, value)?),
            "id" => id = Some((manifest_id(&file, value)?, value.span())),
            "name" => name = Some(project_name(&file, value)?),
            "default-tags" => default_tags = manifest_tags(&file, value)?,
            other => return Err(file.unknown_key(other, key.span(), &MANIFEST_PROJECT_KEYS)),
        }
    }

    let key = project_key.ok_or_else(|| {
        PaceError::new(
            ErrorKind::Project,
            "the manifest {path} names no project key",
        )
        .attach_context("path", path.display().to_string())
        .attach_hint("add `key = \"acme-webshop\"` under `[project]`")
        .attach_error_code("pace::project::no-key")
    })?;
    if let Some((id, span)) = &id {
        check_record_id(&file, &key, *id, span.clone(), records)?;
    }
    Ok(Manifest {
        path: path.to_path_buf(),
        id: id.map(|(id, _)| id),
        key,
        name,
        default_tags,
    })
}

/// The manifest's `key`.
fn manifest_key(file: &TomlFile<'_>, value: TomlValue<'_>) -> PaceResult<ProjectKey> {
    let text = file.string("key", value, "a project key like `acme-webshop`")?;
    text.parse::<ProjectKey>().map_err(|error| {
        file.point_at(error, value.span(), "this key")
            .attach_error_code("pace::project::invalid-value")
    })
}

/// The manifest's `id`.
fn manifest_id(file: &TomlFile<'_>, value: TomlValue<'_>) -> PaceResult<ProjectId> {
    let text = file.string("id", value, "the project's ID, a UUIDv7")?;
    text.parse::<ProjectId>().map_err(|error| {
        file.error_at(
            value.span(),
            "`{id}` is no project ID",
            [("id", text.to_owned())],
            "write the project's ID, or remove the line to go by the key",
            "this ID",
        )
        .attach_source(error)
        .attach_error_code("pace::project::invalid-value")
    })
}

/// The manifest's `default-tags`, each a tag.
fn manifest_tags(file: &TomlFile<'_>, value: TomlValue<'_>) -> PaceResult<BTreeSet<Tag>> {
    file.strings("default-tags", value, "a list of tags")?
        .into_iter()
        .map(|(text, span)| {
            text.parse::<Tag>().map_err(|error| {
                file.point_at(error, span, "this tag")
                    .attach_error_code("pace::project::invalid-value")
            })
        })
        .collect()
}

/// Check that the manifest's `id`, at `span`, is the ID of the record of
/// `key`, if `records` hold one, and no other project's ID.
fn check_record_id(
    file: &TomlFile<'_>,
    key: &ProjectKey,
    id: ProjectId,
    span: Range<usize>,
    records: Records<'_>,
) -> PaceResult<()> {
    if let Some(other) = records
        .catalog
        .project_by_id(id)
        .filter(|record| &record.key != key)
    {
        return Err(id_of_other_project(
            file,
            key,
            id,
            span,
            other,
            records.home,
        ));
    }
    let Some(record) = records
        .catalog
        .project(key)
        .filter(|record| record.id != id)
    else {
        return Ok(());
    };
    Err(file
        .error_at(
            span,
            "the manifest {manifest} gives the project `{key}` the ID {id}, but its record \
             {record} has the ID {record_id}",
            [
                ("manifest", file.path.display().to_string()),
                ("key", key.to_string()),
                ("id", id.to_string()),
                (
                    "record",
                    project_record_path(records.home, key).display().to_string(),
                ),
                ("record_id", record.id.to_string()),
            ],
            "write the record's ID into the manifest, or remove the manifest's `id` line to go \
             by the key",
            "this ID",
        )
        .attach_error_code("pace::project::id-mismatch"))
}

/// The error for a manifest whose `id`, at `span`, is the ID of the
/// `other` project's record, not of the project `key`.
fn id_of_other_project(
    file: &TomlFile<'_>,
    key: &ProjectKey,
    id: ProjectId,
    span: Range<usize>,
    other: &ProjectRecord,
    home: &Path,
) -> Box<PaceError> {
    file.error_at(
        span,
        "the manifest {manifest} gives the project `{key}` the ID {id}, but that is the ID of \
         the project `{other}` in {record}",
        [
            ("manifest", file.path.display().to_string()),
            ("key", key.to_string()),
            ("id", id.to_string()),
            ("other", other.key.to_string()),
            (
                "record",
                project_record_path(home, &other.key).display().to_string(),
            ),
        ],
        "write the key of that project into the manifest, or remove the manifest's `id` line \
         to make a project of its own",
        "this ID",
    )
    .attach_error_code("pace::project::id-of-other-project")
}

/// The project's `name`, which must hold more than blanks.
fn project_name(file: &TomlFile<'_>, value: TomlValue<'_>) -> PaceResult<String> {
    let name = file.string("name", value, "the project's name")?;
    if name.trim().is_empty() {
        return Err(file
            .error_at(
                value.span(),
                "the project's `name` holds no name",
                [],
                "write the project's name, or remove the line",
                "empty name",
            )
            .attach_error_code("pace::project::invalid-value"));
    }
    Ok(name.to_owned())
}

/// The mapping with the longest directory that contains `cwd`.
fn mapped<'paths>(cwd: &Path, project_paths: &'paths [ProjectPath]) -> Option<&'paths ProjectPath> {
    let canonical =
        |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let cwd = canonical(cwd);
    project_paths
        .iter()
        .map(|mapping| (canonical(&mapping.dir), mapping))
        .filter(|(dir, _)| cwd.starts_with(dir))
        .max_by_key(|(dir, _)| dir.components().count())
        .map(|(_, mapping)| mapping)
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const RECORD_ID: &str = "0193a7c4-0000-7000-8000-000000000001";

    fn webshop_record() -> ProjectRecord {
        ProjectRecord {
            id: RECORD_ID.parse().unwrap(),
            key: "acme-webshop".parse().unwrap(),
            name: "Webshop relaunch".to_owned(),
            client: None,
            role: None,
            billable: false,
            budget: None,
            starts_on: None,
            ends_on: None,
            done_when: None,
            allow_overlap: false,
            archived: None,
        }
    }

    fn with_record() -> Catalog {
        Catalog::new(Vec::new(), Vec::new(), vec![webshop_record()]).unwrap()
    }

    /// The text of `error` with `/` as the separator on every OS, as the
    /// made-up paths of these tests write it.
    fn message(error: &PaceError) -> String {
        error.to_string().replace('\\', "/")
    }

    fn parse(text: &str, catalog: &Catalog) -> PaceResult<Project> {
        let records = Records {
            catalog,
            home: Path::new("/home/me/.local/share/pace"),
        };
        parse_manifest(Path::new("/work/.pace.toml"), text, records)
    }

    #[test]
    fn manifest_reads_id_key_and_default_tags() -> TestResult {
        let text = "version = 1\n\n[project]\nid = \"0193a7c4-0000-7000-8000-0000000000ff\"\n\
                    key = \"garden\"\ndefault-tags = [\"dev\", \"ops\"]\n";

        let project = parse(text, &Catalog::default())?;

        assert_eq!(
            project.id,
            Some("0193a7c4-0000-7000-8000-0000000000ff".parse()?)
        );
        assert_eq!(project.key, "garden".parse()?);
        let tags: Vec<&str> = project.default_tags.iter().map(Tag::as_str).collect();
        assert_eq!(tags, ["dev", "ops"]);
        assert_eq!(project.warnings, Vec::<String>::new());
        Ok(())
    }

    #[test]
    fn manifest_with_name_reads_and_warns_that_project_init_moves_it() -> TestResult {
        let text = "[project]\nkey = \"garden\"\nname = \"Garden\"\n";

        let project = parse(text, &Catalog::default())?;

        assert_eq!(project.name.as_deref(), Some("Garden"));
        assert_eq!(
            project.warnings,
            [
                "The manifest /work/.pace.toml still holds the project's name; `pace project \
                 init` moves it into the project's record."
            ]
        );
        Ok(())
    }

    #[test]
    fn unknown_key_in_manifest_project_names_closest() -> TestResult {
        let text = "[project]\nkey = \"garden\"\ndefault-tag = [\"dev\"]\n";

        let error = parse(text, &Catalog::default())
            .err()
            .ok_or("an unknown key reads")?;

        assert_eq!(error.to_string(), "`default-tag` is not a key pace knows");
        assert_eq!(error.hint(), Some("did you mean `default-tags`?"));
        Ok(())
    }

    #[test]
    fn default_tag_that_is_no_tag_is_refused() -> TestResult {
        let text = "[project]\nkey = \"garden\"\ndefault-tags = [\"two words\"]\n";

        let error = parse(text, &Catalog::default())
            .err()
            .ok_or("a tag with a space reads")?;

        assert_eq!(error.to_string(), "`two words` is not a tag");
        Ok(())
    }

    #[test]
    fn manifest_key_with_record_goes_by_record_id_and_name() -> TestResult {
        let project = parse("[project]\nkey = \"acme-webshop\"\n", &with_record())?;

        assert_eq!(project.reference(), ProjectRef::Id(RECORD_ID.parse()?));
        assert_eq!(project.name.as_deref(), Some("Webshop relaunch"));
        Ok(())
    }

    #[test]
    fn manifest_key_without_record_goes_by_key() -> TestResult {
        let project = parse("[project]\nkey = \"acme-webshop\"\n", &Catalog::default())?;

        assert_eq!(
            project.reference(),
            ProjectRef::Key("acme-webshop".parse()?)
        );
        Ok(())
    }

    #[test]
    fn manifest_id_other_than_record_id_names_both_files() -> TestResult {
        let text =
            "[project]\nid = \"0193a7c4-0000-7000-8000-0000000000ff\"\nkey = \"acme-webshop\"\n";

        let error = parse(text, &with_record())
            .err()
            .ok_or("a manifest with another ID reads")?;

        assert_eq!(
            message(&error),
            "the manifest /work/.pace.toml gives the project `acme-webshop` the ID \
             0193a7c4-0000-7000-8000-0000000000ff, but its record \
             /home/me/.local/share/pace/projects/acme-webshop.toml has the ID \
             0193a7c4-0000-7000-8000-000000000001"
        );
        assert!(error.is_code("pace::project::id-mismatch"), "{error:?}");
        Ok(())
    }

    #[test]
    fn manifest_id_of_other_key_record_names_that_record() -> TestResult {
        let text = format!("[project]\nid = \"{RECORD_ID}\"\nkey = \"shop\"\n");

        let error = parse(&text, &with_record())
            .err()
            .ok_or("a manifest with another project's ID reads")?;

        assert_eq!(
            message(&error),
            "the manifest /work/.pace.toml gives the project `shop` the ID \
             0193a7c4-0000-7000-8000-000000000001, but that is the ID of the project \
             `acme-webshop` in /home/me/.local/share/pace/projects/acme-webshop.toml"
        );
        assert!(
            error.is_code("pace::project::id-of-other-project"),
            "{error:?}"
        );
        Ok(())
    }

    #[test]
    fn manifest_in_directory_gives_fields_as_written() -> TestResult {
        let dir = pace_store::testing::scratch_dir()?;
        let path = dir.path().join(".pace.toml");
        std::fs::write(
            &path,
            "[project]\nkey = \"acme-webshop\"\nname = \"Shop\"\n",
        )?;
        let catalog = with_record();
        let records = Records {
            catalog: &catalog,
            home: Path::new("/home"),
        };

        let manifest = manifest_in(dir.path(), records)?.ok_or("no manifest")?;

        assert_eq!(manifest.path, path);
        assert_eq!(manifest.id, None);
        assert_eq!(manifest.key, "acme-webshop".parse()?);
        assert_eq!(manifest.name.as_deref(), Some("Shop"));
        Ok(())
    }

    #[test]
    fn manifest_in_directory_looks_only_into_that_directory() -> TestResult {
        let dir = pace_store::testing::scratch_dir()?;
        std::fs::write(
            dir.path().join("pace.toml"),
            "[project]\nkey = \"garden\"\n",
        )?;
        std::fs::create_dir(dir.path().join("beds"))?;
        let records = Records {
            catalog: &Catalog::default(),
            home: Path::new("/home"),
        };

        assert_eq!(manifest_in(&dir.path().join("beds"), records)?, None);
        Ok(())
    }

    #[test]
    fn project_flag_with_record_goes_by_record_id() -> TestResult {
        let catalog = with_record();
        let records = Records {
            catalog: &catalog,
            home: Path::new("/home"),
        };

        let project = discover_project(Some("acme-webshop"), None, Path::new("/"), &[], records)?
            .ok_or("no project")?;

        assert_eq!(project.reference(), ProjectRef::Id(RECORD_ID.parse()?));
        assert_eq!(project.reason, Reason::Flag);
        Ok(())
    }
}
