//! `pace project`: make the working directory a project, and add, list,
//! archive and edit the project records in `projects/<key>.toml`.

use std::path::Path;

use abscissa_core::{Command, Runnable};
use pace_core::{
    ArchiveReason, Archived, Catalog, Ids, ProjectId, ProjectKey, ProjectRecord, RoleId,
};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult, closest_name};
use pace_store::{
    catalog::{project_record_path, read_catalog_with, write_catalog_file},
    field_edit::{FieldEdit, FieldValue, write_field_edits},
    toml_file::TomlFile,
};

use crate::application::{Context, PACE_APP, exit_on_error, print_error};
use crate::clock::{RandomIds, clock_for_run};
use crate::editor::{EditedFile, edit_in_editor, edit_until_checked, editor_command};
use crate::output::{aligned, listed, usage};
use crate::print::print_lines;
use crate::project::{Records, manifest_in};
use crate::prompt::Ask;
use crate::questions::add_project::{AddProjectFlow, ProjectFlags};
use crate::questions::init_project::{
    InitFlags, InitInputs, InitProject, InitProjectFlow, ManifestChange,
};
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
fn described(record: &ProjectRecord) -> String {
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
fn defaults_line(defaults: &[String], key: &ProjectKey) -> Option<String> {
    (!defaults.is_empty()).then(|| {
        format!(
            "pace took these defaults: {}; `pace project edit {key}` changes them.",
            listed(defaults)
        )
    })
}

/// `--billable` (`true`) or `--not-billable` (`false`); none without
/// either.
const fn billable_flag(billable: bool, not_billable: bool) -> Option<bool> {
    if billable {
        Some(true)
    } else if not_billable {
        Some(false)
    } else {
        None
    }
}

/// `pace project init`
#[derive(clap::Args, Command, Debug)]
pub struct ProjectInitCmd {
    #[arg(
        long,
        help = "What you type for the project; the directory's name if left out"
    )]
    key: Option<String>,

    #[arg(long, help = "What you read; the directory's name if left out")]
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

    #[arg(long, help = "Write the manifest as pace.toml, not as .pace.toml")]
    visible: bool,
}

impl Runnable for ProjectInitCmd {
    fn run(&self) {
        exit_on_error(PACE_APP.context().and_then(|context| self.execute(context)));
    }
}

impl ProjectInitCmd {
    /// Ask for what the directory and the flags leave open, then write the
    /// record, the manifest and a client or role made on the way.
    fn execute(&self, context: &Context) -> PaceResult<()> {
        let flags = InitFlags {
            key: self.key.clone(),
            project: ProjectFlags {
                name: self.name.clone(),
                client: self.client.clone(),
                role: self.role.clone(),
                billable: billable_flag(self.billable, self.not_billable),
                ..ProjectFlags::default()
            },
            visible: self.visible,
        };
        let home = &context.dirs.data.path;
        let records = Records {
            catalog: &context.catalog,
            home,
        };
        let inputs = InitInputs {
            home: home.clone(),
            dir: context.cwd.clone(),
            manifest: manifest_in(&context.cwd, records)?,
            today: context.today()?,
            project_id: ProjectId::try_from(RandomIds.next_id())?,
            role_id: RoleId::try_from(RandomIds.next_id())?,
        };
        let flow = InitProjectFlow::new(&context.catalog, inputs, flags.clone())?;
        let asked = ask_flow(flow, &context.terminal)?;
        refuse_unused_record_flags(&asked.made, &flags)?;
        let lines = initialised_lines(&asked);
        write_field_edits(asked.edits)?;
        print_lines(lines)
    }
}

/// Refuse the flags of a new record when `made` found the record of its
/// key: they would change nothing.
///
/// # Errors
///
/// If the record exists and `flags` give its name, client, role or
/// billable flag: a usage error that names `pace project edit`.
fn refuse_unused_record_flags(made: &InitProject, flags: &InitFlags) -> PaceResult<()> {
    let unused: Vec<String> = flags
        .record_flags()
        .iter()
        .map(|flag| format!("`{flag}`"))
        .collect();
    if made.record_added || unused.is_empty() {
        return Ok(());
    }
    let key = &made.record.key;
    Err(PaceError::new(
        ErrorKind::InvalidInput,
        "the project `{key}` has a record already, which `pace project init` doesn't change \
         with {flags}",
    )
    .attach_context("key", key.to_string())
    .attach_context("flags", listed(&unused))
    .attach_hint(format!(
        "leave the flags out, and change the record with `pace project edit {key}`"
    ))
    .attach_exit_code(ExitCode::Usage))
}

