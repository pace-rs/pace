//! `pace project`: make the working directory a project, and add, list,
//! archive and edit the project records in `projects/<key>.toml`.

use std::path::{Path, PathBuf};

use abscissa_core::{Command, Runnable};
use pace_core::{ArchiveReason, Archived, Catalog, Ids, ProjectId, ProjectKey, ProjectRecord};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult, closest_name};
use pace_store::{
    catalog::{
        MendedCatalog, points_at_file, project_record_keys, project_record_path,
        read_catalog_mending, write_catalog_file,
    },
    field_edit::{FieldEdit, FieldValue, write_field_edits},
    toml_file::{TomlFile, TomlTable, TomlValue},
};

use crate::application::{Context, PACE_APP, exit_on_error, print_error};
use crate::clock::{RandomIds, clock_for_run};
use crate::commands::listing_hint;
use crate::commands::project_init::ProjectInitCmd;
use crate::editor::{EditedFile, SavedFile, edit_in_editor, edit_until_checked, editor_command};
use crate::output::{aligned, listed, usage};
use crate::print::print_lines;
use crate::prompt::Ask;
use crate::questions::add_project::{AddProjectFlow, ProjectFlags};
use crate::questions::{Asked, ask_flow};

/// Keep the project records: each project's client, role, billing and
/// budget, also without a directory.
#[derive(clap::Subcommand, Command, Debug, Runnable)]
pub enum ProjectCmd {
    /// Make this directory a project: its record and its manifest
    Init(ProjectInitCmd),

    /// Add a project record; pace asks for what the flags leave open
    Add(ProjectAddCmd),

    /// Show the projects in use
    List(ProjectListCmd),

    /// Archive a project: its record stays, so old entries keep it
    Archive(ProjectArchiveCmd),

    /// Change a project record in your editor
    Edit(ProjectEditCmd),
}

/// `pace project add`
#[derive(clap::Args, Command, Debug)]
pub struct ProjectAddCmd {
    #[arg(
        value_name = "KEY",
        help = "What you type for the project, such as `acme-webshop`"
    )]
    key: String,

    #[arg(long, help = "What you read; the key if left out")]
    name: Option<String>,

    #[arg(
        long,
        value_name = "KEY",
        help = "The client who pays for it; none if left out"
    )]
    client: Option<String>,

    #[arg(
        long,
        value_name = "KEY",
        help = "The role its time goes to; none if left out"
    )]
    role: Option<String>,

    #[arg(
        long,
        conflicts_with = "not_billable",
        help = "Its time is billable; the default with a client"
    )]
    billable: bool,

    #[arg(long, help = "Its time is not billable; the default without a client")]
    not_billable: bool,

    #[arg(
        long,
        value_name = "DURATION",
        help = "The time it may take, such as 120h"
    )]
    budget: Option<String>,

    #[arg(long, value_name = "DAY", help = "Its first day, such as 2026-12-01")]
    starts_on: Option<String>,

    #[arg(long, value_name = "DAY", help = "Its last day, such as 2027-03-31")]
    ends_on: Option<String>,

    #[arg(long, value_name = "TEXT", help = "The result that makes it done")]
    done_when: Option<String>,

    #[arg(long, help = "Its entries may overlap other entries")]
    allow_overlap: bool,
}

impl Runnable for ProjectAddCmd {
    fn run(&self) {
        exit_on_error(PACE_APP.context().and_then(|context| self.execute(context)));
    }
}

impl ProjectAddCmd {
    fn execute(&self, context: &Context) -> PaceResult<()> {
        let key = project_key(&self.key)?;
        let flags = ProjectFlags {
            name: self.name.clone(),
            client: self.client.clone(),
            role: self.role.clone(),
            billable: billable_flag(self.billable, self.not_billable),
            budget: self.budget.clone(),
            starts_on: self.starts_on.clone(),
            ends_on: self.ends_on.clone(),
            done_when: self.done_when.clone(),
            allow_overlap: self.allow_overlap,
        };
        let id = ProjectId::try_from(RandomIds.next_id())?;
        let record_file = project_record_path(&context.dirs.data.path, &key);
        let flow = AddProjectFlow::new(&context.catalog, &record_file, key, id, flags)?;
        let asked = ask_flow(flow, &context.terminal)?;
        let lines = added_lines(&asked);
        write_field_edits(asked.edits)?;
        print_lines(lines)
    }
}

