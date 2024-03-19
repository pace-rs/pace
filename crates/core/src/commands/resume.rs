#[cfg(feature = "clap")]
use clap::Parser;

use getset::Getters;
use typed_builder::TypedBuilder;

use crate::PaceNaiveDateTime;

/// `resume` subcommand options
#[derive(Debug, Getters, TypedBuilder, Clone, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "clap", derive(Parser))]
#[getset(get = "pub")]
pub struct ResumeCommandOptions {
    /// The time the activity has been resumed at. Format: HH:MM
    #[cfg_attr(
        feature = "clap",
        clap(short, long, name = "Resumed Time", alias = "resumed")
    )]
    at: Option<String>,

    /// Show a list of all recent activities to continue
    #[cfg_attr(feature = "clap", clap(short, long))]
    #[getset(get = "pub")]
    list: bool,
}

impl ResumeCommandOptions {
    // FIXME: Inner run implementation for the resume command kept in pace-rs crate for now
    // FIXME: due to the dependency on pace-cli
}

/// Options for resuming an activity
#[derive(Debug, Clone, PartialEq, TypedBuilder, Eq, Hash, Default, Getters)]
#[getset(get = "pub")]
#[non_exhaustive]
pub struct ResumeOptions {
    /// The resume time of the intermission
    #[builder(default, setter(into))]
    resume_time: Option<PaceNaiveDateTime>,
}
