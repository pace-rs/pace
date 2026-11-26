//! The project context: which project the working directory belongs to.
//!
//! The first hit wins: `--project`, then `PACE_PROJECT`, then the nearest
//! `pace.toml` or `.pace.toml` above the working directory (like git), then
//! a path mapping in `[project-paths]` of the global file. Without any,
//! there is no project.

use std::{
    ffi::OsString,
    fmt,
    path::{Path, PathBuf},
};

use pace_core::{ProjectId, ProjectKey, ProjectRef};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};

use pace_store::toml_file::{TomlFile, in_file_order};

use crate::{config::ProjectPath, origin::Origin};

/// The two names of a project manifest: visible and hidden.
pub const MANIFEST_FILES: [&str; 2] = ["pace.toml", ".pace.toml"];

/// The version of the manifest format this pace reads and writes.
const MANIFEST_VERSION: i64 = 1;

/// The project the working directory belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// Its ID, once `project init` wrote it into the manifest.
    pub id: Option<ProjectId>,
    /// Its key.
    pub key: ProjectKey,
    /// Its readable name, if the manifest gives one.
    pub name: Option<String>,
    /// Why this project was picked.
    pub reason: Reason,
}

impl Project {
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
        /// The file the mapping is written in, and its line.
        written_in: Origin,
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
                written_in: Origin::File { path, line },
            } => write!(
                f,
                "from `{written}` in [project-paths] of {}:{line}",
                path.display()
            ),
            Self::Mapping {
                written,
                written_in,
            } => write!(f, "from `{written}` in [project-paths], {written_in}"),
        }
    }
}

/// Find the project for the working directory `cwd`.
///
/// # Errors
///
/// If `--project` or `PACE_PROJECT` is no project key, a directory has both
/// manifest names, or the nearest manifest cannot be read or has an
/// unknown key or a value that doesn't fit.
pub fn discover_project(
    flag: Option<&str>,
    variable: Option<OsString>,
    cwd: &Path,
    project_paths: &[ProjectPath],
) -> PaceResult<Option<Project>> {
    if let Some(key) = flag {
        let key = parse_key(key, "--project")?;
        return Ok(Some(Project {
            id: None,
            key,
            name: None,
            reason: Reason::Flag,
        }));
    }

    if let Some(key) = variable.filter(|value| !value.is_empty()) {
        let key = parse_key(&key.to_string_lossy(), "PACE_PROJECT")
            .map_err(|error| error.attach_exit_code(ExitCode::Error))?;
        return Ok(Some(Project {
            id: None,
            key,
            name: None,
            reason: Reason::Variable,
        }));
    }

    if let Some(path) = nearest_manifest(cwd)? {
        let (id, key, name) = read_manifest(&path)?;
        return Ok(Some(Project {
            id,
            key,
            name,
            reason: Reason::Manifest(path),
        }));
    }

    Ok(mapped(cwd, project_paths).map(|mapping| Project {
        id: None,
        key: mapping.key.clone(),
        name: None,
        reason: Reason::Mapping {
            written: mapping.written.clone(),
            written_in: mapping.origin.clone(),
        },
    }))
}

fn parse_key(key: &str, given_by: &str) -> PaceResult<ProjectKey> {
    key.parse().map_err(|error: Box<PaceError>| {
        PaceError::new(
            ErrorKind::InvalidInput,
            "{given_by}: `{key}` is not a project key",
        )
        .attach_context("given_by", given_by)
        .attach_context("key", key)
        .attach_hint(error.hint().unwrap_or_default())
        .attach_exit_code(ExitCode::Usage)
    })
}

fn nearest_manifest(cwd: &Path) -> PaceResult<Option<PathBuf>> {
    for dir in cwd.ancestors() {
        let found: Vec<PathBuf> = MANIFEST_FILES
            .iter()
            .map(|name| dir.join(name))
            .filter(|path| path.is_file())
            .collect();
        match found.as_slice() {
            [] => {}
            [manifest] => return Ok(Some(manifest.clone())),
            _ => {
                return Err(PaceError::new(
                    ErrorKind::Project,
                    "{dir} has both a pace.toml and a .pace.toml",
                )
                .attach_context("dir", dir.display().to_string())
                .attach_hint("keep one of them: pace reads either name, but not both")
                .attach_error_code("pace::project::two-manifests"));
            }
        }
    }
    Ok(None)
}

fn read_manifest(path: &Path) -> PaceResult<(Option<ProjectId>, ProjectKey, Option<String>)> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        PaceError::with_source(ErrorKind::Project, "cannot read the manifest {path}", error)
            .attach_context("path", path.display().to_string())
            .attach_hint("check that pace may read the file")
    })?;
    let file = TomlFile {
        path,
        text: &text,
        what: "the project manifest",
        noun: "key",
        code: "pace::project",
        version: MANIFEST_VERSION,
    };
    let table = file.parse()?;

    let mut project = None;
    for (key, value) in in_file_order(table.get_ref()) {
        match key.get_ref().as_ref() {
            "version" => file.check_version(value)?,
            "project" => project = Some(file.table("project", value)?),
            name => return Err(file.unknown_key(name, key.span(), &["version", "project"])),
        }
    }

    let mut id = None;
    let mut project_key = None;
    let mut name = None;
    for (key, value) in project.map_or_else(Vec::new, in_file_order) {
        match key.get_ref().as_ref() {
            "key" => {
                let text = file.string("key", value, "a project key like `acme-webshop`")?;
                project_key = Some(text.parse::<ProjectKey>().map_err(|error| {
                    file.error_at(
                        value.span(),
                        error.to_string(),
                        error.hint().unwrap_or_default(),
                        "this key",
                    )
                    .attach_error_code("pace::project::invalid-value")
                })?);
            }
            "id" => {
                let text = file.string(
                    "id",
                    value,
                    "the project's ID, written by `pace project init`",
                )?;
                id = Some(text.parse::<ProjectId>().map_err(|_| {
                    file.error_at(
                        value.span(),
                        format!("`{text}` is no project ID"),
                        "`pace project init` writes the ID; remove the line to go by the key",
                        "this ID",
                    )
                    .attach_error_code("pace::project::invalid-value")
                })?);
            }
            "name" => name = Some(file.string("name", value, "the project's name")?.to_owned()),
            other => return Err(file.unknown_key(other, key.span(), &["id", "key", "name"])),
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
    Ok((id, key, name))
}

/// The mapping with the longest directory that contains `cwd`.
fn mapped<'p>(cwd: &Path, project_paths: &'p [ProjectPath]) -> Option<&'p ProjectPath> {
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
