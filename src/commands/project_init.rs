//! `pace project init`: make the working directory a project, with the
//! record and the manifest that links the directory to it.

use abscissa_core::{Command, Runnable};
use pace_core::{Ids, ProjectId, RoleId};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_store::field_edit::write_field_edits;

use crate::application::{Context, PACE_APP, exit_on_error};
use crate::clock::RandomIds;
use crate::commands::project::{billable_flag, defaults_line, described};
use crate::output::listed;
use crate::print::print_lines;
use crate::project::{Records, manifest_in};
use crate::questions::add_project::ProjectFlags;
use crate::questions::init_project::{
    InitFlags, InitInputs, InitProject, InitProjectFlow, ManifestChange,
};
use crate::questions::{Asked, ask_flow};

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
    lines.extend(if made.record_added {
        defaults_line(&asked.defaults, &record.key)
    } else {
        found_defaults_line(&asked.defaults)
    });
    lines
}

/// The line that names the `defaults` pace took for a record it found:
/// the key, which only `--key` gives otherwise.
fn found_defaults_line(defaults: &[String]) -> Option<String> {
    (!defaults.is_empty()).then(|| {
        format!(
            "pace took these defaults: {}; `pace project init --key` names another project.",
            listed(defaults)
        )
    })
}
