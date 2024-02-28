#[cfg(feature = "clap")]
use clap::Parser;

use getset::Getters;
use typed_builder::TypedBuilder;

use crate::PaceDateTime;

/// `resume` subcommand
#[derive(Debug, Getters, TypedBuilder, Clone, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "clap", derive(Parser))]
pub struct ResumeCommandOptions {
    /// Show a list of all recent activities to continue
    #[cfg_attr(feature = "clap", clap(short, long))]
    #[getset(get = "pub")]
    list: bool,
}

impl ResumeCommandOptions {
    // Inner run implementation for the resume command kept in pace-rs crate for now
    // due to the dependency on pace-cli
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
