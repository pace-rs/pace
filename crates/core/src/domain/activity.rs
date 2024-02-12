//! Activity entity and business logic

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, SubsecRound, TimeZone};
use getset::{CopyGetters, Getters, MutGetters, Setters};
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    fmt::{format, Display},
    fs,
    path::Path,
};
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::{
    domain::{
        category::Category,
        filter::ActivityFilter,
        intermission::{self, IntermissionPeriod},
        status::ItemStatus,
        tag::Tag,
        task::TaskList,
        time::duration_to_str,
    },
    error::{ActivityLogErrorKind, PaceErrorKind, PaceResult},
    storage::ActivityStorage,
};

pub type ActivityDequeCollection = VecDeque<Activity>;

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

#[derive(Debug, Serialize, Deserialize, TypedBuilder, Getters, MutGetters, Clone)]
pub struct Activity {
    #[builder(default = Some(ActivityId::default()), setter(strip_option))]
    #[getset(get = "pub", get_mut = "pub")]
    id: Option<ActivityId>,

    // TODO: We had it as a struct before with an ID, but it's questionable if we should go for this
    // TODO: Reconsider when we implement the project management part
    // category: Category,
    category: Option<String>,

    #[builder(default, setter(strip_option))]
    description: Option<String>,

    #[builder(default, setter(strip_option))]
    #[getset(get = "pub", get_mut = "pub")]
    end_date: Option<NaiveDate>,

    #[builder(default, setter(strip_option))]
    #[getset(get = "pub", get_mut = "pub")]
    end_time: Option<NaiveTime>,

    #[getset(get = "pub")]
    start_date: NaiveDate,

    #[getset(get = "pub")]
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
        let rel_time =
            match NaiveDateTime::new(self.start_date, self.start_time).and_local_timezone(Local) {
                chrono::LocalResult::Single(time) => duration_to_str(time),
                chrono::LocalResult::None | chrono::LocalResult::Ambiguous(_, _) => {
                    format!("at {} {}", self.start_date, self.start_time)
                }
            };

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

    pub fn archived(&self) -> bool {
        self.end_date.is_some() && self.end_time.is_some()
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Getters)]
pub struct ActivityLog {
    activities: ActivityDequeCollection,
}

impl From<ActivityDequeCollection> for ActivityLog {
    fn from(activities: ActivityDequeCollection) -> Self {
        Self { activities }
    }
}

impl ActivityLog {
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
        let _ = toml::from_str::<Vec<Activity>>(&toml_string)?;

        Ok(())
    }
}
