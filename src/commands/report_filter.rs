//! The filters that `reflect` and `export` share, `--project`, `--role`,
//! `--client`, `--tag` and `--device`, resolved to a `ReportFilter`; and
//! their period, from its name or from `--from` and `--to`.

use std::collections::BTreeSet;

use jiff::civil::Date;
use pace_core::{
    Catalog, ClientKey, Device, DeviceId, ProjectKey, ProjectRef, ReportFilter, RoleId, RoleKey,
    State,
};
use pace_error::{ErrorKind, PaceError, PaceResult, closest_name};
use pace_time::TimeFrame;

use crate::application::Context;
use crate::commands::{client::unknown_client, role::known_role, tracking::tags};
use crate::output::usage;
use crate::project::Reason;
use crate::session::Session;

/// The filters of a report or an export; several of one flag combine
/// with "or", different flags with "and".
#[derive(clap::Args, Debug)]
pub struct ReportFilterArgs {
    // The id `project` takes the place of the global `--project` here, so
    // the flag can repeat.
    #[arg(
        id = "project",
        long = "project",
        value_name = "KEY",
        help = "Only the entries of this project; repeat for more"
    )]
    pub(crate) projects: Vec<String>,

    #[arg(
        long = "role",
        value_name = "KEY",
        help = "Only the entries whose time goes to this role; repeat for more"
    )]
    roles: Vec<String>,

    #[arg(
        long = "client",
        value_name = "KEY",
        help = "Only the entries of this client's projects; repeat for more"
    )]
    clients: Vec<String>,

    #[arg(
        long = "tag",
        value_name = "TAG",
        help = "Only the entries with this tag; repeat for more"
    )]
    tags: Vec<String>,

    #[arg(
        long = "device",
        value_name = "DEVICE",
        help = "Only the entries of this device: its ID, the last characters of its ID, \
                or its name; repeat for more"
    )]
    devices: Vec<String>,
}

impl ReportFilterArgs {
    /// The filters, with the keys and names of the flags resolved. A
    /// global `pace --project KEY reflect` filters as `--project` does;
    /// beside the command's own `--project`, clap keeps only its keys.
    pub(crate) fn filter(
        &self,
        context: &Context,
        session: &Session,
        state: &State,
    ) -> PaceResult<ReportFilter> {
        let catalog = &context.catalog;
        let mut keys = self.projects.clone();
        if let Some(project) = context
            .project
            .as_ref()
            .filter(|project| project.reason == Reason::Flag)
        {
            keys.push(project.key.to_string());
        }
        let devices = if self.devices.is_empty() {
            BTreeSet::new()
        } else {
            device_filter(&self.devices, &session.devices()?)?
        };
        Ok(ReportFilter {
            projects: project_filter(&keys, catalog, state)?,
            roles: role_filter(&self.roles, catalog)?,
            clients: client_filter(&self.clients, catalog)?,
            tags: tags(&self.tags)?,
            devices,
        })
    }
}

/// The period of `period`, or of `--from` and `--to`; `default` without
/// any of them.
///
/// # Errors
///
/// A usage error for a name that is no period.
pub(crate) fn period_frame(
    period: Option<&str>,
    from: Option<Date>,
    to: Option<Date>,
    default: TimeFrame,
) -> PaceResult<TimeFrame> {
    match (period, from, to) {
        (Some(period), ..) => period.parse().map_err(usage),
        (None, None, None) => Ok(default),
        (None, from, to) => Ok(TimeFrame::Between { from, to }),
    }
}

