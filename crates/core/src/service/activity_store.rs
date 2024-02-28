use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    sync::Arc,
};

use serde_derive::{Deserialize, Serialize};

use crate::{
    commands::{resume::ResumingOptions, DeletingOptions, UpdatingOptions},
    domain::{
        activity::{Activity, ActivityGuid, ActivityItem},
        filter::{ActivityStatusFilter, FilteredActivities},
    },
    error::{PaceOptResult, PaceResult},
    storage::{
        ActivityQuerying, ActivityReadOps, ActivityStateManagement, ActivityStorage,
        ActivityWriteOps, StorageKind, SyncStorage,
    },
    ActivityStatus, EndingOptions, HoldingOptions,
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
    /// Create a new `ActivityStore`
    #[must_use]
    pub fn new(storage: Arc<StorageKind>) -> Self {
        let store = Self {
            cache: ActivityStoreCache::default(),
            storage,
        };

        store
            .setup_storage()
            .expect("Should not fail to setup storage.");

        store
    }
}

impl ActivityStorage for ActivityStore {
    fn setup_storage(&self) -> PaceResult<()> {
        self.storage.setup_storage()
    }
}

impl SyncStorage for ActivityStore {
    fn sync(&self) -> PaceResult<()> {
        self.storage.sync()
    }
}

impl ActivityReadOps for ActivityStore {
    fn read_activity(&self, activity_id: ActivityGuid) -> PaceResult<ActivityItem> {
        self.storage.read_activity(activity_id)
    }

    fn list_activities(&self, filter: ActivityStatusFilter) -> PaceOptResult<FilteredActivities> {
        self.storage.list_activities(filter)
    }
}

impl ActivityWriteOps for ActivityStore {
    fn create_activity(&self, activity: Activity) -> PaceResult<ActivityItem> {
        self.storage.create_activity(activity)
    }

    fn update_activity(
        &self,
        activity_id: ActivityGuid,
        updated_activity: Activity,
        update_opts: UpdatingOptions,
    ) -> PaceResult<ActivityItem> {
        self.storage
            .update_activity(activity_id, updated_activity, update_opts)
    }

    fn delete_activity(
        &self,
        activity_id: ActivityGuid,
        delete_opts: DeletingOptions,
    ) -> PaceResult<ActivityItem> {
        self.storage.delete_activity(activity_id, delete_opts)
    }
}

impl ActivityStateManagement for ActivityStore {
    fn begin_activity(&self, activity: Activity) -> PaceResult<ActivityItem> {
        self.storage.begin_activity(activity)
    }

    fn end_activity(
        &self,
        activity_id: ActivityGuid,
        end_opts: EndingOptions,
    ) -> PaceResult<ActivityItem> {
        self.storage.end_activity(activity_id, end_opts)
    }

    fn end_all_unfinished_activities(
        &self,
        end_opts: EndingOptions,
    ) -> PaceOptResult<Vec<ActivityItem>> {
        self.storage.end_all_unfinished_activities(end_opts)
    }

    fn end_last_unfinished_activity(&self, end_opts: EndingOptions) -> PaceOptResult<ActivityItem> {
        self.storage.end_last_unfinished_activity(end_opts)
    }

    fn hold_most_recent_active_activity(
        &self,
        hold_opts: HoldingOptions,
    ) -> PaceOptResult<ActivityItem> {
        self.storage.hold_most_recent_active_activity(hold_opts)
    }

    fn end_all_active_intermissions(
        &self,
        end_opts: EndingOptions,
    ) -> PaceOptResult<Vec<ActivityGuid>> {
        self.storage.end_all_active_intermissions(end_opts)
    }

    fn resume_activity(
        &self,
        activity_id: ActivityGuid,
        resume_opts: ResumingOptions,
    ) -> PaceResult<ActivityItem> {
        self.storage.resume_activity(activity_id, resume_opts)
    }

    fn hold_activity(
        &self,
        activity_id: ActivityGuid,
        hold_opts: HoldingOptions,
    ) -> PaceResult<ActivityItem> {
        self.storage.hold_activity(activity_id, hold_opts)
    }

    fn resume_most_recent_activity(
        &self,
        resume_opts: ResumingOptions,
    ) -> PaceOptResult<ActivityItem> {
        self.storage.resume_most_recent_activity(resume_opts)
    }
}

impl ActivityQuerying for ActivityStore {
    fn list_activities_by_id(&self) -> PaceOptResult<BTreeMap<ActivityGuid, Activity>> {
        self.storage.list_activities_by_id()
    }

    fn group_activities_by_duration_range(
        &self,
    ) -> PaceOptResult<BTreeMap<crate::PaceDurationRange, Vec<ActivityItem>>> {
        self.storage.group_activities_by_duration_range()
    }

    fn group_activities_by_start_date(
        &self,
    ) -> PaceOptResult<BTreeMap<crate::PaceDate, Vec<ActivityItem>>> {
        self.storage.group_activities_by_start_date()
    }

    fn list_activities_with_intermissions(
        &self,
    ) -> PaceOptResult<BTreeMap<ActivityGuid, Vec<ActivityItem>>> {
        self.storage.list_activities_with_intermissions()
    }

    fn group_activities_by_keywords(
        &self,
        keyword_opts: crate::KeywordOptions,
    ) -> PaceOptResult<BTreeMap<String, Vec<ActivityItem>>> {
        self.storage.group_activities_by_keywords(keyword_opts)
    }

    fn group_activities_by_kind(
        &self,
    ) -> PaceOptResult<BTreeMap<crate::ActivityKind, Vec<ActivityItem>>> {
        self.storage.group_activities_by_kind()
    }

    fn list_activities_by_time_range(
        &self,
        time_range_opts: crate::TimeRangeOptions,
    ) -> PaceOptResult<Vec<ActivityItem>> {
        self.storage.list_activities_by_time_range(time_range_opts)
    }

    fn group_activities_by_status(
        &self,
    ) -> PaceOptResult<BTreeMap<ActivityStatus, Vec<ActivityItem>>> {
        self.storage.group_activities_by_status()
    }
}
