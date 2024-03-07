use std::{collections::BTreeMap, sync::Arc};

use getset::{Getters, MutGetters, Setters};
use tracing::debug;
use typed_builder::TypedBuilder;

use crate::{
    commands::{resume::ResumeOptions, DeleteOptions, UpdateOptions},
    domain::{
        activity::{Activity, ActivityGuid, ActivityItem},
        filter::{ActivityFilterKind, FilteredActivities},
        review::SummaryGroupByCategory,
    },
    error::{ActivityStoreErrorKind, PaceOptResult, PaceResult},
    storage::{
        ActivityQuerying, ActivityReadOps, ActivityStateManagement, ActivityStorage,
        ActivityWriteOps, StorageKind, SyncStorage,
    },
    ActivityGroup, ActivityStatus, EndOptions, HoldOptions, PaceDate, SummaryGroup,
    TimeRangeOptions,
};

/// The activity store entity
#[derive(TypedBuilder, Getters, Setters, MutGetters)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct ActivityStore {
    /// In-memory cache for activities
    cache: ActivityStoreCache,

    /// The storage backend
    storage: Arc<StorageKind>,
}

#[derive(Debug, TypedBuilder, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct ActivityStoreCache {
    by_start_date: BTreeMap<PaceDate, Vec<ActivityItem>>,
}

impl ActivityStore {
    /// Create a new `ActivityStore` with a given storage backend
    ///
    /// # Arguments
    ///
    /// * `storage` - The storage backend to use for the activity store
    ///
    /// # Errors
    ///
    /// This method will return an error if the storage backend cannot be used
    ///
    /// # Returns
    ///
    /// This method returns a new `ActivityStore` if the storage backend
    /// was successfully created
    pub fn with_storage(storage: Arc<StorageKind>) -> PaceResult<Self> {
        debug!("Creating activity store with storage: {}", storage);

        let mut store = Self {
            cache: ActivityStoreCache::default(),
            storage,
        };

        store.setup_storage()?;

        store.populate_caches()?;

        Ok(store)
    }

    /// Populate the in-memory cache with activities from the storage backend
    ///
    /// This method is called during the initialization of the activity store
    ///
    /// # Errors
    ///
    /// This method will return an error if the cache cannot be populated
    ///
    /// # Returns
    ///
    /// This method returns `Ok(())` if the cache was successfully populated
    fn populate_caches(&mut self) -> PaceResult<()> {
        self.cache.by_start_date = self
            .storage
            .group_activities_by_start_date()?
            .ok_or(ActivityStoreErrorKind::PopulatingCache)?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn summary_groups_by_category_for_time_range(
        &self,
        time_range_opts: TimeRangeOptions,
    ) -> PaceOptResult<SummaryGroupByCategory> {
        let Some(activity_guids) = self.list_activities_by_time_range(time_range_opts)? else {
            debug!("No activities found for time range: {:?}", time_range_opts);

            return Ok(None);
        };

        // merge the activities into summary groups
        let mut summary_groups: SummaryGroupByCategory = BTreeMap::new();

        for activity_guid in activity_guids {
            let activity_item = self.read_activity(activity_guid)?;

            let mut activity_group = ActivityGroup::new(activity_item.clone());

            if let Some(intermissions) =
                self.list_intermissions_for_activity_id(*activity_item.guid())?
            {
                activity_group.add_multiple_intermissions(intermissions);
            };

            _ = summary_groups
                .entry(
                    activity_item
                        .activity()
                        .category()
                        .clone()
                        .unwrap_or_else(|| "Uncategorized".to_string()),
                )
                .and_modify(|e| e.add_activity_group(activity_group.clone()))
                .or_insert_with(|| SummaryGroup::with_activity_group(activity_group));
        }

        Ok(Some(summary_groups))
    }
}

impl ActivityStorage for ActivityStore {
    #[tracing::instrument(skip(self))]
    fn setup_storage(&self) -> PaceResult<()> {
        self.storage.setup_storage()
    }
}

impl SyncStorage for ActivityStore {
    #[tracing::instrument(skip(self))]
    fn sync(&self) -> PaceResult<()> {
        self.storage.sync()
    }
}

impl ActivityReadOps for ActivityStore {
    #[tracing::instrument(skip(self))]
    fn read_activity(&self, activity_id: ActivityGuid) -> PaceResult<ActivityItem> {
        self.storage.read_activity(activity_id)
    }

    #[tracing::instrument(skip(self))]
    fn list_activities(&self, filter: ActivityFilterKind) -> PaceOptResult<FilteredActivities> {
        self.storage.list_activities(filter)
    }
}

impl ActivityWriteOps for ActivityStore {
    #[tracing::instrument(skip(self))]
    fn create_activity(&self, activity: Activity) -> PaceResult<ActivityItem> {
        self.storage.create_activity(activity)
    }

