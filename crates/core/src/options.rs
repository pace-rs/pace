use getset::{Getters, MutGetters, Setters};
use pace_time::date_time::PaceDateTime;
use serde_derive::Serialize;
use typed_builder::TypedBuilder;

use crate::domain::{
    category::PaceCategory, description::PaceDescription, intermission::IntermissionAction,
};

/// Options for holding an activity
#[derive(Debug, Clone, PartialEq, TypedBuilder, Eq, Hash, Default, Getters)]
#[getset(get = "pub")]
#[non_exhaustive]
pub struct HoldOptions {
    /// The action to take on the intermission
    #[builder(default)]
    action: IntermissionAction,

    /// The start time of the intermission
    #[builder(default, setter(into))]
    begin_time: PaceDateTime,

    /// The reason for holding the activity
    #[builder(default, setter(into))]
    reason: Option<PaceDescription>,
}

/// Options for ending an activity
#[derive(Debug, Clone, PartialEq, TypedBuilder, Eq, Hash, Default, Getters)]
#[getset(get = "pub")]
#[non_exhaustive]
pub struct EndOptions {
    /// The end time
    #[builder(default, setter(into))]
    end_time: PaceDateTime,
}

impl From<HoldOptions> for EndOptions {
    fn from(hold_opts: HoldOptions) -> Self {
        Self {
            end_time: *hold_opts.begin_time(),
        }
    }
}

impl From<ResumeOptions> for EndOptions {
    fn from(resume_opts: ResumeOptions) -> Self {
        Self {
            end_time: resume_opts.resume_time().unwrap_or_else(PaceDateTime::now),
        }
    }
}

/// Options for updating an activity
#[derive(Debug, Clone, PartialEq, TypedBuilder, Eq, Hash, Default, Getters)]
#[getset(get = "pub")]
#[non_exhaustive]
pub struct UpdateOptions {}

/// Options for deleting an activity
#[derive(Debug, Clone, PartialEq, TypedBuilder, Eq, Hash, Default, Getters)]
#[getset(get = "pub")]
#[non_exhaustive]
pub struct DeleteOptions {}

#[derive(Debug, Clone, PartialEq, TypedBuilder, Eq, Hash, Default, Getters)]
#[getset(get = "pub")]
#[non_exhaustive]
pub struct KeywordOptions {
    #[builder(default, setter(into, strip_option))]
    category: Option<PaceCategory>,
}

/// Options for resuming an activity
#[derive(Debug, Clone, PartialEq, TypedBuilder, Eq, Hash, Default, Getters)]
#[getset(get = "pub")]
#[non_exhaustive]
pub struct ResumeOptions {
    /// The resume time of the intermission
    #[builder(default, setter(into))]
    resume_time: Option<PaceDateTime>,
}

#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
#[getset(get = "pub")]
pub struct FilterOptions {
    category: Option<PaceCategory>,
    case_sensitive: bool,
}