/// What `pace project add` says it did, and the defaults it took.
fn added_lines(asked: &Asked<ProjectRecord>) -> Vec<String> {
    let record = &asked.made;
    let mut lines = vec![format!("Added the project {}.", described(record))];
    lines.extend(defaults_line(&asked.defaults, &record.key));
    lines
}

/// `record` as a sentence names it: "`webshop` (Webshop) for the client
/// `acme` in the role `work`, billable".
pub(crate) fn described(record: &ProjectRecord) -> String {
    let client = record
        .client
        .as_ref()
        .map_or_else(String::new, |client| format!(" for the client `{client}`"));
    let role = record
        .role
        .as_ref()
        .map_or_else(String::new, |role| format!(" in the role `{role}`"));
    let billable = if record.billable {
        "billable"
    } else {
        "not billable"
    };
    format!(
        "`{}` ({}){client}{role}, {billable}",
        record.key, record.name
    )
}

/// The line that names the `defaults` taken for the project `key`; none
/// without defaults.
pub(crate) fn defaults_line(defaults: &[String], key: &ProjectKey) -> Option<String> {
    (!defaults.is_empty()).then(|| {
        format!(
            "pace took these defaults: {}; `pace project edit {key}` changes them.",
            listed(defaults)
        )
    })
}

/// `--billable` (`true`) or `--not-billable` (`false`); none without
/// either.
pub(crate) const fn billable_flag(billable: bool, not_billable: bool) -> Option<bool> {
    if billable {
        Some(true)
    } else if not_billable {
        Some(false)
    } else {
        None
    }
}

/// `pace project list`
#[derive(clap::Args, Command, Debug)]
pub struct ProjectListCmd {
    #[arg(long, help = "Show the archived projects too")]
    archived: bool,
}

impl Runnable for ProjectListCmd {
    fn run(&self) {
        exit_on_error(
            PACE_APP
                .context()
                .and_then(|context| print_lines(project_lines(&context.catalog, self.archived))),
        );
    }
}

/// The project records of `catalog` as a table: key, name, client, role,
/// billable and budget, and with `archived` the archived projects, with
/// when and why.
fn project_lines(catalog: &Catalog, archived: bool) -> Vec<String> {
    let records: Vec<&ProjectRecord> = catalog
        .projects()
        .filter(|record| archived || record.archived.is_none())
        .collect();
    if records.is_empty() {
        let any_archived = catalog.projects().next().is_some();
        return vec![if any_archived {
            "Every project is archived; `pace project list --archived` shows them.".to_owned()
        } else {
            "There are no project records yet; `pace project add <key>` adds one.".to_owned()
        }];
    }
    let mut rows = vec![
        ["key", "name", "client", "role", "billable", "budget"]
            .map(str::to_owned)
            .to_vec(),
    ];
    if archived {
        rows[0].push("archived".to_owned());
    }
    for record in records {
        let mut row = vec![
            record.key.to_string(),
            record.name.clone(),
            record
                .client
                .as_ref()
                .map_or_else(String::new, ToString::to_string),
            record
                .role
                .as_ref()
                .map_or_else(String::new, ToString::to_string),
            if record.billable { "yes" } else { "no" }.to_owned(),
            record
                .budget
                .map_or_else(String::new, |budget| format!("{budget:#}")),
        ];
        if archived {
            row.push(record.archived.map_or_else(String::new, |archived| {
                format!("{}, {}", archived.on, archived.reason.as_str())
            }));
        }
        rows.push(row);
    }
    aligned(&rows)
}

/// `pace project archive`
#[derive(clap::Args, Command, Debug)]
pub struct ProjectArchiveCmd {
    #[arg(value_name = "KEY", help = "The project, by its key")]
    key: String,

