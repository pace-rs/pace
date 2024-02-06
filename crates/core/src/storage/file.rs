use chrono::{Local, NaiveTime};
use std::{
    collections::BTreeMap,
    fs::{self, File},
};
use std::{fs::OpenOptions, path::PathBuf};
use std::{
    io::{Read, Write},
    path::Path,
};
use toml;
use uuid::Uuid;

use crate::{
    domain::activity::{Activity, ActivityId, ActivityLog},
    error::{PaceErrorKind, PaceResult},
    storage::ActivityStorage,
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
    fn load_all_activities(&self) -> PaceResult<ActivityLog> {
        let contents = fs::read_to_string(&self.path)?;
        let activities: ActivityLog = toml::from_str(&contents)?;
        Ok(activities)
    }

    fn list_current_activities(&self) -> PaceResult<Option<Vec<Activity>>> {
        let activities = self.load_all_activities()?;
        Ok(activities.current_activities())
    }

    fn save_activity(&self, activity: &Activity) -> PaceResult<()> {
        let mut activities = self.load_all_activities()?;
        activities.add(activity.clone())?;

        let toml = toml::to_string_pretty(&activities)?;

        // Write the new contents back to the file
        fs::write(&self.path, toml)?;
        Ok(())
    }

    fn end_all_unfinished_activities(
        &self,
        time: Option<NaiveTime>,
    ) -> PaceResult<Option<Vec<Activity>>> {
        let mut activities = self.load_all_activities()?;
        let unfinished = activities.end_all_unfinished_activities(time)?;
        let toml = toml::to_string_pretty(&activities)?;
        fs::write(&self.path, toml)?;
        Ok(unfinished)
    }

    fn end_last_unfinished_activity(
        &self,
        time: Option<NaiveTime>,
    ) -> PaceResult<Option<Activity>> {
        let mut activities = self.load_all_activities()?;
        let unfinished = activities.end_last_unfinished_activity(time)?;
        let toml = toml::to_string_pretty(&activities)?;
        fs::write(&self.path, toml)?;
        Ok(unfinished)
    }

    fn get_activities_by_id(
        &self,
        _uuid: Uuid,
    ) -> PaceResult<Option<BTreeMap<ActivityId, Activity>>> {
        todo!()
    }
}
