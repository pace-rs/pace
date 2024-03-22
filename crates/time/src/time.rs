use chrono::NaiveTime;
use serde_derive::{Deserialize, Serialize};

use crate::date_time::PaceDateTime;

/// Wrapper for a time of an activity
#[derive(Debug, Serialize, Deserialize, Hash, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct PaceTime(pub NaiveTime);

impl From<PaceDateTime> for PaceTime {
    fn from(date_time: PaceDateTime) -> Self {
        Self(date_time.inner().time())
    }
}

impl From<&PaceDateTime> for PaceTime {
    fn from(date_time: &PaceDateTime) -> Self {
        Self(date_time.inner().time())
    }
}

impl From<NaiveTime> for PaceTime {
    fn from(time: NaiveTime) -> Self {
        Self(time)
    }
}

impl std::ops::Deref for PaceTime {
    type Target = NaiveTime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
