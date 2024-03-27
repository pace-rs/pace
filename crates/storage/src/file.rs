use std::{
    collections::BTreeMap,
    fs::{create_dir_all, write, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use pace_time::{date::PaceDate, duration::PaceDurationRange, time_range::TimeRangeOptions};

use pace_core::prelude::{
    Activity, ActivityFilterKind, ActivityGuid, ActivityItem, ActivityKind, ActivityLog,
    ActivityQuerying, ActivityReadOps, ActivityStateManagement, ActivityStatusKind,
    ActivityStorage, ActivityWriteOps, DeleteOptions, EndOptions, FilteredActivities, HoldOptions,
    KeywordOptions, PaceStorageOptResult, PaceStorageResult, ResumeOptions, SyncStorage,
    UpdateOptions,
};

use crate::{error::TomlFileStorageErrorKind, in_memory::InMemoryActivityStorage};

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
    fn sync(&self) -> PaceStorageResult<()> {
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
    pub fn new(path: impl AsRef<Path>) -> PaceStorageResult<Self> {
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
    #[tracing::instrument(skip(self))]
    fn load(&mut self) -> PaceStorageResult<()> {
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
    #[tracing::instrument(skip(self))]
    pub fn sync_to_file(&self) -> PaceStorageResult<()> {
        let data = toml::to_string(&self.cache.get_activity_log())?;
        write(&self.path, data)?;
        Ok(())
    }
}

impl ActivityStorage for TomlActivityStorage {
    #[tracing::instrument(skip(self))]
    fn setup(&self) -> PaceStorageResult<()> {
        if !self.path.exists() {
            create_dir_all(
                self.path.parent().ok_or_else(|| {
                    TomlFileStorageErrorKind::ParentDirNotFound(self.path.clone())
                })?,
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

    #[tracing::instrument(skip(self))]
    fn teardown(&self) -> PaceStorageResult<()> {
        self.sync_to_file()
    }

    #[tracing::instrument(skip(self))]
    fn identify(&self) -> String {
        format!("TOML file storage: {}", self.path.display())
    }
}

impl ActivityReadOps for TomlActivityStorage {
    #[tracing::instrument(skip(self))]
    fn read_activity(&self, activity_id: ActivityGuid) -> PaceStorageResult<ActivityItem> {
        self.cache.read_activity(activity_id)
    }

    #[tracing::instrument(skip(self))]
    fn list_activities(
        &self,
        filter: ActivityFilterKind,
    ) -> PaceStorageOptResult<FilteredActivities> {
        self.cache.list_activities(filter)
    }
}

impl ActivityStateManagement for TomlActivityStorage {
    #[tracing::instrument(skip(self))]
    fn end_all_activities(&self, end_opts: EndOptions) -> PaceStorageOptResult<Vec<ActivityItem>> {
        self.cache.end_all_activities(end_opts)
    }

    #[tracing::instrument(skip(self))]
    fn end_last_unfinished_activity(
        &self,
        end_opts: EndOptions,
    ) -> PaceStorageOptResult<ActivityItem> {
        self.cache.end_last_unfinished_activity(end_opts)
    }

    #[tracing::instrument(skip(self))]
    fn end_activity(
        &self,
        activity_id: ActivityGuid,
        end_opts: EndOptions,
    ) -> PaceStorageResult<ActivityItem> {
        self.cache.end_activity(activity_id, end_opts)
    }

    #[tracing::instrument(skip(self))]
    fn hold_most_recent_active_activity(
        &self,
        hold_opts: HoldOptions,
    ) -> PaceStorageOptResult<ActivityItem> {
        self.cache.hold_most_recent_active_activity(hold_opts)
    }

    #[tracing::instrument(skip(self))]
    fn end_all_active_intermissions(
        &self,
        end_opts: EndOptions,
    ) -> PaceStorageOptResult<Vec<ActivityGuid>> {
        self.cache.end_all_active_intermissions(end_opts)
    }

    #[tracing::instrument(skip(self))]
    fn resume_activity(
        &self,
        activity_id: ActivityGuid,
        resume_opts: ResumeOptions,
    ) -> PaceStorageResult<ActivityItem> {
        self.cache.resume_activity(activity_id, resume_opts)
    }

    #[tracing::instrument(skip(self))]
    fn hold_activity(
        &self,
        activity_id: ActivityGuid,
        hold_opts: HoldOptions,
    ) -> PaceStorageResult<ActivityItem> {
        self.cache.hold_activity(activity_id, hold_opts)
    }

    #[tracing::instrument(skip(self))]
    fn resume_most_recent_activity(
        &self,
        resume_opts: ResumeOptions,
    ) -> PaceStorageOptResult<ActivityItem> {
        self.cache.resume_most_recent_activity(resume_opts)
    }
}

impl ActivityWriteOps for TomlActivityStorage {
    #[tracing::instrument(skip(self))]
    fn create_activity(&self, activity: Activity) -> PaceStorageResult<ActivityItem> {
        self.cache.create_activity(activity)
    }

    #[tracing::instrument(skip(self))]
    fn update_activity(
        &self,
        activity_id: ActivityGuid,
        updated_activity: Activity,
        update_opts: UpdateOptions,
    ) -> PaceStorageResult<ActivityItem> {
        self.cache
            .update_activity(activity_id, updated_activity, update_opts)
    }

    #[tracing::instrument(skip(self))]
    fn delete_activity(
        &self,
        activity_id: ActivityGuid,
        delete_opts: DeleteOptions,
    ) -> PaceStorageResult<ActivityItem> {
        self.cache.delete_activity(activity_id, delete_opts)
    }
}

impl ActivityQuerying for TomlActivityStorage {
    #[tracing::instrument(skip(self))]
    fn list_activities_by_id(&self) -> PaceStorageOptResult<BTreeMap<ActivityGuid, Activity>> {
        self.cache.list_activities_by_id()
    }

    #[tracing::instrument(skip(self))]
    fn most_recent_active_activity(&self) -> PaceStorageOptResult<ActivityItem> {
        self.cache.most_recent_active_activity()
    }

    #[tracing::instrument(skip(self))]
    fn group_activities_by_duration_range(
        &self,
    ) -> PaceStorageOptResult<BTreeMap<PaceDurationRange, Vec<ActivityItem>>> {
        self.cache.group_activities_by_duration_range()
    }

    #[tracing::instrument(skip(self))]
    fn group_activities_by_start_date(
        &self,
    ) -> PaceStorageOptResult<BTreeMap<PaceDate, Vec<ActivityItem>>> {
        self.cache.group_activities_by_start_date()
    }

    #[tracing::instrument(skip(self))]
    fn list_activities_with_intermissions(
        &self,
    ) -> PaceStorageOptResult<BTreeMap<ActivityGuid, Vec<ActivityItem>>> {
        self.cache.list_activities_with_intermissions()
    }

    #[tracing::instrument(skip(self))]
    fn group_activities_by_keywords(
        &self,
        keyword_opts: KeywordOptions,
    ) -> PaceStorageOptResult<BTreeMap<String, Vec<ActivityItem>>> {
        self.cache.group_activities_by_keywords(keyword_opts)
    }

    #[tracing::instrument(skip(self))]
    fn group_activities_by_kind(
        &self,
    ) -> PaceStorageOptResult<BTreeMap<ActivityKind, Vec<ActivityItem>>> {
        self.cache.group_activities_by_kind()
    }

    #[tracing::instrument(skip(self))]
    fn list_activities_by_time_range(
        &self,
        time_range_opts: TimeRangeOptions,
    ) -> PaceStorageOptResult<Vec<ActivityGuid>> {
        self.cache.list_activities_by_time_range(time_range_opts)
    }

    #[tracing::instrument(skip(self))]
    fn group_activities_by_status(
        &self,
    ) -> PaceStorageOptResult<BTreeMap<ActivityStatusKind, Vec<ActivityItem>>> {
        self.cache.group_activities_by_status()
    }
}
