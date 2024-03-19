pub mod adjust;
pub mod begin;
pub mod docs;
pub mod end;
pub mod hold;
pub mod now;
pub mod reflect;
pub mod resume;
pub mod settings;

use getset::Getters;
use typed_builder::TypedBuilder;

use crate::{
    commands::{hold::HoldOptions, resume::ResumeOptions},
    domain::time::PaceNaiveDateTime,
};

/// Options for ending an activity
#[derive(Debug, Clone, PartialEq, TypedBuilder, Eq, Hash, Default, Getters)]
#[getset(get = "pub")]
#[non_exhaustive]
pub struct EndOptions {
    /// The end time
    #[builder(default, setter(into))]
    end_time: PaceNaiveDateTime,
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
            end_time: resume_opts
                .resume_time()
                .unwrap_or_else(PaceNaiveDateTime::now),
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
    category: Option<String>,
}
