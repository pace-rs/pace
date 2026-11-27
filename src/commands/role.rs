//! `pace role`: add, list, archive and edit the roles in `roles.toml`.

use std::path::Path;

use abscissa_core::{Command, Runnable};
use pace_core::{ArchiveReason, Archived, Catalog, Ids, Role, RoleId, RoleKey};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult, closest_name};
use pace_store::{
    catalog::{ROLES_FILE, points_at_file, read_catalog_with, write_catalog_file},
    field_edit::{FieldEdit, FieldValue, write_field_edits},
};

use crate::clock::{RandomIds, clock_for_run};
use crate::editor::{EditedFile, edit_in_editor, edit_until_checked, editor_command};
use crate::output::{aligned, counted, listed, usage};
use crate::print::print_lines;
use crate::prompt::Ask;
use crate::questions::add_role::{AddRoleFlow, RoleFlags};
use crate::questions::ask_flow;
use crate::{
    application::{Context, PACE_APP, exit_on_error, print_error},
    questions::Asked,
};

/// The text of a roles file that doesn't exist yet, as `pace role edit`
/// opens it.
const NEW_ROLES_FILE: &str = "\
version = 1

# [work]
# id = \"…\" # `pace role add` makes the ID
# kind = \"area\" # area | area-project | meta
# name = \"Work\"
# purpose = \"Earn a living with good work\"
# weekly-target = \"30h\"
";

/// Keep the roles: the lasting responsibilities time goes to.
#[derive(clap::Subcommand, Command, Debug, Runnable)]
pub enum RoleCmd {
    /// Add a role; pace asks for what the flags leave open
    Add(RoleAddCmd),

    /// Show the roles in use
    List(RoleListCmd),

    /// Archive a role: it stays, so old entries keep it
    Archive(RoleArchiveCmd),

    /// Change the roles file in your editor
    Edit(RoleEditCmd),
}

/// `pace role add`
#[derive(clap::Args, Command, Debug)]
pub struct RoleAddCmd {
    #[arg(
        value_name = "KEY",
        help = "What you type for the role, such as `work`"
    )]
    key: String,

    #[arg(long, help = "What you read; the key if left out")]
    name: Option<String>,

    #[arg(
        long,
        value_parser = ["area", "area-project", "meta"],
        help = "area, area-project or meta; area if left out"
    )]
    kind: Option<String>,

    #[arg(long, help = "Why the role exists")]
    purpose: Option<String>,

    #[arg(
        long,
        value_name = "DURATION",
        help = "The time the role should get each week, such as 30h"
    )]
    weekly_target: Option<String>,
}

impl Runnable for RoleAddCmd {
    fn run(&self) {
        exit_on_error(PACE_APP.context().and_then(|context| self.execute(context)));
    }
}

impl RoleAddCmd {
    fn execute(&self, context: &Context) -> PaceResult<()> {
        let key = role_key(&self.key)?;
        let flags = RoleFlags {
            name: self.name.clone(),
            kind: self
                .kind
                .as_deref()
                .map(str::parse)
                .transpose()
                .map_err(usage)?,
            purpose: self.purpose.clone(),
            weekly_target: self.weekly_target.clone(),
        };
        let id = RoleId::try_from(RandomIds.next_id())?;
        let roles_file = context.dirs.data.path.join(ROLES_FILE);
        let flow = AddRoleFlow::new(&context.catalog, &roles_file, key, id, flags)?;
        let asked = ask_flow(flow, &context.terminal)?;
        let lines = added_lines(&asked);
        write_field_edits(asked.edits)?;
        print_lines(lines)
    }
}

/// What `pace role add` says it did, and the defaults it took.
fn added_lines(asked: &Asked<Role>) -> Vec<String> {
    let role = &asked.made;
    let mut lines = vec![format!(
        "Added the role `{}` ({}, {}).",
        role.key,
        role.name,
        role.kind.as_str()
    )];
    if !asked.defaults.is_empty() {
        lines.push(format!(
            "pace took these defaults: {}; `pace role edit` changes them.",
            listed(&asked.defaults)
        ));
    }
    lines
}

/// `pace role list`
#[derive(clap::Args, Command, Debug)]
pub struct RoleListCmd {
    #[arg(long, help = "Show the archived roles too")]
    archived: bool,
}

