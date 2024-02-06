use std::collections::BTreeMap;

use chrono::NaiveTime;
use serde::{Deserialize, Serialize};

use crate::{
    domain::activity::{Activity, ActivityId, ActivityLog},
    error::{PaceErrorKind, PaceResult},
};

pub mod file;
// TODO: Implement in-memory Storage
// pub mod in_memory;
// TODO: Implement conversion FromSQL and ToSQL
// pub mod sqlite;

pub trait ActivityStorage {
    fn setup_storage(&self) -> PaceResult<()>;

    fn load_all_activities(&self) -> PaceResult<ActivityLog>;

    fn list_current_activities(&self) -> PaceResult<Option<Vec<Activity>>>;

    fn save_activity(&self, activity: &Activity) -> PaceResult<()>;

    fn end_all_unfinished_activities(
        &self,
        time: Option<NaiveTime>,
    ) -> PaceResult<Option<Vec<Activity>>>;

    fn end_last_unfinished_activity(&self, time: Option<NaiveTime>)
        -> PaceResult<Option<Activity>>;

    fn get_activities_by_id(
        &self,
        uuid: uuid::Uuid,
    ) -> PaceResult<Option<BTreeMap<ActivityId, Activity>>>;
}
