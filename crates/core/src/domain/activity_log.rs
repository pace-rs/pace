use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, SubsecRound, TimeZone};
use getset::{CopyGetters, Getters, MutGetters, Setters};
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    fmt::{format, Display},
    fs,
    iter::FromIterator,
    path::Path,
    time::Duration,
};
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::{
    domain::{
        activity::Activity,
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

#[derive(Debug, Clone, Serialize, Deserialize, Getters, MutGetters)]
pub struct ActivityLog {
    #[getset(get = "pub", get_mut = "pub")]
    activities: VecDeque<Activity>,
}

impl Default for ActivityLog {
    fn default() -> Self {
        Self {
            activities: VecDeque::from(vec![Activity::default()]),
        }
    }
}

impl FromIterator<Activity> for ActivityLog {
    fn from_iter<T: IntoIterator<Item = Activity>>(iter: T) -> Self {
        Self {
            activities: iter.into_iter().collect::<VecDeque<Activity>>(),
        }
    }
}

impl ActivityLog {
    pub fn current_activities(&self) -> Option<Vec<Activity>> {
        let current_activities = self
            .activities
            .iter()
            .filter(|activity| activity.is_active())
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
        let _ = toml::from_str::<ActivityLog>(&toml_string)?;

        Ok(())
    }
}