impl Runnable for RoleListCmd {
    fn run(&self) {
        exit_on_error(
            PACE_APP
                .context()
                .and_then(|context| print_lines(role_lines(&context.catalog, self.archived))),
        );
    }
}

/// The roles of `catalog` as a table: key, kind, name and weekly target,
/// and with `archived` the archived roles, with when and why.
fn role_lines(catalog: &Catalog, archived: bool) -> Vec<String> {
    let roles: Vec<&Role> = catalog
        .roles()
        .filter(|role| archived || role.archived.is_none())
        .collect();
    if roles.is_empty() {
        let any_archived = catalog.roles().next().is_some();
        return vec![if any_archived {
            "Every role is archived; `pace role list --archived` shows them.".to_owned()
        } else {
            "There are no roles yet; `pace role add <key>` adds one.".to_owned()
        }];
    }
    let mut rows = vec![vec![
        "key".to_owned(),
        "kind".to_owned(),
        "name".to_owned(),
        "weekly target".to_owned(),
    ]];
    if archived {
        rows[0].push("archived".to_owned());
    }
    for role in roles {
        let mut row = vec![
            role.key.to_string(),
            role.kind.as_str().to_owned(),
            role.name.clone(),
            role.weekly_target
                .map_or_else(String::new, |target| format!("{target:#}")),
        ];
        if archived {
            row.push(role.archived.map_or_else(String::new, |archived| {
                format!("{}, {}", archived.on, archived.reason.as_str())
            }));
        }
        rows.push(row);
    }
    aligned(&rows)
}

/// `pace role archive`
#[derive(clap::Args, Command, Debug)]
pub struct RoleArchiveCmd {
    #[arg(value_name = "KEY", help = "The role, by its key")]
    key: String,

    #[arg(
        long,
        required = true,
        value_parser = ["completed", "abandoned", "ended", "inactive"],
        help = "Why: completed, abandoned, ended or inactive"
    )]
    reason: String,
}

impl Runnable for RoleArchiveCmd {
    fn run(&self) {
        exit_on_error(PACE_APP.context().and_then(|context| self.execute(context)));
    }
}

impl RoleArchiveCmd {
    fn execute(&self, context: &Context) -> PaceResult<()> {
        let key = role_key(&self.key)?;
        let reason: ArchiveReason = self.reason.parse().map_err(usage)?;
        let clock = clock_for_run(
            &context.settings.settings.time_zone,
            context.test_now.clone(),
        )?;
        let archived = Archived {
            on: clock.now().date(),
            reason,
        };
        let roles_file = context.dirs.data.path.join(ROLES_FILE);
        let (role, edit) = archive_edit(&context.catalog, &roles_file, &key, archived)?;
        let line = format!(
            "Archived the role `{}` ({}) on {} as {}.",
            role.key,
            role.name,
            archived.on,
            reason.as_str()
        );
        write_field_edits(vec![edit])?;
        print_lines([line])
    }
}

/// The role `key` of `catalog`, and the edit of `roles_file` that archives
/// it as `archived` says.
///
/// # Errors
///
/// If `catalog` has no role `key`: the error names the closest key. If the
/// role is archived already, the error exits with 3.
fn archive_edit<'catalog>(
    catalog: &'catalog Catalog,
    roles_file: &Path,
    key: &RoleKey,
    archived: Archived,
) -> PaceResult<(&'catalog Role, FieldEdit)> {
    let Some(role) = catalog.role(key) else {
        let hint = closest_name(key.as_str(), catalog.roles().map(|role| role.key.as_str()))
            .map_or_else(
                || "`pace role list` shows the roles".to_owned(),
                |closest| format!("did you mean `{closest}`?"),
            );
        return Err(PaceError::new(
            ErrorKind::InvalidInput,
            "pace doesn't know the role `{key}`",
        )
        .attach_context("key", key.to_string())
        .attach_hint(hint));
    };
    if let Some(earlier) = role.archived {
        return Err(PaceError::new(
            ErrorKind::InvalidInput,
            "the role `{key}` is archived already, since {on}",
        )
        .attach_context("key", key.to_string())
        .attach_context("on", earlier.on.to_string())
        .attach_hint("`pace role edit` changes the day or the reason")
        .attach_exit_code(ExitCode::NothingToDo));
    }
    let edit = FieldEdit::set(
        roles_file,
        &[key.as_str(), "archived"],
        FieldValue::InlineTable(vec![
            ("on".to_owned(), FieldValue::Date(archived.on)),
            (
                "reason".to_owned(),
                FieldValue::String(archived.reason.as_str().to_owned()),
            ),
        ]),
    );
    Ok((role, edit))
}

