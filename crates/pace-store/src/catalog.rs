//! The catalog's files in `PACE_HOME`: `roles.toml`, `clients.toml` and
//! `projects/<key>.toml`, read into one `Catalog`.
//!
//! [`read_catalog`] reads the three kinds of file and hands their records to
//! [`Catalog::new`], which checks them against each other. Every error
//! points at its file and line: a reader's error through [`TomlFile`], an
//! error of the check through the spot where one record names another.
//! A missing file holds no record, so a fresh `PACE_HOME` gives an empty
//! catalog.

mod clients;
mod fields;
mod projects;
mod roles;

use std::{
    io,
    ops::Range,
    path::{Path, PathBuf},
};

use pace_core::{Catalog, ProjectKey};
use pace_error::{ErrorKind, PaceError, PaceResult, Snippet};

use crate::disk::write_atomically;
use crate::toml_file::{TomlFile, read_hint};

/// The version of the roles file, the clients file and the project records
/// this pace reads.
const CATALOG_VERSION: i64 = 1;

/// The key that holds a catalog file's version, so no record can have it.
pub const VERSION_KEY: &str = "version";

/// The roles file in `PACE_HOME`.
pub const ROLES_FILE: &str = "roles.toml";

/// The clients file in `PACE_HOME`.
pub const CLIENTS_FILE: &str = "clients.toml";

/// The directory of the project records in `PACE_HOME`.
pub const PROJECTS_DIR: &str = "projects";

/// The file of the record of the project `key` in `home`:
/// `projects/<key>.toml`.
#[must_use]
pub fn project_record_path(home: &Path, key: &ProjectKey) -> PathBuf {
    home.join(PROJECTS_DIR).join(format!("{key}.toml"))
}

/// Read the roles, the clients and the project records in `home` into the
/// catalog.
///
/// # Errors
///
/// If a file cannot be read, is no valid TOML, has another `version`, an
/// unknown key or a value that doesn't fit; if a project record's key
/// differs from its file name; or if [`Catalog::new`] refuses the records,
/// such as a project whose client doesn't exist. Each error points at the
/// file and the line.
pub fn read_catalog(home: &Path) -> PaceResult<Catalog> {
    read_catalog_replacing(home, None)
}

/// [`read_catalog`], with `text` in place of the file `replaced`, such as
/// the roles file as the user edited it. The file itself stays as it is;
/// the errors name it, so a mistake in `text` points at its line.
///
/// # Errors
///
/// As [`read_catalog`].
pub fn read_catalog_with(home: &Path, replaced: &Path, text: &str) -> PaceResult<Catalog> {
    read_catalog_replacing(home, Some((replaced, text)))
}

/// Write `text` as the catalog file `path` whole, through a temporary file
/// next to it, such as the roles file once the user edited it.
///
/// # Errors
///
/// If the directory can't be created or the file can't be written.
pub fn write_catalog_file(path: &Path, text: &str) -> PaceResult<()> {
    write_atomically(path, text)
}

/// [`read_catalog`], with the text of one file `replaced`.
fn read_catalog_replacing(home: &Path, replaced: Option<(&Path, &str)>) -> PaceResult<Catalog> {
    let read = |path: PathBuf| match replaced {
        Some((replaced, text)) if path == replaced => Ok(Some(Source {
            path,
            text: text.to_owned(),
        })),
        _ => Source::read(path),
    };
    let mut sources = Vec::new();
    if let Some(source) = read(home.join(ROLES_FILE))? {
        sources.push((Kind::Roles, source));
    }
    if let Some(source) = read(home.join(CLIENTS_FILE))? {
        sources.push((Kind::Clients, source));
    }
    let projects_dir = home.join(PROJECTS_DIR);
    let mut project_paths = project_files(&projects_dir)?;
    if let Some((replaced, _)) = replaced
        && replaced.parent() == Some(projects_dir.as_path())
        && !project_paths.iter().any(|path| path == replaced)
    {
        project_paths.push(replaced.to_path_buf());
        project_paths.sort();
    }
    for path in project_paths {
        if let Some(source) = read(path)? {
            sources.push((Kind::Project, source));
        }
    }

    let mut mentions = Mentions::default();
    let (mut roles, mut clients, mut projects) = (Vec::new(), Vec::new(), Vec::new());
    for (index, (kind, source)) in sources.iter().enumerate() {
        let file = kind.file(source);
        let mut mentions = mentions.of_source(index);
        match kind {
            Kind::Roles => roles = roles::read_roles(&file, &mut mentions)?,
            Kind::Clients => clients = clients::read_clients(&file, &mut mentions)?,
            Kind::Project => projects.push(projects::read_project(&file, &mut mentions)?),
        }
    }
    Catalog::new(roles, clients, projects).map_err(|error| mentions.point(error, &sources))
}

/// The three kinds of file the catalog comes from.
#[derive(Debug, Clone, Copy)]
enum Kind {
    Roles,
    Clients,
    Project,
}