    #[arg(
        long,
        required = true,
        value_parser = ["completed", "abandoned", "ended", "inactive"],
        help = "Why: completed, abandoned, ended or inactive"
    )]
    reason: String,
}

impl Runnable for ProjectArchiveCmd {
    fn run(&self) {
        exit_on_error(PACE_APP.context().and_then(|context| self.execute(context)));
    }
}

impl ProjectArchiveCmd {
    fn execute(&self, context: &Context) -> PaceResult<()> {
        let key = project_key(&self.key)?;
        let reason: ArchiveReason = self.reason.parse().map_err(usage)?;
        let clock = clock_for_run(
            &context.settings.settings.time_zone,
            context.test_now.clone(),
        )?;
        let archived = Archived {
            on: clock.now().date(),
            reason,
        };
        let (record, edit) =
            archive_edit(&context.catalog, &context.dirs.data.path, &key, archived)?;
        let line = format!(
            "Archived the project `{}` ({}) on {} as {}.",
            record.key,
            record.name,
            archived.on,
            reason.as_str()
        );
        write_field_edits(vec![edit])?;
        print_lines([line])
    }
}

/// The record of the project `key` of `catalog`, and the edit of its file
/// in `home` that archives it as `archived` says.
///
/// # Errors
///
/// If `catalog` has no record of `key`: the error names the closest key.
/// If the project is archived already, the error exits with 3.
fn archive_edit<'catalog>(
    catalog: &'catalog Catalog,
    home: &Path,
    key: &ProjectKey,
    archived: Archived,
) -> PaceResult<(&'catalog ProjectRecord, FieldEdit)> {
    let record = known_project(catalog, key)?;
    if let Some(earlier) = record.archived {
        return Err(PaceError::new(
            ErrorKind::InvalidInput,
            "the project `{key}` is archived already, since {on}",
        )
        .attach_context("key", key.to_string())
        .attach_context("on", earlier.on.to_string())
        .attach_hint(format!(
            "`pace project edit {key}` changes the day or the reason"
        ))
        .attach_exit_code(ExitCode::NothingToDo));
    }
    let edit = FieldEdit::set(
        project_record_path(home, key),
        &["archived"],
        FieldValue::InlineTable(vec![
            ("on".to_owned(), FieldValue::Date(archived.on)),
            (
                "reason".to_owned(),
                FieldValue::String(archived.reason.as_str().to_owned()),
            ),
        ]),
    );
    Ok((record, edit))
}

/// The record of the project `key` of `catalog`.
///
/// # Errors
///
/// If `catalog` has no record of `key`: the error names the closest key.
fn known_project<'catalog>(
    catalog: &'catalog Catalog,
    key: &ProjectKey,
) -> PaceResult<&'catalog ProjectRecord> {
    catalog.project(key).ok_or_else(|| {
        unknown_project(
            key,
            catalog.projects().map(|record| record.key.as_str()),
            None,
        )
    })
}

/// The error for the project `key` that pace has no record of: it names
/// the closest of `keys`.
fn unknown_project<'key>(
    key: &ProjectKey,
    keys: impl Iterator<Item = &'key str>,
    left: Option<(&Path, &PaceError)>,
) -> Box<PaceError> {
    let hint = closest_name(key.as_str(), keys).map_or_else(
        || listing_hint("pace project list", "projects", left),
        |closest| format!("did you mean `{closest}`?"),
    );
    PaceError::new(
        ErrorKind::InvalidInput,
        "pace has no record of the project `{key}`",
    )
    .attach_context("key", key.to_string())
    .attach_hint(hint)
}

/// `pace project edit`
#[derive(clap::Args, Command, Debug)]
pub struct ProjectEditCmd {
    #[arg(value_name = "KEY", help = "The project, by its key")]
    key: String,
}

impl Runnable for ProjectEditCmd {
    fn run(&self) {
        exit_on_error(PACE_APP.context().and_then(|context| self.execute(context)));
    }
}

