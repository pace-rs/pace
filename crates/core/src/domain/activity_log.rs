use getset::{Getters, MutGetters};
use serde_derive::{Deserialize, Serialize};
use std::{collections::VecDeque, iter::FromIterator};

use crate::domain::activity::Activity;

/// The activity log entity
///
/// The activity log entity is used to store and manage activities
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
}