/// `pace role edit`
#[derive(clap::Args, Command, Debug)]
pub struct RoleEditCmd {}

impl Runnable for RoleEditCmd {
    fn run(&self) {
        exit_on_error(PACE_APP.context().and_then(edit_roles));
    }
}

/// Open the roles file in the editor until the catalog reads it, then
/// write it.
fn edit_roles(context: &Context) -> PaceResult<()> {
    let home = &context.dirs.data.path;
    let editor = editor_command(context.editor.as_deref(), true)?;
    let line = edit_roles_file(
        home,
        &context.terminal,
        |text| edit_in_editor(&editor, text, ".toml"),
        print_error,
    )?;
    print_lines([line])
}

/// Let the user change the roles file of `home` with `edit` until the
/// catalog reads it, and write it; the sentence that says so. A mistake is
/// shown with `report`, a mistake of the file as it is before the editor
/// opens, and `asker` offers to open the file again.
///
/// # Errors
///
/// If a mistake in another file of the catalog keeps it from reading, as
/// the roles file can't mend it; without a terminal, the roles file's
/// mistake or [`no_terminal_to_edit`]; if the user leaves a mistake in the
/// file, or the file can't be read or written.
fn edit_roles_file(
    home: &Path,
    asker: &dyn Ask,
    edit: impl FnMut(&str) -> PaceResult<String>,
    report: impl Fn(&PaceError),
) -> PaceResult<String> {
    let roles_file = home.join(ROLES_FILE);
    let text = match std::fs::read_to_string(&roles_file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => NEW_ROLES_FILE.to_owned(),
        Err(error) => {
            return Err(PaceError::with_source(
                ErrorKind::InputOutput,
                "cannot read {path}",
                error,
            )
            .attach_context("path", roles_file.display().to_string()));
        }
    };
    let mistake = match read_catalog_with(home, &roles_file, &text) {
        Err(mistake) if !points_at_file(&mistake, &roles_file) => return Err(mistake),
        checked => checked.err(),
    };
    if !asker.can_ask() {
        return Err(mistake.map_or_else(
            || no_terminal_to_edit(&roles_file),
            |mistake| {
                mistake
                    .append_guidance_line(NO_TERMINAL)
                    .attach_context("roles-file", roles_file.display().to_string())
            },
        ));
    }
    if let Some(mistake) = &mistake {
        report(mistake);
    }
    let (edited, catalog) = edit_until_checked(
        EditedFile {
            path: &roles_file,
            command: "pace role edit",
        },
        &text,
        asker,
        edit,
        |text| read_catalog_with(home, &roles_file, text),
        report,
    )?;
    write_catalog_file(&roles_file, &edited)?;
    Ok(format!(
        "Saved {}, with {}.",
        roles_file.display(),
        counted(catalog.roles().count(), "role")
    ))
}

/// The error of `pace role edit` without a terminal: it names the file to
/// edit by hand.
fn no_terminal_to_edit(roles_file: &Path) -> Box<PaceError> {
    PaceError::new(ErrorKind::InvalidInput, NO_TERMINAL)
        .attach_context("roles-file", roles_file.display().to_string())
        .attach_hint("edit the file by hand, or add a role with `pace role add`")
        .attach_exit_code(ExitCode::Usage)
}

/// The line that says `pace role edit` has no terminal for the editor; its
/// placeholder is `{roles-file}`.
const NO_TERMINAL: &str = "`pace role edit` has no terminal to open {roles-file} in an editor";

/// `text` as a role key; a usage error if it is none.
fn role_key(text: &str) -> PaceResult<RoleKey> {
    text.parse().map_err(usage)
}

#[cfg(test)]
mod tests {
    use jiff::{
        SignedDuration,
        civil::{Date, date},
    };
    use pace_core::RoleKind;

