//! Activity entity and business logic

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, SubsecRound, TimeZone};
use derive_getters::Getters;
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    fmt::Display,
    fs,
    path::Path,
};
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::{
    domain::{
        category::Category,
        intermission::{self, IntermissionPeriod},
        status::ItemStatus,
        tag::Tag,
        task::TaskList,
        time::duration_to_str,
    },
    error::{ActivityLogErrorKind, PaceErrorKind, PaceResult},
    storage::ActivityStorage,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ActivityKind {
    #[default]
    Activity,
    Task,
    Intermission,
    PomodoroWork,
    PomodoroIntermission,
}

// Optional: Track Pomodoro work/break cycles
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
enum PomodoroCycle {
    Work(usize), // usize could represent the work session number in a sequence
    #[default]
    Intermission,
}

#[derive(Debug, Serialize, Deserialize, TypedBuilder, Getters, Clone)]
pub struct Activity {
    #[builder(default = Some(ActivityId::default()), setter(strip_option))]
    id: Option<ActivityId>,

    // TODO: We had it as a struct before with an ID, but it's questionable if we should go for this
    // TODO: Reconsider when we implement the project management part
    // category: Category,
    category: Option<String>,

    #[builder(default, setter(strip_option))]
    description: Option<String>,

    #[builder(default, setter(strip_option))]
    end_date: Option<NaiveDate>,

    #[builder(default, setter(strip_option))]
    end_time: Option<NaiveTime>,

    start_date: NaiveDate,

    start_time: NaiveTime,

    kind: ActivityKind,

    // TODO: How to better support subcategories
    // subcategory: Option<Category>,

    // TODO: Was `Tag` before, but we want to check how to better support that
    // TODO: also, we should consider using a HashSet instead of a Vec
    // TODO: also, we might want to reconsider
    // #[builder(default, setter(strip_option))]
    // tags: Option<Vec<String>>,

    // Pomodoro-specific attributes
    #[builder(default, setter(strip_option))]
    pomodoro_cycle: Option<PomodoroCycle>,

    // Intermission-specific attributes
    #[builder(default, setter(strip_option))]
    intermission_periods: Option<Vec<IntermissionPeriod>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialEq, PartialOrd, Eq)]
pub struct ActivityId(Uuid);

impl Default for ActivityId {
    fn default() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Display for Activity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // FIXME: Refactor to use non-deprecated methods
        let time = DateTime::from_local(
            NaiveDateTime::new(self.start_date, self.start_time),
            *Local::now().offset(),
        );

        let rel_time = duration_to_str(time);

        write!(
            f,
            "Activity: \"{}\" started {}",
            self.description.as_deref().unwrap_or("No description"),
            rel_time,
        )
    }
}

impl rusqlite::types::FromSql for ActivityId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let bytes = <[u8; 16]>::column_result(value)?;
        Ok(ActivityId(uuid::Uuid::from_u128(u128::from_be_bytes(
            bytes,
        ))))
    }
}

impl rusqlite::types::ToSql for ActivityId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.as_ref().to_sql()
    }
}

impl Activity {
    pub fn start_intermission(&mut self, date: NaiveDate, time: NaiveTime) {
        let new_intermission = IntermissionPeriod::new(date, time);
        if let Some(ref mut periods) = self.intermission_periods {
            periods.push(new_intermission);
        } else {
            self.intermission_periods = Some(vec![new_intermission]);
        }
    }

    pub fn end_intermission(&mut self, date: NaiveDate, time: NaiveTime) {
        if let Some(intermission_periods) = &mut self.intermission_periods {
            if let Some(last_period) = intermission_periods.last_mut() {
                // Assuming intermissions can't overlap, the last one is the one to end
                last_period.end(date, time);
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Getters)]
pub struct ActivityLog {
    activities: VecDeque<Activity>,
}

impl ActivityLog {
    pub fn load(activity_path: impl AsRef<Path>) -> PaceResult<Self> {
        let toml_string = fs::read_to_string(activity_path)?;
        Ok(toml::from_str::<Self>(&toml_string)?)
    }

    pub fn add(&mut self, activity: Activity) -> PaceResult<()> {
        self.activities.push_front(activity);
        Ok(())
    }

    pub fn current_activities(&self) -> Option<Vec<Activity>> {
        let current_activities = self
            .activities
            .iter()
            .filter(|activity| activity.end_date.is_none() || activity.end_time.is_none())
            .cloned()
            .collect::<Vec<Activity>>();

        if current_activities.is_empty() {
            return None;
        }

        Some(current_activities)
    }

    pub fn end_all_unfinished_activities(
        &mut self,
        time: Option<NaiveTime>,
    ) -> PaceResult<Option<Vec<Activity>>> {
        // TODO: Make date formats configurable
        let date = Local::now().date_naive();
        let time = time.unwrap_or_else(|| Local::now().time().round_subsecs(0));

        let unfinished_activities = self
            .activities
            .iter_mut()
            .filter(|activity| activity.end_date.is_none() || activity.end_time.is_none())
            .map(|activity| {
                activity.end_date = Some(date);
                activity.end_time = Some(time);
                activity.clone()
            })
            .collect::<Vec<Activity>>();

        if unfinished_activities.is_empty() {
            return Ok(None);
        }

        Ok(Some(unfinished_activities))
    }

    pub fn end_last_unfinished_activity(
        &mut self,
        time: Option<NaiveTime>,
    ) -> PaceResult<Option<Activity>> {
        let Some(last_activity) = self.activities.front_mut() else {
            return Err(ActivityLogErrorKind::NoActivityToEnd.into());
        };

        // TODO: Make date formats configurable
        let date = Local::now().date_naive();
        let time = time.unwrap_or_else(|| Local::now().time().round_subsecs(0));

        if last_activity.end_date.is_some() && last_activity.end_time.is_some() {
            return Ok(None);
        }

        last_activity.end_date = Some(date);
        last_activity.end_time = Some(time);

        Ok(Some(last_activity.clone()))
    }

    // pub fn activities_by_id(&self) -> PaceResult<BTreeMap<ActivityId, Activity>> {
    //     let activities_by_id = self
    //         .activities
    //         .into_iter()
    //         .map(|activity| (activity.id, activity))
    //         .collect::<BTreeMap<ActivityId, Activity>>();
    // }
}

#[cfg(test)]
mod tests {

    use crate::{domain::project::ProjectConfig, domain::task::TaskList, error::TestResult};

    use super::*;
    use rstest::*;
    use std::{fs, path::PathBuf};

    #[rstest]
    fn test_parse_activity_log_passes(
        #[files("../../data/*.toml")] activity_path: PathBuf,
    ) -> TestResult<()> {
        let toml_string = fs::read_to_string(activity_path)?;
        let _ = toml::from_str::<ActivityLog>(&toml_string)?;

        Ok(())
    }
}
