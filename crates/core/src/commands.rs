pub mod hold;
pub mod resume;

use getset::Getters;
use typed_builder::TypedBuilder;

use crate::{HoldOptions, PaceDateTime};

/// Options for ending an activity
#[derive(Debug, Clone, PartialEq, TypedBuilder, Eq, Hash, Default, Getters)]
#[getset(get = "pub")]
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
