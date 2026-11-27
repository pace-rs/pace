//! What a tracking command works with: the tracker over the files backend,
//! with this machine's device ID, the clock and the settings.

use std::{path::PathBuf, sync::Arc};

use abscissa_core::Application;
use abscissa_tokio::TokioComponent;
use pace_core::{Command, DeviceId, Ids, Outcome, Policy, Tracker};
use pace_error::{ErrorKind, PaceError, PaceResult};
use pace_store::{FilesStore, Resplit, Split, device_id, existing_device_id};
use pace_time::TimeRange;

use crate::clock::clock;
use crate::{
    application::{Context, PACE_APP, print_error},
    clock::{RandomIds, TEST_NOW},
};

/// A tracker for this run.
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
        let ids: Arc<dyn Ids> = Arc::new(RandomIds);
        let device = device_id(&context.dirs.machine.path, &*ids)?.id;
        Self::with_device(context, ids, device)
    }

    /// Open the store of this machine without creating its device ID: on
    /// the first run, the session takes an ID that it never saves. So a
    /// query writes nothing; a command that writes needs [`Session::open`].
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
        Self::with_device(context, ids, device)
    }

    fn with_device(context: &Context, ids: Arc<dyn Ids>, device: DeviceId) -> PaceResult<Self> {
        let clock = clock(
            &context.settings.settings.time_zone,
            std::env::var_os(TEST_NOW),
        )?;
        let data = context.dirs.data.path.clone();
        let settings = &context.settings.settings;
        let store = FilesStore::new(
            data.clone(),
            context.dirs.machine.path.clone(),
            device,
            clock.clone(),
            settings.split,
        );
        let policy = Policy {
            default_hold_source: settings.default_hold_source,
        };
        Ok(Self {
            tracker: Tracker::new(store, clock, ids, policy, device),
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
        let outcome = block_on(self.tracker.run(command))??;
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
        block_on(self.tracker.state(range))?
    }

    /// Rewrite this device's event files split by `split`.
    ///
    /// # Errors
    ///
    /// If a file can't be read, written or removed.
    pub fn resplit(&self, split: Split) -> PaceResult<Resplit> {
        self.tracker.store().resplit(split)
    }

    /// A note for the first run that created the data directory.
    #[must_use]
    pub fn first_use_note(&self) -> Option<String> {
        (!self.data_existed && self.data.exists())
            .then(|| format!("pace keeps its data in {}.", self.data.display()))
    }
}

/// Run a future on the application's one runtime, borrowed from its
/// component, so a command can run several futures.
fn block_on<F: Future>(future: F) -> PaceResult<F::Output> {
    let components = PACE_APP.state().components();
    let runtime = components
        .get_downcast_ref::<TokioComponent>()
        .ok_or_else(|| {
            PaceError::new(ErrorKind::Internal, "the runtime component is missing").ask_report()
        })?
        .runtime()
        .map_err(|error| {
            PaceError::with_source(ErrorKind::Internal, "the runtime is gone", error).ask_report()
        })?;
    let output = runtime.block_on(future);
    drop(components);
    Ok(output)
}