impl ProjectEditCmd {
    /// The record file in `home` that the command opens; `None` if its key
    /// is no project key.
    pub(crate) fn record_file(&self, home: &Path) -> Option<PathBuf> {
        project_key(&self.key)
            .ok()
            .map(|key| project_record_path(home, &key))
    }

    /// Open the project's record in the editor until the catalog reads it,
    /// then write it.
    fn execute(&self, context: &Context) -> PaceResult<()> {
        let key = project_key(&self.key)?;
        let home = &context.dirs.data.path;
        let opened = opened_record(home, &context.catalog, key)?;
        let asker = &context.terminal;
        if !asker.can_ask() && context.editor.is_none() {
            return Err(without_editor(opened));
        }
        let editor = editor_command(context.editor.as_deref(), asker.can_ask())?;
        edit_record(
            home,
            opened,
            asker,
            |text| edit_in_editor(&editor, text, ".toml"),
            print_error,
        )?
        .print()
    }
}

/// The record file of a project that `pace project edit` opens.
#[derive(Debug)]
struct OpenedRecord {
    /// The project, whose key the edited file keeps.
    key: ProjectKey,
    /// The ID that the edited file keeps; `None` while a mistake keeps the
    /// catalog from reading and the file holds no ID that reads.
    id: Option<ProjectId>,
    /// The record file.
    path: PathBuf,
    /// The text of the record file as it is.
    text: String,
    /// The project's name as the catalog reads it; its key while a mistake
    /// keeps the catalog from reading.
    name: String,
    /// The mistake of the record file that keeps the catalog from reading,
    /// if there is one.
    mistake: Option<Box<PaceError>>,
}

/// The record file of the project `key` in `home`, which `catalog` reads.
/// When a mistake keeps the catalog from reading, `catalog` is empty: the
/// ID and the name come from the record as it reads alone, or, with a
/// mistake in the record file, the ID is the one the file holds.
///
/// # Errors
///
/// If no record of `key` reads: the error names the closest key. If the
/// file can't be read.
fn opened_record(home: &Path, catalog: &Catalog, key: ProjectKey) -> PaceResult<OpenedRecord> {
    let path = project_record_path(home, &key);
    let (id, name, mistake) = match catalog.project(&key) {
        Some(record) => (Some(record.id), record.name.clone(), None),
        None => match record_alone(home, &path, catalog, &key)? {
            RecordAlone::Read(record) => (Some(record.id), record.name, None),
            RecordAlone::Broken(mistake) => (None, key.to_string(), Some(mistake)),
        },
    };
    let text = std::fs::read_to_string(&path).map_err(|error| {
        PaceError::with_source(ErrorKind::InputOutput, "cannot read {path}", error)
            .attach_context("path", path.display().to_string())
    })?;
    let id = id.or_else(|| written_id(&path, &text));
    Ok(OpenedRecord {
        key,
        id,
        path,
        text,
        name,
        mistake,
    })
}

/// A record file as it reads alone, while a mistake keeps the catalog
/// from reading.
#[derive(Debug)]
enum RecordAlone {
    /// The file reads; the mistake is in another file.
    Read(ProjectRecord),
    /// The mistake of the file itself.
    Broken(Box<PaceError>),
}

/// The record file `path` of the project `key`, which `catalog` has no
/// record of, as it reads alone in `home`.
///
/// # Errors
///
/// If no record of `key` reads: the error names the closest key of the
/// records that read. If a file can't be read, or every file reads and
/// [`Catalog::new`] refuses them for a mistake in another file.
fn record_alone(
    home: &Path,
    path: &Path,
    catalog: &Catalog,
    key: &ProjectKey,
) -> PaceResult<RecordAlone> {
    match read_catalog_mending(home, path, None) {
        Err(mistake) if points_at_file(&mistake, path) => Ok(RecordAlone::Broken(mistake)),
        Err(error) => Err(error),
        Ok(MendedCatalog::Alone(mut records, left)) => {
            let found = records
                .projects
                .iter()
                .position(|record| record.key == *key);
            let Some(at) = found else {
                let files = project_record_keys(home)?;
                let keys = records.projects.iter().map(|record| record.key.as_str());
                return Err(unknown_project(
                    key,
                    keys.chain(files.iter().map(String::as_str)),
                    Some((home, &left)),
                ));
            };
            Ok(RecordAlone::Read(records.projects.swap_remove(at)))
        }
        Ok(MendedCatalog::Whole(_)) => Err(unknown_project(
            key,
            catalog.projects().map(|record| record.key.as_str()),
            None,
        )),
    }
}

