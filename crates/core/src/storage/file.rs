use chrono::{Local, NaiveTime, SubsecRound};
use std::{
    collections::{BTreeMap, VecDeque},
    fs::{self, File},
};
use std::{fs::OpenOptions, path::PathBuf};
use std::{
    io::{Read, Write},
    path::Path,
};
use toml;
use uuid::Uuid;

use itertools::Itertools;

use crate::{
    domain::{
        activity::{self, Activity, ActivityId, ActivityLog},
        filter::{ActivityFilter, FilteredActivities},
    },
    error::{ActivityLogErrorKind, PaceErrorKind, PaceResult},
    storage::{ActivityReadOps, ActivityStateManagement, ActivityStorage, ActivityWriteOps},
};

pub struct TomlActivityStorage {
    path: PathBuf,
}

impl TomlActivityStorage {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
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
                .open(&self.path)?;

            file.write_all(b"")?;
        }
        Ok(())
    }
}

impl ActivityReadOps for TomlActivityStorage {
    fn read_activity(&self, activity_id: ActivityId) -> PaceResult<Option<Activity>> {
        self.list_activities(ActivityFilter::default())?
            .map(|filtered| {
                filtered
                    .into_log()
                    .activities()
                    .iter()
                    .find(|activity| {
                        if let Some(id) = activity.id() {
                            *id == activity_id
                        } else {
                            false
                        }
                    })
                    .cloned()
            })
            .ok_or(ActivityLogErrorKind::FailedToReadActivity(activity_id).into())
    }

    fn list_activities(&self, filter: ActivityFilter) -> PaceResult<Option<FilteredActivities>> {
        let contents = fs::read_to_string(&self.path)?;
        let activity_log: ActivityLog = toml::from_str(&contents)?;

        let filtered = activity_log
            .activities()
            .iter()
            .filter(|activity| match filter {
                ActivityFilter::Active => {
                    activity.end_date().is_none() || activity.end_time().is_none()
                }
                ActivityFilter::Ended => {
                    activity.end_date().is_some() && activity.end_time().is_some()
                }
                ActivityFilter::All => true,
                ActivityFilter::Archived => false, // TODO: Implement archived filter
            })
            .cloned()
            .collect::<ActivityLog>();

        if filtered.activities().is_empty() {
            return Ok(None);
        }

        match filter {
            ActivityFilter::Active => Ok(Some(FilteredActivities::Active(filtered))),
            ActivityFilter::Ended => Ok(Some(FilteredActivities::Ended(filtered))),
            ActivityFilter::All => Ok(Some(FilteredActivities::All(filtered))),
            ActivityFilter::Archived => Ok(Some(FilteredActivities::Archived(filtered))),
        }
    }
}

impl ActivityStateManagement for TomlActivityStorage {
    fn end_all_unfinished_activities(
        &self,
        time: Option<NaiveTime>,
    ) -> PaceResult<Option<Vec<Activity>>> {
        // TODO: Make date formats configurable
        let date = Local::now().date_naive();
        let time = time.unwrap_or_else(|| Local::now().time().round_subsecs(0));

        let mut unfinished_activities: Vec<Activity> = vec![];

        let activities = self.list_activities(ActivityFilter::All)?.map(|filtered| {
            filtered
                .into_log()
                .activities_mut()
                .iter_mut()
                .map(|activity| {
                    if activity.end_date().is_none() && activity.end_time().is_none() {
                        activity.end_date_mut().replace(date);
                        activity.end_time_mut().replace(time);
                        unfinished_activities.push(activity.clone());
                    }

                    activity.clone()
                })
                .collect::<ActivityLog>()
        });

        // Return early with Ok(None) if there are no activities to end
        if unfinished_activities.is_empty() {
            Ok(None)
        } else {
            // Sort the activities by start date
            unfinished_activities.sort_by(|a, b| a.start_date().cmp(b.start_date()));

            // Write the updated (all activities) content back to the file
            let toml = toml::to_string_pretty(&activities)?;
            fs::write(&self.path, toml)?;

            // Return the activities that were ended
            Ok(Some(unfinished_activities))
        }
    }

    fn end_last_unfinished_activity(
        &self,
        time: Option<NaiveTime>,
    ) -> PaceResult<Option<Activity>> {
        let mut activity_log = self
            .list_activities(ActivityFilter::Active)?
            .ok_or(ActivityLogErrorKind::NoActivityToEnd)?
            .into_log();

        let activity: Activity;

        // Return early with Ok(None) if there are no activities to end
        if activity_log.activities().is_empty() {
            return Ok(None);
        }

        // Scope for mutable borrow of last_activity
        {
            let Some(last_activity) = activity_log.activities_mut().front_mut() else {
                return Err(ActivityLogErrorKind::NoActivityToEnd.into());
            };

            // TODO: Make date formats configurable
            let date = Local::now().date_naive();
            let time = time.unwrap_or_else(|| Local::now().time().round_subsecs(0));

            // If the last activity already has an end date and time, return early with Ok(None)
            if last_activity.end_date().is_some() && last_activity.end_time().is_some() {
                return Ok(None);
            }

            last_activity.end_date_mut().replace(date);
            last_activity.end_time_mut().replace(time);

            // Clone the last activity to return it after the mutable borrow ends
            activity = last_activity.clone();
        }

        let toml = toml::to_string_pretty(&activity_log.clone())?;
        fs::write(&self.path, toml)?;

        Ok(Some(activity))
    }

    fn start_activity(&self, _activity: &Activity) -> PaceResult<ActivityId> {
        todo!()
    }

    fn end_activity(
        &self,
        _activity_id: ActivityId,
        _end_time: Option<NaiveTime>,
    ) -> PaceResult<ActivityId> {
        todo!()
    }
}

impl ActivityWriteOps for TomlActivityStorage {
    fn create_activity(&self, activity: &Activity) -> PaceResult<ActivityId> {
        let mut activity = activity.clone();

        let mut activity_log = self
            .list_activities(ActivityFilter::default())?
            .ok_or(ActivityLogErrorKind::NoActivitiesFound)?
            .into_log();

        // Generate an ID for the activity if it doesn't have one
        _ = activity.id_mut().get_or_insert_with(ActivityId::default);

        let activity_id = activity.id().clone().unwrap();

        activity_log.activities_mut().push_front(activity);

        let toml = toml::to_string_pretty(&activity_log)?;

        // Write the new contents back to the file
        fs::write(&self.path, toml)?;

        // Return the ID of the newly created activity
        Ok(activity_id)
    }

    fn update_activity(&self, _activity_id: ActivityId, _activity: &Activity) -> PaceResult<()> {
        todo!()
    }

    fn delete_activity(&self, _activity_id: ActivityId) -> PaceResult<()> {
        todo!()
    }
}
