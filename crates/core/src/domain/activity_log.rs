use getset::{Getters, MutGetters};
use rayon::iter::{FromParallelIterator, IntoParallelIterator};
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::domain::activity::{Activity, ActivityGuid, ActivityItem};

/// The activity log entity
///
/// The activity log entity is used to store and manage activities
#[derive(Debug, Clone, Serialize, Deserialize, Getters, MutGetters, Default, PartialEq, Eq)]
#[getset(get = "pub", get_mut = "pub")]
pub struct ActivityLog {
    /// The activities in the log
    #[serde(flatten)]
    activities: BTreeMap<ActivityGuid, Activity>,
}

impl std::ops::DerefMut for ActivityLog {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.activities
    }
}

impl std::ops::Deref for ActivityLog {
    type Target = BTreeMap<ActivityGuid, Activity>;

    fn deref(&self) -> &Self::Target {
        &self.activities
    }
}

impl FromIterator<ActivityItem> for ActivityLog {
    fn from_iter<T: IntoIterator<Item = ActivityItem>>(iter: T) -> Self {
        let iter = iter
            .into_iter()
            .map(|item| (*item.guid(), item.activity().clone()));
        let map = iter.collect::<BTreeMap<_, _>>();

        Self { activities: map }
    }
}

impl FromIterator<(ActivityGuid, Activity)> for ActivityLog {
    fn from_iter<T: IntoIterator<Item = (ActivityGuid, Activity)>>(iter: T) -> Self {
        let map = BTreeMap::from_iter(iter);

        Self { activities: map }
    }
}

impl FromParallelIterator<(ActivityGuid, Activity)> for ActivityLog {
    fn from_par_iter<T: IntoParallelIterator<Item = (ActivityGuid, Activity)>>(
        par_iter: T,
    ) -> Self {
        let map = BTreeMap::from_par_iter(par_iter);

        Self { activities: map }
    }
}

#[cfg(test)]
mod tests {

    use crate::error::TestResult;

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

    #[rstest]
    fn test_parse_activity_log_fails() {
        let toml_string = r#"
            [01HPY70577H375FDKT9XXAT7VB]
            name = "Test Activity"
            guid = "test-activity"
            start_time = "2021-01-01T00:00:00Z"
            end_time = "2021-01-01T00:00:00Z"
            duration = -5
            tags = ["test"]
        "#;

        let result = toml::from_str::<ActivityLog>(toml_string);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_parse_activity_log_empty() {
        let toml_string = r"";

        let result = toml::from_str::<ActivityLog>(toml_string);
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_parse_activity_log_single_activity() {
        let toml_string = r#"
            [01HPY70577H375FDKT9XXAT7VB]
            category = "development::pace"
            description = "Implemented the login feature."
            end = "2024-02-04T10:30:00+01:00"
            begin = "2024-02-04T09:00:00+01:00"
            duration = 5400
            kind = "activity"
        "#;

        let result = toml::from_str::<ActivityLog>(toml_string);
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_parse_activity_log_multiple_activities() {
        let toml_string = r#"
            [01HQH12254TEXQ16WCR95YZ1SN]
            category = "development::pace"
            description = "Implemented the login feature."
            end = "2024-02-04T10:30:00+00:00"
            begin = "2024-02-04T09:00:00+00:00"
            duration = 5400
            kind = "activity"

            [01HQH129DHAWRKQG6NM13NH6MH]
            category = "development::pace"
            description = "Implemented the login feature."
            end = "2024-02-04T10:30:00+00:00"
            begin = "2024-02-04T09:00:00+00:00"
            duration = 5400
            kind = "activity"
        "#;

        let result = toml::from_str::<ActivityLog>(toml_string);
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_parse_activity_log_multiple_activities_with_same_id_fails() {
        let toml_string = r#"
            [01HPY70577H375FDKT9XXAT7VB]
            category = "development::pace"
            description = "Implemented the login feature."
            end = "2024-02-04T10:30:00+00:00"
            begin = "2024-02-04T09:00:00+00:00"
            duration = 5400
            kind = "activity"

            [01HPY70577H375FDKT9XXAT7VB]
            category = "development::pace"
            description = "Implemented the login feature."
            end = "2024-02-04T10:30:00+00:00"
            begin = "2024-02-04T09:00:00+00:00"
            duration = 5400
            kind = "activity"
        "#;

        let result = toml::from_str::<ActivityLog>(toml_string);
        assert!(result.is_err());
    }
}