/// The projects of `--project`: a key of a record by its ID, else a key
/// that an entry of `state` names without a record.
///
/// # Errors
///
/// A usage error for a key that neither a record nor an entry of the
/// period has: it names the closest of them.
fn project_filter(
    keys: &[String],
    catalog: &Catalog,
    state: &State,
) -> PaceResult<BTreeSet<ProjectRef>> {
    let named: BTreeSet<&ProjectKey> = state
        .entries()
        .filter_map(|entry| match &entry.project {
            Some(ProjectRef::Key(key)) => Some(key),
            _ => None,
        })
        .collect();
    keys.iter()
        .map(|text| {
            let key: ProjectKey = text.parse().map_err(usage)?;
            if let Some(record) = catalog.project(&key) {
                return Ok(ProjectRef::Id(record.id));
            }
            if named.contains(&key) {
                return Ok(ProjectRef::Key(key));
            }
            let known = catalog
                .projects()
                .map(|record| record.key.as_str())
                .chain(named.iter().map(|key| key.as_str()));
            let hint = closest_name(key.as_str(), known).map_or_else(
                || "`pace project list` shows the projects".to_owned(),
                |closest| format!("did you mean `{closest}`?"),
            );
            Err(usage(
                PaceError::new(
                    ErrorKind::InvalidInput,
                    "pace has no record of the project `{key}`, and no entry of the period names it",
                )
                .attach_context("key", key.to_string())
                .attach_hint(hint),
            ))
        })
        .collect()
}

/// The roles of `--role`, by their IDs.
///
/// # Errors
///
/// A usage error for a key that is no role: it names the closest key.
fn role_filter(keys: &[String], catalog: &Catalog) -> PaceResult<BTreeSet<RoleId>> {
    keys.iter()
        .map(|text| {
            let key: RoleKey = text.parse().map_err(usage)?;
            Ok(known_role(catalog, &key).map_err(usage)?.id)
        })
        .collect()
}

/// The clients of `--client`.
///
/// # Errors
///
/// A usage error for a key that is no client: it names the closest key.
fn client_filter(keys: &[String], catalog: &Catalog) -> PaceResult<BTreeSet<ClientKey>> {
    keys.iter()
        .map(|text| {
            let key: ClientKey = text.parse().map_err(usage)?;
            if catalog.client(&key).is_none() {
                let known = catalog.clients().map(|client| client.key.as_str());
                return Err(usage(unknown_client(&key, known, None)));
            }
            Ok(key)
        })
        .collect()
}

/// The fewest last characters of a device ID that `--device` takes.
const SHORT_DEVICE_ID: usize = 4;

/// The devices of `--device`, among the `known` ones: a whole ID, the
/// last characters of one ID (at least four), or a name, which takes every
/// device of that name.
///
/// # Errors
///
/// A usage error for a text that names no device, or whose characters end
/// the IDs of two devices.
fn device_filter(texts: &[String], known: &[Device]) -> PaceResult<BTreeSet<DeviceId>> {
    let mut devices = BTreeSet::new();
    for text in texts {
        devices.extend(named_devices(text, known)?);
    }
    Ok(devices)
}

fn named_devices(text: &str, known: &[Device]) -> PaceResult<Vec<DeviceId>> {
    if let Ok(id) = text.parse::<DeviceId>() {
        return if known.iter().any(|device| device.id == id) {
            Ok(vec![id])
        } else {
            Err(usage(unknown_device(text, known)))
        };
    }
    let by_name: Vec<DeviceId> = known
        .iter()
        .filter(|device| device.name == text)
        .map(|device| device.id)
        .collect();
    if !by_name.is_empty() {
        return Ok(by_name);
    }
    let lowercase = text.to_ascii_lowercase();
    let by_end: Vec<DeviceId> = known
        .iter()
        .filter(|device| {
            lowercase.len() >= SHORT_DEVICE_ID
                && device.id.uuid().simple().to_string().ends_with(&lowercase)
        })
        .map(|device| device.id)
        .collect();
    match by_end.as_slice() {
        [id] => Ok(vec![*id]),
        [] => Err(usage(unknown_device(text, known))),
        _ => Err(usage(
            PaceError::new(
                ErrorKind::InvalidInput,
                "`{device}` ends the IDs of {count} devices",
            )
            .attach_context("device", text)
            .attach_context("count", by_end.len().to_string())
            .attach_hint("give more of the ID's last characters, or the whole ID"),
        )),
    }
}