/// Let the user change the `opened` record file with `edit` until the
/// catalog of `home` reads it with the same key and ID, and write it; the
/// sentence that says so. A mistake is shown with `report`, a mistake of
/// the file as it is before the editor opens, and `asker` offers to open
/// the record again. When the record reads alone but another file has a
/// mistake of its own, pace saves the record and leaves that mistake.
fn edit_record(
    home: &Path,
    opened: OpenedRecord,
    asker: &dyn Ask,
    edit: impl FnMut(&str) -> PaceResult<String>,
    report: impl Fn(&PaceError),
) -> PaceResult<SavedFile> {
    let OpenedRecord {
        key,
        id,
        path,
        text,
        name,
        mistake,
    } = opened;
    if let Some(mistake) = &mistake {
        report(mistake);
    }
    let command = format!("pace project edit {key}");
    let (edited, read) = edit_until_checked(
        EditedFile {
            home,
            path: &path,
            command: &command,
        },
        &text,
        asker,
        edit,
        |edited| {
            same_identity(&path, edited, &key, id)?;
            read_catalog_mending(home, &path, Some(edited))
        },
        report,
    )?;
    write_catalog_file(&path, &edited)?;
    let saved_name =
        |record: Option<&ProjectRecord>| record.map_or(name, |saved| saved.name.clone());
    let (name, left) = match read {
        MendedCatalog::Whole(catalog) => (saved_name(catalog.project(&key)), None),
        MendedCatalog::Alone(records, mistake) => (
            saved_name(records.projects.iter().find(|record| record.key == key)),
            Some(mistake),
        ),
    };
    let line = format!("Saved the project `{key}` ({name}) in {}.", path.display());
    Ok(SavedFile::new(home, line, left))
}

/// The record file `path` with `text`, to read its key and its ID.
const fn record_toml<'file>(path: &'file Path, text: &'file str) -> TomlFile<'file> {
    TomlFile {
        path,
        text,
        what: "a project record",
        noun: "key",
        code: "pace::project-record",
        kind: ErrorKind::InvalidInput,
        // The catalog's reader checks the version; this check reads only
        // the key and the ID.
        version: 1,
    }
}

/// The text of the string `name` of `table` in `file`, and its value;
/// `None` if `table` has no `name`, or no string.
fn written<'file>(
    file: &TomlFile<'_>,
    table: TomlTable<'file>,
    name: &str,
) -> Option<(TomlValue<'file>, &'file str)> {
    table
        .get(name)
        .and_then(|value| Some((value, file.string(name, value, "").ok()?)))
}

/// The ID that `text`, the record file `path`, holds; `None` if the text
/// is no TOML, or its ID is missing or doesn't read.
fn written_id(path: &Path, text: &str) -> Option<ProjectId> {
    let file = record_toml(path, text);
    let document = file.parse().ok()?;
    let (_, id) = written(&file, document.table(), "id")?;
    id.parse().ok()
}