impl Kind {
    /// The file `source` as a [`TomlFile`] of this kind.
    fn file(self, source: &Source) -> TomlFile<'_> {
        let (what, code) = match self {
            Self::Roles => ("the roles file", "pace::roles"),
            Self::Clients => ("the clients file", "pace::clients"),
            Self::Project => ("a project record", "pace::project-record"),
        };
        TomlFile {
            path: &source.path,
            text: &source.text,
            what,
            noun: "key",
            code,
            kind: ErrorKind::InvalidInput,
            version: CATALOG_VERSION,
        }
    }
}

/// A file of the catalog and its content.
#[derive(Debug)]
struct Source {
    path: PathBuf,
    text: String,
}

impl Source {
    /// The file at `path`; `None` if it doesn't exist.
    fn read(path: PathBuf) -> PaceResult<Option<Self>> {
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(Some(Self { path, text })),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(cannot_read(&path, error)),
        }
    }
}

/// The project records in `dir`, ordered by name: its `.toml` files, but
/// for hidden ones such as a field edit's temporary file. None if `dir`
/// doesn't exist.
fn project_files(dir: &Path) -> PaceResult<Vec<PathBuf>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(cannot_read(dir, error)),
    };
    let mut files = Vec::new();
    for entry in entries {
        let path = entry.map_err(|error| cannot_read(dir, error))?.path();
        let hidden = path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with('.'));
        let toml = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("toml"));
        if toml && !hidden && !path.is_dir() {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// The error for a file or directory of the catalog that pace cannot read.
fn cannot_read(path: &Path, error: io::Error) -> Box<PaceError> {
    let hint = read_hint(&error);
    PaceError::with_source(ErrorKind::InputOutput, "cannot read {path}", error)
        .attach_context("path", path.display().to_string())
        .attach_hint(hint)
}

/// What a record names in a field that [`Catalog::new`] checks.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Field {
    /// A project's `client`.
    Client,
    /// A project's `role`.
    Role,
    /// A role's or a project's `id`.
    Id,
    /// A client's rates for the project with this key.
    ProjectRates(String),
}

/// Where a record names something that [`Catalog::new`] checks: the file,
/// the spot and what it names.
#[derive(Debug)]
struct Mention {
    /// The kind of record: `role`, `client` or `project`.
    noun: &'static str,
    /// The record's key.
    owner: String,
    field: Field,
    /// The index of the file in the list of sources.
    source: usize,
    span: Range<usize>,
}

/// The spots where records name others, so an error of [`Catalog::new`]
/// points at its file and line.
#[derive(Debug, Default)]
struct Mentions(Vec<Mention>);

impl Mentions {
    /// The recorder for the mentions in the source at `index`.
    const fn of_source(&mut self, index: usize) -> SourceMentions<'_> {
        SourceMentions {
            mentions: self,
            source: index,
        }
    }

    /// `error` of [`Catalog::new`], pointed at the spot it is about, if
    /// pace recorded it.
    fn point(&self, error: Box<PaceError>, sources: &[(Kind, Source)]) -> Box<PaceError> {
        let Some((noun, owner, field, label)) = about(&error) else {
            return error;
        };
        let mention = self.0.iter().find(|mention| {
            mention.noun == noun && mention.owner == owner && mention.field == field
        });
        let Some((mention, (_, source))) =
            mention.and_then(|mention| Some((mention, sources.get(mention.source)?)))
        else {
            return error;
        };
        error.attach_snippet(Snippet::new(
            source.path.display().to_string(),
            source.text.as_str(),
            mention.span.clone(),
            label,
        ))
    }
}

/// What an error of [`Catalog::new`] is about: the kind of record, its key,
/// the field, and the label for the spot.
fn about(error: &PaceError) -> Option<(&'static str, String, Field, &'static str)> {
    let context = |key| error.context_value(key).map(str::to_owned);
    if error.is_code("pace::core::unknown-client") {
        return Some(("project", context("owner")?, Field::Client, "this client"));
    }
    if error.is_code("pace::core::unknown-role") {
        return Some(("project", context("owner")?, Field::Role, "this role"));
    }
    if error.is_code("pace::core::unknown-project") {
        let project = Field::ProjectRates(context("key")?);
        return Some(("client", context("owner")?, project, "these rates"));
    }
    if error.is_code("pace::core::rate-project-client") {
        let project = Field::ProjectRates(context("project")?);
        return Some(("client", context("client")?, project, "these rates"));
    }
    if error.is_code("pace::core::same-id") {
        let noun = match error.context_value("plural")? {
            "roles" => "role",
            "projects" => "project",
            _ => return None,
        };
        return Some((noun, context("second")?, Field::Id, "the same ID"));
    }
    None
}

/// The recorder of mentions for one source.
#[derive(Debug)]
struct SourceMentions<'mentions> {
    mentions: &'mentions mut Mentions,
    source: usize,
}

impl SourceMentions<'_> {
    /// Record that the `noun` with the key `owner` names `field` at `span`.
    fn record(&mut self, noun: &'static str, owner: &str, field: Field, span: Range<usize>) {
        self.mentions.0.push(Mention {
            noun,
            owner: owner.to_owned(),
            field,
            source: self.source,
            span,
        });
    }
}