    #[tracing::instrument(skip(self))]
    fn update_activity(
        &self,
        activity_id: ActivityGuid,
        updated_activity: Activity,
        update_opts: UpdateOptions,
    ) -> PaceResult<ActivityItem> {
        self.storage
            .update_activity(activity_id, updated_activity, update_opts)
    }

    #[tracing::instrument(skip(self))]
    fn delete_activity(
        &self,
        activity_id: ActivityGuid,
        delete_opts: DeleteOptions,
    ) -> PaceResult<ActivityItem> {
        self.storage.delete_activity(activity_id, delete_opts)
    }
}

impl ActivityStateManagement for ActivityStore {
    #[tracing::instrument(skip(self))]
    fn begin_activity(&self, activity: Activity) -> PaceResult<ActivityItem> {
        self.storage.begin_activity(activity)
    }

    #[tracing::instrument(skip(self))]
    fn end_activity(
        &self,
        activity_id: ActivityGuid,
        end_opts: EndOptions,
    ) -> PaceResult<ActivityItem> {
        self.storage.end_activity(activity_id, end_opts)
    }

    #[tracing::instrument(skip(self))]
    fn end_all_activities(&self, end_opts: EndOptions) -> PaceOptResult<Vec<ActivityItem>> {
        self.storage.end_all_activities(end_opts)
    }

    #[tracing::instrument(skip(self))]
    fn end_last_unfinished_activity(&self, end_opts: EndOptions) -> PaceOptResult<ActivityItem> {
        self.storage.end_last_unfinished_activity(end_opts)
    }

    #[tracing::instrument(skip(self))]
    fn hold_most_recent_active_activity(
        &self,
        hold_opts: HoldOptions,
    ) -> PaceOptResult<ActivityItem> {
        self.storage.hold_most_recent_active_activity(hold_opts)
    }

    #[tracing::instrument(skip(self))]
    fn end_all_active_intermissions(
        &self,
        end_opts: EndOptions,
    ) -> PaceOptResult<Vec<ActivityGuid>> {
        self.storage.end_all_active_intermissions(end_opts)
    }

    #[tracing::instrument(skip(self))]
    fn resume_activity(
        &self,
        activity_id: ActivityGuid,
        resume_opts: ResumeOptions,
    ) -> PaceResult<ActivityItem> {
        self.storage.resume_activity(activity_id, resume_opts)
    }

    #[tracing::instrument(skip(self))]
    fn hold_activity(
        &self,
        activity_id: ActivityGuid,
        hold_opts: HoldOptions,
    ) -> PaceResult<ActivityItem> {
        self.storage.hold_activity(activity_id, hold_opts)
    }

    #[tracing::instrument(skip(self))]
    fn resume_most_recent_activity(
        &self,
        resume_opts: ResumeOptions,
    ) -> PaceOptResult<ActivityItem> {
        self.storage.resume_most_recent_activity(resume_opts)
    }
}

impl ActivityQuerying for ActivityStore {
    #[tracing::instrument(skip(self))]
    fn list_activities_by_id(&self) -> PaceOptResult<BTreeMap<ActivityGuid, Activity>> {
        self.storage.list_activities_by_id()
    }

    #[tracing::instrument(skip(self))]
    fn group_activities_by_duration_range(
        &self,
    ) -> PaceOptResult<BTreeMap<crate::PaceDurationRange, Vec<ActivityItem>>> {
        self.storage.group_activities_by_duration_range()
    }

    #[tracing::instrument(skip(self))]
    fn group_activities_by_start_date(
        &self,
    ) -> PaceOptResult<BTreeMap<PaceDate, Vec<ActivityItem>>> {
        self.storage.group_activities_by_start_date()
    }

    #[tracing::instrument(skip(self))]
    fn list_activities_with_intermissions(
        &self,
    ) -> PaceOptResult<BTreeMap<ActivityGuid, Vec<ActivityItem>>> {
        self.storage.list_activities_with_intermissions()
    }

    #[tracing::instrument(skip(self))]
    fn group_activities_by_keywords(
        &self,
        keyword_opts: crate::KeywordOptions,
    ) -> PaceOptResult<BTreeMap<String, Vec<ActivityItem>>> {
        self.storage.group_activities_by_keywords(keyword_opts)
    }

    #[tracing::instrument(skip(self))]
    fn group_activities_by_kind(
        &self,
    ) -> PaceOptResult<BTreeMap<crate::ActivityKind, Vec<ActivityItem>>> {
        self.storage.group_activities_by_kind()
    }

    #[tracing::instrument(skip(self))]
    fn list_activities_by_time_range(
        &self,
        time_range_opts: TimeRangeOptions,
    ) -> PaceOptResult<Vec<ActivityGuid>> {
        self.storage.list_activities_by_time_range(time_range_opts)
    }

    #[tracing::instrument(skip(self))]
    fn group_activities_by_status(
        &self,
    ) -> PaceOptResult<BTreeMap<ActivityStatus, Vec<ActivityItem>>> {
        self.storage.group_activities_by_status()
    }
}