    use crate::prompt::Scripted;

    use super::*;

    fn role(key: &str, archived: Option<Archived>) -> Role {
        Role {
            id: RoleId::try_from(RandomIds.next_id()).unwrap(),
            key: key.parse().unwrap(),
            kind: RoleKind::Area,
            name: key.to_uppercase(),
            purpose: None,
            weekly_target: None,
            archived,
        }
    }

    fn ended(on: Date) -> Archived {
        Archived {
            on,
            reason: ArchiveReason::Ended,
        }
    }

    fn catalog(roles: Vec<Role>) -> Catalog {
        Catalog::new(roles, Vec::new(), Vec::new()).unwrap()
    }

    #[test]
    fn list_aligns_columns_and_leaves_archived_roles_out() {
        let mut work = role("work", None);
        work.weekly_target = Some(SignedDuration::from_mins(90));
        let catalog = catalog(vec![work, role("old", Some(ended(date(2026, 1, 1))))]);

        assert_eq!(
            role_lines(&catalog, false),
            [
                "key   kind  name  weekly target",
                "work  area  WORK  1h 30m"
            ]
        );
    }

    #[test]
    fn list_with_archived_shows_when_and_why() {
        let catalog = catalog(vec![role("old", Some(ended(date(2026, 1, 1))))]);

        assert_eq!(
            role_lines(&catalog, true),
            [
                "key  kind  name  weekly target  archived",
                "old  area  OLD                  2026-01-01, ended"
            ]
        );
    }

    #[test]
    fn list_of_archived_roles_only_points_to_archived_flag() {
        let catalog = catalog(vec![role("old", Some(ended(date(2026, 1, 1))))]);

        assert_eq!(
            role_lines(&catalog, false),
            ["Every role is archived; `pace role list --archived` shows them."]
        );
    }

    #[test]
    fn archive_of_archived_role_has_nothing_to_do() {
        let catalog = catalog(vec![role("old", Some(ended(date(2026, 1, 1))))]);

        let error = archive_edit(
            &catalog,
            Path::new("roles.toml"),
            &"old".parse().unwrap(),
            ended(date(2026, 11, 26)),
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "the role `old` is archived already, since 2026-01-01"
        );
        assert_eq!(error.exit_code(), ExitCode::NothingToDo);
    }

