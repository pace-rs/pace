use getset::Getters;
use typed_builder::TypedBuilder;

use crate::PaceDateTime;

/// Options for resuming an activity
#[derive(Debug, Clone, PartialEq, TypedBuilder, Eq, Hash, Default, Getters)]
#[getset(get = "pub")]
#[non_exhaustive]
pub struct ResumeOptions {
    /// The resume time of the intermission
    #[builder(default, setter(into))]
    resume_time: Option<PaceDateTime>,
}
