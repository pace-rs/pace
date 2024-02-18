use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use chrono::{NaiveDate, NaiveDateTime};

use crate::{
    domain::{
        activity::{Activity, ActivityGuid},
        activity_log::ActivityLog,
        filter::{ActivityFilter, FilteredActivities},
    },
    error::{PaceErrorKind, PaceOptResult, PaceResult},
    storage::{
        in_memory::InMemoryActivityStorage, ActivityQuerying, ActivityReadOps,
        ActivityStateManagement, ActivityStorage, ActivityWriteOps, SyncStorage,
    },
};

/// In-memory backed TOML activity storage
///
/// This storage is backed by an in-memory cache and a TOML file on disk for persistence.
pub struct TomlActivityStorage {
    /// The in-memory cache
    cache: InMemoryActivityStorage,

    /// The path to the TOML file
    path: PathBuf,
}

impl SyncStorage for TomlActivityStorage {
    fn sync(&self) -> PaceResult<()> {
        self.sync_to_file()
    }
}

impl TomlActivityStorage {
    /// Create a new `TomlActivityStorage`
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the TOML file
    ///
    /// # Errors
    ///
    /// Returns an error if the path is not a valid file path.
    ///
    /// # Returns
    ///
    /// Returns a new `TomlActivityStorage`
    pub fn new(path: impl AsRef<Path>) -> PaceResult<Self> {
        let mut storage = Self {
            cache: InMemoryActivityStorage::new(),
            path: path.as_ref().to_path_buf(),
        };

        storage.load()?;

        Ok(storage)
    }

    /// Load the TOML file into the in-memory cache
    ///
    /// This will read the TOML file from disk and load it into the in-memory cache
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or the data cannot be deserialized
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the data is loaded successfully
    fn load(&mut self) -> PaceResult<()> {
        let data = std::fs::read_to_string(&self.path)?;
        self.cache = InMemoryActivityStorage::from(toml::from_str::<ActivityLog>(&data)?);

        Ok(())
    }

    /// Sync the in-memory cache to the TOML file
    ///
    /// This will write the in-memory cache to the TOML file on disk
    ///
    /// # Errors
    ///
    /// Returns an error if the cache cannot be written to the file
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the cache is written successfully
    pub fn sync_to_file(&self) -> PaceResult<()> {
        let data = toml::to_string(&self.cache.get_activity_log()?)?;
        std::fs::write(&self.path, data)?;
        Ok(())
    }
}

impl ActivityStorage for TomlActivityStorage {
    fn setup_storage(&self) -> PaceResult<()> {
        if !self.path.exists() {
            fs::create_dir_all(
                self.path
                    .parent()
                    .ok_or(PaceErrorKind::ParentDirNotFound(self.path.clone()))?,
            )?;

            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&self.path)?;

            file.write_all(b"")?;
        }
        Ok(())
    }
}

impl ActivityReadOps for TomlActivityStorage {
    fn read_activity(&self, activity_id: ActivityGuid) -> PaceResult<Activity> {
        self.cache.read_activity(activity_id)
    }

    fn list_activities(&self, filter: ActivityFilter) -> PaceOptResult<FilteredActivities> {
        self.cache.list_activities(filter)
    }
}

impl ActivityStateManagement for TomlActivityStorage {
    fn end_all_unfinished_activities(
        &self,
        time: Option<NaiveDateTime>,
    ) -> PaceOptResult<Vec<Activity>> {
        self.cache.end_all_unfinished_activities(time)
    }

    fn end_last_unfinished_activity(&self, time: Option<NaiveDateTime>) -> PaceOptResult<Activity> {
        self.cache.end_last_unfinished_activity(time)
    }

    fn end_single_activity(
        &self,
        activity_id: ActivityGuid,
        end_time: Option<NaiveDateTime>,
    ) -> PaceResult<ActivityGuid> {
        self.cache.end_single_activity(activity_id, end_time)
    }

    fn hold_last_unfinished_activity(
        &self,
        end_time: Option<NaiveDateTime>,
    ) -> PaceOptResult<Activity> {
        self.cache.hold_last_unfinished_activity(end_time)
    }
}

impl ActivityWriteOps for TomlActivityStorage {
    fn create_activity(&self, activity: Activity) -> PaceResult<ActivityGuid> {
        self.cache.create_activity(activity)
    }

    fn update_activity(
        &self,
        activity_id: ActivityGuid,
        activity: Activity,
    ) -> PaceResult<Activity> {
        self.cache.update_activity(activity_id, activity)
    }

    fn delete_activity(&self, activity_id: ActivityGuid) -> PaceResult<Activity> {
        self.cache.delete_activity(activity_id)
    }
}

impl ActivityQuerying for TomlActivityStorage {
    fn list_activities_by_id(&self) -> PaceOptResult<BTreeMap<ActivityGuid, Activity>> {
        todo!("Implement `activities_by_id` for `TomlActivityStorage`")
    }

    fn find_activities_in_date_range(
        &self,
        _start_date: NaiveDate,
        _end_date: NaiveDate,
    ) -> PaceResult<ActivityLog> {
        todo!("Implement `find_activities_in_date_range` for `TomlActivityStorage`")
    }
}