    #[test]
    fn archive_of_unknown_role_without_close_key_points_to_list() {
        let catalog = catalog(vec![role("work", None)]);

        let error = archive_edit(
            &catalog,
            Path::new("roles.toml"),
            &"health".parse().unwrap(),
            ended(date(2026, 11, 26)),
        )
        .unwrap_err();

        assert_eq!(error.hint(), Some("`pace role list` shows the roles"));
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

    const WORK: &str = "version = 1\n\n[work]\nid = \"0193a7c4-0000-7000-8000-000000000001\"\nkind = \"area\"\nname = \"Work\"\n";

    #[test]
    fn edit_of_missing_roles_file_starts_with_commented_example() {
        let home = pace_store::testing::scratch_dir().unwrap();
        let opened = std::cell::RefCell::new(String::new());
        let asker = Scripted::confirming([]);

        let line = edit_roles_file(
            home.path(),
            &asker,
            |text| {
                opened.borrow_mut().push_str(text);
                Ok(WORK.to_owned())
            },
            |_| {},
        )
        .unwrap();

        assert_eq!(opened.into_inner(), NEW_ROLES_FILE);
        assert_eq!(
            std::fs::read_to_string(home.path().join(ROLES_FILE)).unwrap(),
            WORK
        );
        assert!(line.ends_with("roles.toml, with 1 role."), "{line}");
    }

    #[test]
    fn edit_with_mistake_opens_again_and_writes_fixed_file() {
        let home = pace_store::testing::scratch_dir().unwrap();
        let broken = WORK.replace("kind = \"area\"", "kind = \"aera\"");
        let asker = Scripted::confirming([true]);
        let reported = std::cell::RefCell::new(Vec::new());

        _ = edit_roles_file(home.path(), &asker, saving(&[&broken, WORK]), |error| {
            reported.borrow_mut().push(error.to_string());
        })
        .unwrap();

        assert_eq!(reported.into_inner(), ["`aera` is not a role kind"]);
        assert_eq!(
            std::fs::read_to_string(home.path().join(ROLES_FILE)).unwrap(),
            WORK
        );
    }

    #[test]
    fn edit_of_broken_roles_file_shows_mistake_before_editor_opens() {
        let home = pace_store::testing::scratch_dir().unwrap();
        let broken = WORK.replace("kind = \"area\"", "kind = \"aera\"");
        std::fs::write(home.path().join(ROLES_FILE), &broken).unwrap();
        let events = std::cell::RefCell::new(Vec::new());

        _ = edit_roles_file(
            home.path(),
            &Scripted::confirming([]),
            |text| {
                events
                    .borrow_mut()
                    .push(format!("opened {}", text == broken));
                Ok(WORK.to_owned())
            },
            |error| events.borrow_mut().push(error.to_string()),
        )
        .unwrap();

        assert_eq!(
            events.into_inner(),
            ["`aera` is not a role kind", "opened true"]
        );
        assert_eq!(
            std::fs::read_to_string(home.path().join(ROLES_FILE)).unwrap(),
            WORK
        );
    }

    #[test]
    fn edit_stops_at_mistake_in_other_catalog_file_before_editor_opens() {
        let home = pace_store::testing::scratch_dir().unwrap();
        std::fs::write(home.path().join(ROLES_FILE), WORK).unwrap();
        let clients_file = home.path().join("clients.toml");
        std::fs::write(&clients_file, "version = 1\n\n[acme]\nname = 1\n").unwrap();
        let reported = std::cell::RefCell::new(Vec::new());

        let error = edit_roles_file(
            home.path(),
            &Scripted::confirming([]),
            saving(&[]),
            |error| reported.borrow_mut().push(error.to_string()),
        )
        .unwrap_err();

        assert!(points_at_file(&error, &clients_file), "{error:?}");
        assert_eq!(reported.into_inner(), Vec::<String>::new());
    }

    #[test]
    fn edit_without_terminal_shows_mistake_of_roles_file() {
        let home = pace_store::testing::scratch_dir().unwrap();
        let roles_file = home.path().join(ROLES_FILE);
        std::fs::write(
            &roles_file,
            WORK.replace("kind = \"area\"", "kind = \"aera\""),
        )
        .unwrap();

        let error = edit_roles_file(
            home.path(),
            &Scripted::without_terminal(),
            saving(&[]),
            |_| {},
        )
        .unwrap_err();

        assert!(points_at_file(&error, &roles_file), "{error:?}");
        assert_eq!(
            error.to_string(),
            format!(
                "`aera` is not a role kind\n`pace role edit` has no terminal to open {} in an editor",
                roles_file.display()
            )
        );
        assert_ne!(error.hint(), no_terminal_to_edit(&roles_file).hint());
    }

    #[test]
    fn edit_without_terminal_names_file_to_edit_by_hand() {
        let home = pace_store::testing::scratch_dir().unwrap();
        std::fs::write(home.path().join(ROLES_FILE), WORK).unwrap();

        let error = edit_roles_file(
            home.path(),
            &Scripted::without_terminal(),
            saving(&[]),
            |_| {},
        )
        .unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(
            std::fs::read_to_string(home.path().join(ROLES_FILE)).unwrap(),
            WORK
        );
    }

    #[test]
    fn edit_with_mistake_left_writes_nothing() {
        let home = pace_store::testing::scratch_dir().unwrap();
        let asker = Scripted::confirming([false]);

        let error = edit_roles_file(
            home.path(),
            &asker,
            saving(&["version = 1\n[work]\n"]),
            |_| {},
        )
        .unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::NothingToDo);
        assert!(!home.path().join(ROLES_FILE).exists());
    }

    #[test]
    fn roles_file_that_is_directory_cannot_be_read_for_edit() {
        let home = pace_store::testing::scratch_dir().unwrap();
        std::fs::create_dir(home.path().join(ROLES_FILE)).unwrap();

        let error = edit_roles_file(home.path(), &Scripted::confirming([]), saving(&[]), |_| {})
            .unwrap_err();

        assert_eq!(error.kind(), ErrorKind::InputOutput);
    }
}