/// Check that `text`, the edited record file `path` of the project `key`,
/// keeps the key and the ID `id`, if there is one. A key or an ID that is
/// missing or doesn't read is left to the catalog's reader.
///
/// # Errors
///
/// If `text` holds another key or another ID: renaming a project is not
/// built yet, and the ID is what its entries name. The error points at the
/// line.
fn same_identity(
    path: &Path,
    text: &str,
    key: &ProjectKey,
    id: Option<ProjectId>,
) -> PaceResult<()> {
    let file = record_toml(path, text);
    let document = file.parse()?;
    let table = document.table();
    if let Some((value, edited)) = written(&file, table, "key")
        && edited != key.as_str()
    {
        return Err(file
            .error_at(
                value.span(),
                "`pace project edit` can't change the key of the project `{key}` to `{edited}`",
                [("key", key.to_string()), ("edited", edited.to_owned())],
                format!("write `key = \"{key}\"` again; pace can't rename a project yet"),
                "another key",
            )
            .attach_error_code("pace::project-record::key-changed"));
    }
    if let Some(id) = id
        && let Some((value, edited)) = written(&file, table, "id")
        && edited.parse::<ProjectId>().is_ok_and(|edited| edited != id)
    {
        return Err(file
            .error_at(
                value.span(),
                "`pace project edit` can't change the ID of the project `{key}`",
                [("key", key.to_string())],
                format!("write `id = \"{id}\"` again; the entries of the project name its ID"),
                "another ID",
            )
            .attach_error_code("pace::project-record::id-changed"));
    }
    Ok(())
}

/// The error of `pace project edit` without a terminal and without an
/// editor: the mistake of the `opened` file with [`NO_TERMINAL`] after it,
/// as the user mends it by hand; [`no_terminal_to_edit`] if it has none.
fn without_editor(opened: OpenedRecord) -> Box<PaceError> {
    match opened.mistake {
        Some(mistake) => mistake
            .append_guidance_line(NO_TERMINAL)
            .attach_context("project-key", opened.key.to_string()),
        None => no_terminal_to_edit(&opened.path, &opened.key),
    }
}

/// The error of `pace project edit` without a terminal and without an
/// editor for a file without a mistake: it names the record to edit by hand.
fn no_terminal_to_edit(record_file: &Path, key: &ProjectKey) -> Box<PaceError> {
    PaceError::new(ErrorKind::InvalidInput, NO_TERMINAL)
        .attach_context("project-key", key.to_string())
        .attach_hint(format!(
            "edit {} by hand, or set VISUAL or EDITOR",
            record_file.display()
        ))
        .attach_exit_code(ExitCode::Usage)
}

/// The line that says `pace project edit` has no terminal and no editor;
/// its placeholder is `{project-key}`.
const NO_TERMINAL: &str = "`pace project edit` has no terminal and no editor in VISUAL or EDITOR to open the project `{project-key}`";

