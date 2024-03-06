use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    sync::Arc,
};

use serde_derive::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    commands::{resume::ResumeOptions, DeleteOptions, UpdateOptions},
    domain::{
        activity::{Activity, ActivityGuid, ActivityItem},
        filter::{ActivityStatusFilter, FilteredActivities},
    },
    error::{PaceOptResult, PaceResult},
    storage::{
        ActivityQuerying, ActivityReadOps, ActivityStateManagement, ActivityStorage,
        ActivityWriteOps, StorageKind, SyncStorage,
    },
    ActivityStatus, EndOptions, HoldOptions,
};

/// The activity store entity
pub struct ActivityStore {
    /// In-memory cache for activities
    cache: ActivityStoreCache,

    /// The storage backend
    storage: Arc<StorageKind>,
}

/// TODO: Optimization for later to make lookup faster
#[derive(Serialize, Deserialize, Debug, Default)]
struct ActivityStoreCache {
    activity_ids: HashSet<ActivityGuid>,
    activities_by_id: BTreeMap<ActivityGuid, Activity>,
    last_entries: VecDeque<ActivityGuid>,
}

impl ActivityStore {
    /// Create a new `ActivityStore` with a given storage backend
    pub fn with_storage(storage: Arc<StorageKind>) -> PaceResult<Self> {
        debug!("Creating activity store with storage: {}", storage);

        let store = Self {
            cache: ActivityStoreCache::default(),
            storage,
        };

        store.setup_storage()?;

        Ok(store)
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
    fn list_activities(&self, filter: ActivityStatusFilter) -> PaceOptResult<FilteredActivities> {
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
    ) -> PaceOptResult<BTreeMap<crate::PaceDate, Vec<ActivityItem>>> {
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
        time_range_opts: crate::TimeRangeOptions,
    ) -> PaceOptResult<Vec<ActivityItem>> {
        self.storage.list_activities_by_time_range(time_range_opts)
    }

    #[tracing::instrument(skip(self))]
    fn group_activities_by_status(
        &self,
    ) -> PaceOptResult<BTreeMap<ActivityStatus, Vec<ActivityItem>>> {
        self.storage.group_activities_by_status()
    }
}