/// What `pace project init` says it did: the client and the role it
/// added, the record it added or found, the manifest it wrote, and the
/// defaults it took.
fn initialised_lines(asked: &Asked<InitProject>) -> Vec<String> {
    let made = &asked.made;
    let mut lines = Vec::new();
    if let Some(client) = &made.client {
        lines.push(format!(
            "Added the client `{}` ({}) in {}.",
            client.key,
            client.name,
            client.currency.code()
        ));
    }
    if let Some(role) = &made.role {
        lines.push(format!(
            "Added the role `{}` ({}, {}).",
            role.key,
            role.name,
            role.kind.as_str()
        ));
    }
    let record = &made.record;
    lines.push(if made.record_added {
        format!("Added the project {}.", described(record))
    } else {
        format!(
            "Found the record of the project `{}` ({}).",
            record.key, record.name
        )
    });
    let manifest = made.manifest.file_name().map_or_else(
        || made.manifest.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    lines.extend(match made.manifest_change {
        ManifestChange::Added => Some(format!(
            "Wrote the manifest {manifest}, which links this directory to the project."
        )),
        ManifestChange::IdAdded {
            name_removed: false,
        } => Some(format!("Wrote the project's ID into {manifest}.")),
        ManifestChange::IdAdded { name_removed: true } => Some(format!(
            "Wrote the project's ID into {manifest} and removed its name, which the record holds."
        )),
        ManifestChange::NameRemoved => Some(format!(
            "Removed the project's name from {manifest}, which the record holds."
        )),
        ManifestChange::Unchanged => None,
    });
    lines.extend(defaults_line(&asked.defaults, &record.key));
    lines
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
        let hint = closest_name(
            key.as_str(),
            catalog.projects().map(|record| record.key.as_str()),
        )
        .map_or_else(
            || "`pace project list` shows the projects".to_owned(),
            |closest| format!("did you mean `{closest}`?"),
        );
        PaceError::new(
            ErrorKind::InvalidInput,
            "pace has no record of the project `{key}`",
        )
        .attach_context("key", key.to_string())
        .attach_hint(hint)
    })
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
    /// Open the project's record in the editor until the catalog reads it,
    /// then write it.
    fn execute(&self, context: &Context) -> PaceResult<()> {
        let key = project_key(&self.key)?;
        let record = known_project(&context.catalog, &key)?;
        let home = &context.dirs.data.path;
        let asker = &context.terminal;
        if !asker.can_ask() && context.editor.is_none() {
            return Err(no_terminal_to_edit(&project_record_path(home, &key), &key));
        }
        let editor = editor_command(context.editor.as_deref(), asker.can_ask())?;
        let line = edit_record(
            home,
            record,
            asker,
            |text| edit_in_editor(&editor, text, ".toml"),
            print_error,
        )?;
        print_lines([line])
    }
}

/// Let the user change the file of `record` in `home` with `edit` until
/// the catalog reads it with the same key and ID, and write it; the
/// sentence that says so. A mistake is shown with `report`, and `asker`
/// offers to open the record again.
fn edit_record(
    home: &Path,
    record: &ProjectRecord,
    asker: &dyn Ask,
    edit: impl FnMut(&str) -> PaceResult<String>,
    report: impl Fn(&PaceError),
) -> PaceResult<String> {
    let record_file = project_record_path(home, &record.key);
    let text = std::fs::read_to_string(&record_file).map_err(|error| {
        PaceError::with_source(ErrorKind::InputOutput, "cannot read {path}", error)
            .attach_context("path", record_file.display().to_string())
    })?;
    let command = format!("pace project edit {}", record.key);
    let (edited, catalog) = edit_until_checked(
        EditedFile {
            path: &record_file,
            command: &command,
        },
        &text,
        asker,
        edit,
        |edited| {
            same_identity(&record_file, edited, record)?;
            read_catalog_with(home, &record_file, edited)
        },
        report,
    )?;
    write_catalog_file(&record_file, &edited)?;
    let name = catalog
        .project(&record.key)
        .map_or_else(|| record.name.clone(), |saved| saved.name.clone());
    Ok(format!(
        "Saved the project `{}` ({name}) in {}.",
        record.key,
        record_file.display()
    ))
}