fn unknown_device(text: &str, known: &[Device]) -> Box<PaceError> {
    let names: Vec<&str> = known.iter().map(|device| device.name.as_str()).collect();
    let hint = closest_name(text, names.iter().copied()).map_or_else(
        || {
            if names.is_empty() {
                "no device has written to the store yet".to_owned()
            } else {
                format!("the devices are {}", names.join(", "))
            }
        },
        |closest| format!("did you mean `{closest}`?"),
    );
    PaceError::new(
        ErrorKind::InvalidInput,
        "no device has the name or ID `{device}`",
    )
    .attach_context("device", text)
    .attach_hint(hint)
}

#[cfg(test)]
mod tests {
    use jiff::ZonedDateTime;
    use pace_error::ExitCode;

    use super::*;

    const LAPTOP: &str = "0193a7c4-0000-7000-8000-00000001ab12";
    const DESKTOP: &str = "0193a7c4-0000-7000-8000-00000002ab12";

    fn device(id: &str, name: &str) -> Device {
        Device {
            id: id.parse().unwrap(),
            name: name.to_owned(),
            created: "2026-11-01T09:00[Europe/Berlin]"
                .parse::<ZonedDateTime>()
                .unwrap(),
        }
    }

    fn devices() -> Vec<Device> {
        vec![device(LAPTOP, "laptop"), device(DESKTOP, "desktop")]
    }

    #[test]
    fn device_is_found_by_id_end_or_name() {
        let ids = |text: &str| device_filter(&[text.to_owned()], &devices()).unwrap();
        let laptop: DeviceId = LAPTOP.parse().unwrap();

        assert_eq!(ids(LAPTOP), BTreeSet::from([laptop]));
        assert_eq!(ids("1ab12"), BTreeSet::from([laptop]));
        assert_eq!(ids("1AB12"), BTreeSet::from([laptop]));
        assert_eq!(ids("laptop"), BTreeSet::from([laptop]));
    }

    #[test]
    fn whole_id_of_no_device_is_refused() {
        let error = device_filter(
            &["0193a7c4-0000-7000-8000-00000003ab12".to_owned()],
            &devices(),
        )
        .unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert!(
            error
                .to_string()
                .contains("no device has the name or ID `0193a7c4-0000-7000-8000-00000003ab12`"),
            "{error}"
        );
    }

    #[test]
    fn name_of_two_devices_takes_both() {
        let known = vec![device(LAPTOP, "laptop"), device(DESKTOP, "laptop")];

        let ids = device_filter(&["laptop".to_owned()], &known).unwrap();

        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn end_of_two_device_ids_is_refused() {
        let error = device_filter(&["ab12".to_owned()], &devices()).unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert!(
            error.to_string().contains("ends the IDs of 2 devices"),
            "{error}"
        );
    }

    #[test]
    fn unknown_device_names_closest_name() {
        let error = device_filter(&["laptpo".to_owned()], &devices()).unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(error.hint(), Some("did you mean `laptop`?"));
    }

    #[test]
    fn unknown_device_without_close_name_lists_devices() {
        let error = device_filter(&["zz".to_owned()], &devices()).unwrap_err();

        assert_eq!(error.hint(), Some("the devices are laptop, desktop"));
        let none = device_filter(&["zz".to_owned()], &[]).unwrap_err();
        assert_eq!(none.hint(), Some("no device has written to the store yet"));
    }

    #[test]
    fn project_key_without_record_or_entry_names_closest_record() {
        let catalog = Catalog::new(
            Vec::new(),
            Vec::new(),
            vec![pace_core::ProjectRecord {
                id: "0193a7c4-0000-7000-8000-000000000001".parse().unwrap(),
                key: "webshop".parse().unwrap(),
                name: "Webshop".to_owned(),
                client: None,
                role: None,
                billable: false,
                budget: None,
                starts_on: None,
                ends_on: None,
                done_when: None,
                allow_overlap: false,
                archived: None,
            }],
        )
        .unwrap();

        let error =
            project_filter(&["websop".to_owned()], &catalog, &State::default()).unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(error.hint(), Some("did you mean `webshop`?"));
    }
}