/// `text` as a project key; a usage error if it is none.
fn project_key(text: &str) -> PaceResult<ProjectKey> {
    text.parse().map_err(usage)
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use pace_store::catalog::read_catalog;

    use super::*;
    use crate::prompt::Scripted;

    const ID: &str = "0193a7c4-0000-7000-8000-000000000001";

    const WEBSHOP: &str = "version = 1\nid = \"0193a7c4-0000-7000-8000-000000000001\"\nkey = \"webshop\"\nname = \"Webshop\"\n";

    fn record(key: &str, archived: Option<Archived>) -> ProjectRecord {
        ProjectRecord {
            id: RandomIds.next_id().try_into().unwrap(),
            key: key.parse().unwrap(),
            name: key.to_uppercase(),
            client: None,
            role: None,
            billable: false,
            budget: None,
            starts_on: None,
            ends_on: None,
            done_when: None,
            allow_overlap: false,
            archived,
        }
    }

    fn completed() -> Archived {
        Archived {
            on: date(2026, 1, 1),
            reason: ArchiveReason::Completed,
        }
    }

    fn catalog(records: Vec<ProjectRecord>) -> Catalog {
        Catalog::new(Vec::new(), Vec::new(), records).unwrap()
    }

    /// The home of a scratch directory with `text` as the record of
    /// `webshop`, and the record file that `pace project edit` opens.
    fn home_with_webshop_as(text: &str) -> (tempfile::TempDir, OpenedRecord) {
        let home = pace_store::testing::scratch_dir().unwrap();
        let key: ProjectKey = "webshop".parse().unwrap();
        write_catalog_file(&project_record_path(home.path(), &key), text).unwrap();
        let catalog = read_catalog(home.path()).unwrap_or_default();
        let opened = opened_record(home.path(), &catalog, key).unwrap();
        (home, opened)
    }

    /// The home of a scratch directory with the record of `webshop`, and
    /// the record file that `pace project edit` opens.
    fn home_with_webshop() -> (tempfile::TempDir, OpenedRecord) {
        home_with_webshop_as(WEBSHOP)
    }

    /// [`WEBSHOP`] with a mistake: `billable` is no boolean.
    fn broken_webshop() -> String {
        format!("{WEBSHOP}billable = \"yes\"\n")
    }

    /// An editor that turns the text it gets into `saved` in turn.
    fn saving(saved: &[&str]) -> impl FnMut(&str) -> PaceResult<String> {
        let mut saved: Vec<String> = saved.iter().rev().map(|&text| text.to_owned()).collect();
        move |_| {
            saved.pop().ok_or_else(|| {
                PaceError::new(ErrorKind::Internal, "the test has no more saved texts")
            })
        }
    }

    #[test]
    fn list_of_archived_projects_only_points_to_archived_flag() {
        let catalog = catalog(vec![record("old", Some(completed()))]);

        assert_eq!(
            project_lines(&catalog, false),
            ["Every project is archived; `pace project list --archived` shows them."]
        );
    }

    #[test]
    fn archive_of_unknown_project_without_close_key_points_to_list() {
        let catalog = catalog(vec![record("webshop", None)]);

        let error = archive_edit(
            &catalog,
            Path::new("home"),
            &"garden".parse().unwrap(),
            completed(),
        )
        .unwrap_err();

        assert_eq!(error.hint(), Some("`pace project list` shows the projects"));
    }

    #[test]
    fn edit_with_changed_key_opens_again_and_writes_fixed_record() {
        let (home, record) = home_with_webshop();
        let renamed = WEBSHOP.replace("key = \"webshop\"", "key = \"shop\"");
        let fixed = WEBSHOP.replace("name = \"Webshop\"", "name = \"Web shop\"");
        let reported = std::cell::RefCell::new(Vec::new());

        let line = edit_record(
            home.path(),
            record,
            &Scripted::confirming([true]),
            saving(&[&renamed, &fixed]),
            |error| reported.borrow_mut().push(error.to_string()),
        )
        .unwrap()
        .line;

        assert_eq!(
            reported.into_inner(),
            ["`pace project edit` can't change the key of the project `webshop` to `shop`"]
        );
        assert!(
            line.starts_with("Saved the project `webshop` (Web shop) in "),
            "{line}"
        );
        assert_eq!(
            std::fs::read_to_string(home.path().join("projects/webshop.toml")).unwrap(),
            fixed
        );
    }

    #[test]
    fn edit_leaves_key_that_is_no_text_to_catalog_reader() {
        let (home, record) = home_with_webshop();
        let broken = WEBSHOP.replace("key = \"webshop\"", "key = 7");

        let error = edit_record(
            home.path(),
            record,
            &Scripted::without_terminal(),
            saving(&[&broken]),
            |_| {},
        )
        .unwrap_err();

        assert!(!error.to_string().contains("can't change"), "{error}");
        assert_eq!(
            std::fs::read_to_string(home.path().join("projects/webshop.toml")).unwrap(),
            WEBSHOP
        );
    }

    #[test]
    fn same_identity_takes_same_id_written_in_capitals() {
        let capitals = WEBSHOP.replace(ID, &ID.to_uppercase());

        assert!(
            same_identity(
                Path::new("webshop.toml"),
                &capitals,
                &"webshop".parse().unwrap(),
                ID.parse().ok(),
            )
            .is_ok()
        );
    }

    #[test]
    fn edit_of_project_that_catalog_reads_without_it_names_closest_key() {
        let (home, _) = home_with_webshop();
        let catalog = read_catalog(home.path()).unwrap();

        let error = opened_record(home.path(), &catalog, "wehshop".parse().unwrap()).unwrap_err();

        assert_eq!(error.hint(), Some("did you mean `webshop`?"));
    }

    #[test]
    fn edit_of_broken_record_keeps_id_it_holds_and_reports_mistake_first() {
        let (home, opened) = home_with_webshop_as(&broken_webshop());
        let other_id = broken_webshop().replace(ID, "0193a7c4-0000-7000-8000-0000000000ff");
        let fixed = WEBSHOP.replace("name = \"Webshop\"", "name = \"Web shop\"");
        let reported = std::cell::RefCell::new(Vec::new());

        assert_eq!(opened.id, ID.parse().ok());
        let line = edit_record(
            home.path(),
            opened,
            &Scripted::confirming([true]),
            saving(&[&other_id, &fixed]),
            |error| reported.borrow_mut().push(error.to_string()),
        )
        .unwrap()
        .line;

        assert_eq!(
            reported.into_inner(),
            [
                "`billable` takes `true` or `false`",
                "`pace project edit` can't change the ID of the project `webshop`",
            ]
        );
        assert!(
            line.starts_with("Saved the project `webshop` (Web shop) in "),
            "{line}"
        );
        assert_eq!(
            std::fs::read_to_string(home.path().join("projects/webshop.toml")).unwrap(),
            fixed
        );
    }

    #[test]
    fn broken_record_that_is_no_toml_opens_without_id() {
        let (_home, opened) = home_with_webshop_as("version = 1\nid = \"");

        assert_eq!(opened.id, None);
        assert!(opened.mistake.is_some());
    }

    #[test]
    fn record_opens_with_its_id_and_name_while_other_record_has_mistake() {
        let (home, _) = home_with_webshop();
        let garden = home.path().join("projects/garden.toml");
        write_catalog_file(&garden, "version = 1\nkey = \"garden\"\n").unwrap();

        let opened =
            opened_record(home.path(), &Catalog::default(), "webshop".parse().unwrap()).unwrap();

        assert_eq!(
            opened.id,
            Some("0193a7c4-0000-7000-8000-000000000001".parse().unwrap())
        );
        assert_eq!(opened.name, "Webshop");
        assert!(opened.mistake.is_none());
    }

    #[test]
    fn unknown_project_beside_mistake_in_other_record_names_closest_key() {
        let (home, _) = home_with_webshop();
        let garden = home.path().join("projects/garden.toml");
        write_catalog_file(&garden, "version = 1\nkey = \"garden\"\n").unwrap();

        let error = opened_record(home.path(), &Catalog::default(), "webshob".parse().unwrap())
            .unwrap_err();

        assert_eq!(error.hint(), Some("did you mean `webshop`?"));
    }

    #[test]
    fn unknown_project_beside_broken_record_names_its_key() {
        let (home, _) = home_with_webshop_as(&broken_webshop());

        let error = opened_record(home.path(), &Catalog::default(), "webshob".parse().unwrap())
            .unwrap_err();

        assert_eq!(error.hint(), Some("did you mean `webshop`?"));
    }

    #[test]
    fn unknown_project_beside_broken_record_names_command_that_fixes_it() {
        let (home, _) = home_with_webshop_as(&broken_webshop());

        let error =
            opened_record(home.path(), &Catalog::default(), "garden".parse().unwrap()).unwrap_err();

        assert_eq!(
            error.hint(),
            Some("`pace project edit webshop` fixes the mistake that stops `pace project list`")
        );
    }

    #[test]
    fn edit_of_broken_record_without_terminal_or_editor_shows_its_mistake() {
        let (_home, opened) = home_with_webshop_as(&broken_webshop());

        let error = without_editor(opened);

        assert_eq!(
            error.to_string(),
            "`billable` takes `true` or `false`\n`pace project edit` has no terminal and no editor in VISUAL or EDITOR to open the project `webshop`"
        );
        assert_eq!(error.exit_code(), ExitCode::Error);
    }

    #[test]
    fn edit_of_record_without_terminal_or_editor_names_record() {
        let (home, opened) = home_with_webshop();

        let error = without_editor(opened);

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(
            error.hint(),
            Some(
                format!(
                    "edit {} by hand, or set VISUAL or EDITOR",
                    home.path().join("projects/webshop.toml").display()
                )
                .as_str()
            )
        );
    }
}