/// Check that `text`, the edited file `path` of `record`, keeps its key
/// and its ID. A key or an ID that is missing or doesn't read is left to
/// the catalog's reader.
///
/// # Errors
///
/// If `text` holds another key or another ID: renaming a project is not
/// built yet, and the ID is what its entries name. The error points at the
/// line.
fn same_identity(path: &Path, text: &str, record: &ProjectRecord) -> PaceResult<()> {
    let file = TomlFile {
        path,
        text,
        what: "a project record",
        noun: "key",
        code: "pace::project-record",
        kind: ErrorKind::InvalidInput,
        // The catalog's reader checks the version; this check reads only
        // the key and the ID.
        version: 1,
    };
    let document = file.parse()?;
    let table = document.table();
    let written = |name: &str| {
        table
            .get(name)
            .and_then(|value| Some((value, file.string(name, value, "").ok()?)))
    };
    if let Some((value, key)) = written("key")
        && key != record.key.as_str()
    {
        return Err(file
            .error_at(
                value.span(),
                "`pace project edit` can't change the key of the project `{key}` to `{edited}`",
                [("key", record.key.to_string()), ("edited", key.to_owned())],
                format!(
                    "write `key = \"{}\"` again; pace can't rename a project yet",
                    record.key
                ),
                "another key",
            )
            .attach_error_code("pace::project-record::key-changed"));
    }
    if let Some((value, id)) = written("id")
        && id.parse::<ProjectId>().is_ok_and(|id| id != record.id)
    {
        return Err(file
            .error_at(
                value.span(),
                "`pace project edit` can't change the ID of the project `{key}`",
                [("key", record.key.to_string())],
                format!(
                    "write `id = \"{}\"` again; the entries of the project name its ID",
                    record.id
                ),
                "another ID",
            )
            .attach_error_code("pace::project-record::id-changed"));
    }
    Ok(())
}

/// The error of `pace project edit` without a terminal and without an
/// editor: it names the record to edit by hand.
fn no_terminal_to_edit(record_file: &Path, key: &ProjectKey) -> Box<PaceError> {
    PaceError::new(
        ErrorKind::InvalidInput,
        "`pace project edit` has no terminal and no editor in VISUAL or EDITOR to open the project `{key}`",
    )
    .attach_context("key", key.to_string())
    .attach_hint(format!(
        "edit {} by hand, or set VISUAL or EDITOR",
        record_file.display()
    ))
    .attach_exit_code(ExitCode::Usage)
}

/// `text` as a project key; a usage error if it is none.
fn project_key(text: &str) -> PaceResult<ProjectKey> {
    text.parse().map_err(usage)
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

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

    /// The home of a scratch directory with the record of `webshop`, and
    /// that record as the catalog reads it.
    fn home_with_webshop() -> (tempfile::TempDir, ProjectRecord) {
        let home = pace_store::testing::scratch_dir().unwrap();
        let key: ProjectKey = "webshop".parse().unwrap();
        write_catalog_file(&project_record_path(home.path(), &key), WEBSHOP).unwrap();
        let record = pace_store::catalog::read_catalog(home.path())
            .unwrap()
            .project(&key)
            .unwrap()
            .clone();
        (home, record)
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
            &record,
            &Scripted::confirming([true]),
            saving(&[&renamed, &fixed]),
            |error| reported.borrow_mut().push(error.to_string()),
        )
        .unwrap();

        assert_eq!(
            reported.into_inner(),
            ["`pace project edit` can't change the key of the project `webshop` to `shop`"]
        );
        assert!(
            line.starts_with("Saved the project `webshop` (Web shop) in "),
            "{line}"
        );
        assert_eq!(
            std::fs::read_to_string(project_record_path(home.path(), &record.key)).unwrap(),
            fixed
        );
    }

    #[test]
    fn edit_leaves_key_that_is_no_text_to_catalog_reader() {
        let (home, record) = home_with_webshop();
        let broken = WEBSHOP.replace("key = \"webshop\"", "key = 7");

        let error = edit_record(
            home.path(),
            &record,
            &Scripted::without_terminal(),
            saving(&[&broken]),
            |_| {},
        )
        .unwrap_err();

        assert!(!error.to_string().contains("can't change"), "{error}");
        assert_eq!(
            std::fs::read_to_string(project_record_path(home.path(), &record.key)).unwrap(),
            WEBSHOP
        );
    }

    #[test]
    fn same_identity_takes_same_id_written_in_capitals() {
        let (_home, record) = home_with_webshop();
        let capitals = WEBSHOP.replace(ID, &ID.to_uppercase());

        assert!(same_identity(Path::new("webshop.toml"), &capitals, &record).is_ok());
    }
}
