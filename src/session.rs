//! What a tracking command works with: the tracker over the files backend,
//! with this machine's device ID, the clock and the settings.

use std::{ffi::OsStr, path::PathBuf, sync::Arc};

use jiff::tz::TimeZone;
use pace_core::{Command, Device, DeviceId, Ids, Outcome, Policy};
use pace_error::PaceResult;
use pace_store::{EventStore, FilesStore, Resplit, Split, Tracker, device_id, existing_device_id};
use pace_time::TimeRange;

use crate::clock::clock_for_run;
use crate::{
    application::{Context, print_error},
    clock::RandomIds,
};

/// The readable name of this device: the `hostname`, or `device` if it
/// is empty.
#[must_use]
pub fn device_name(hostname: &OsStr) -> String {
    let name = hostname.to_string_lossy();
    if name.is_empty() {
        "device".to_owned()
    } else {
        name.into_owned()
    }
}

/// The settings the rules of a tracking command depend on:
/// `holds.default-source`, with the other fields at their defaults.
fn tracking_policy(context: &Context) -> Policy {
    Policy {
        default_hold_source: context.settings.settings.default_hold_source,
        ..Policy::default()
    }
}

/// A tracker for this run.
#[derive(Debug)]
pub struct Session {
    tracker: Tracker<FilesStore>,
    device: DeviceId,
    data: PathBuf,
    data_existed: bool,
}

impl Session {
    /// Open the store of this machine; creates the device ID on first use.
    ///
    /// # Errors
    ///
    /// If the clock, the device ID or the store can't be set up.
    pub fn open(context: &Context) -> PaceResult<Self> {
        Self::open_with_policy(context, tracking_policy(context))
    }

    /// Open the store of this machine like [`Session::open`], with the
    /// rules reporting in `reports_time_zone`: the zone that
    /// `reports.time-zone` names, which marking a bill needs. Only that
    /// command resolves the setting, so a zone the system can't tell
    /// stops no other command.
    ///
    /// # Errors
    ///
    /// If the clock, the device ID or the store can't be set up.
    pub fn open_reporting_in(context: &Context, reports_time_zone: TimeZone) -> PaceResult<Self> {
        let policy = Policy {
            reports_time_zone,
            ..tracking_policy(context)
        };
        Self::open_with_policy(context, policy)
    }

    fn open_with_policy(context: &Context, policy: Policy) -> PaceResult<Self> {
        let ids: Arc<dyn Ids> = Arc::new(RandomIds);
        let device = device_id(&context.dirs.machine.path, &*ids)?.id;
        Self::with_device(context, ids, device, policy)
    }

    /// Open the store of this machine without creating its device ID: on
    /// the first run, the session takes an ID that it never saves. So a
    /// query creates no device ID; a command that writes needs
    /// [`Session::open`]. A read still saves the index cache `index.json`
    /// when the event files changed, and creates the machine directory for
    /// it.
    ///
    /// # Errors
    ///
    /// If the clock, the device ID or the store can't be set up.
    pub fn open_existing(context: &Context) -> PaceResult<Self> {
        let ids: Arc<dyn Ids> = Arc::new(RandomIds);
        let device = match existing_device_id(&context.dirs.machine.path)? {
            Some(device) => device,
            None => DeviceId::try_from(ids.next_id())?,
        };
        Self::with_device(context, ids, device, tracking_policy(context))
    }

    fn with_device(
        context: &Context,
        ids: Arc<dyn Ids>,
        device: DeviceId,
        policy: Policy,
    ) -> PaceResult<Self> {
        let clock = clock_for_run(
            &context.settings.settings.time_zone,
            context.test_now.clone(),
        )?;
        let data = context.dirs.data.path.clone();
        let settings = &context.settings.settings;
        let store = FilesStore::new(
            data.clone(),
            context.dirs.machine.path.clone(),
            device,
            context.host.clone(),
            clock.clone(),
            settings.split,
        );
        Ok(Self {
            tracker: Tracker::new(store, clock, ids, policy, context.catalog.clone(), device),
            device,
            data_existed: data.exists(),
            data,
        })
    }

    /// This machine's device ID.
    #[must_use]
    pub const fn device(&self) -> DeviceId {
        self.device
    }

    /// Now, by the session's clock.
    #[must_use]
    pub fn now(&self) -> jiff::ZonedDateTime {
        self.tracker.now()
    }

    /// Run `command` through the tracker. What went wrong after the
    /// command was recorded, such as a failed cleaning of forgotten
    /// entries, is printed as a warning.
    ///
    /// # Errors
    ///
    /// If the command doesn't fit the state, or the store fails.
    pub fn run(&self, command: &Command) -> PaceResult<Outcome> {
        let outcome = self.tracker.run(command)?;
        for warning in &outcome.warnings {
            print_error(warning);
        }
        Ok(outcome)
    }

    /// The state of the entries that overlap `range`, and of the open ones.
    ///
    /// # Errors
    ///
    /// If the store fails.
    pub fn state(&self, range: &TimeRange) -> PaceResult<pace_core::State> {
        self.tracker.state(range)
    }

    /// The devices that have written to the store, with their names.
    ///
    /// # Errors
    ///
    /// If a device's `device.toml` can't be read.
    pub fn devices(&self) -> PaceResult<Vec<Device>> {
        self.tracker.store().devices()
    }

    /// Rewrite this device's event files split by `split`.
    ///
    /// # Errors
    ///
    /// If a file can't be read, written or removed.
    pub fn resplit(&self, split: Split) -> PaceResult<Resplit> {
        self.tracker.store().resplit(split)
    }

    /// A note for the first run that created the data directory: where
    /// the data is, and `pace setup`, offered, never required.
    #[must_use]
    pub fn first_use_note(&self) -> Option<String> {
        (!self.data_existed && self.data.exists()).then(|| {
            format!(
                "pace keeps its data in {}. If you like, `pace setup` walks you through \
                 the settings, roles and clients.",
                self.data.display()
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_is_named_after_host() {
        assert_eq!(device_name(OsStr::new("laptop")), "laptop");
    }

    #[test]
    fn device_without_hostname_is_named_device() {
        assert_eq!(device_name(OsStr::new("")), "device");
    }
}
